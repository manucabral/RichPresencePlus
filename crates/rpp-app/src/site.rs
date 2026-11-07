//! A presence for a site, made by clicking on the page.

use std::path::{Path, PathBuf};

use rpp_bridge::SiteRequest;
use rpp_plugins::Manifest;
use serde_json::{Value, json};

use crate::program::SHARED_CLIENT_ID;
use crate::templates;

/// Longest host name.
const MAX_HOST: usize = 253;

/// Tells whether a text is a plain host name: no pattern can hide in it.
fn valid_host(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= MAX_HOST
        && host.contains('.')
        && host.split('.').all(|label| {
            !label.is_empty()
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        })
}

/// Writes the script. The site reaches it as JSON, so it stays a text.
fn script(name: &str, host: &str) -> Result<String, String> {
    let site = serde_json::to_string(host).map_err(|e| e.to_string())?;
    let shown = serde_json::to_string(name).map_err(|e| e.to_string())?;
    Ok(format!(
        r#"// Made by Rich Presence Plus. You can edit this file.
// The texts it reads are chosen in manifest.json, under "page".

const NAME = {shown};
const SITE = {site};

function read(context) {{
  const page = bestPage(context.pages);
  const first = page && cut(page.values.first);
  if (!first || first.length < 2) return null;
  return {{ first, second: cut(page.values.second) }};
}}

function activity(seen) {{
  const second = seen.second && seen.second.length >= 2 ? seen.second : SITE;
  return {{ name: NAME, details: seen.first, state: second, kind: "watching" }};
}}
"#
    ))
}

/// Writes the presence folder for what the extension sent and returns it.
///
/// Raises:
///     String: The site or a selector cannot be used, the name is taken, or the
///         files cannot be written.
pub(crate) fn create(
    root: &Path,
    request: &SiteRequest,
    author: Option<&str>,
) -> Result<PathBuf, String> {
    let host = request.host.trim().to_lowercase();
    if !valid_host(&host) {
        return Err("That site cannot be read.".into());
    }
    let (name, folder) = templates::checked(&request.name)?;
    let mut extract = serde_json::Map::new();
    let text = |selector: &str| -> Value { json!({ "from": "text", "selector": selector }) };
    extract.insert("first".to_owned(), text(&request.first));
    if let Some(second) = request
        .second
        .as_deref()
        .filter(|second| !second.is_empty())
    {
        extract.insert("second".to_owned(), text(second));
    }
    let manifest = json!({
        "name": name,
        "version": "1.0.0",
        "description": format!("Shows what you read on {host}."),
        "client_id": SHARED_CLIENT_ID,
        "kind": "other",
        "interval": 5,
        "entry": "main.js",
        "domains": [host],
        "page": { "matches": [format!("*://{host}/*")], "extract": extract },
        "author": author,
    });
    let manifest = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())? + "\n";
    Manifest::parse(&manifest).map_err(|e| e.to_string())?;
    let source = script(name, &host)?;
    templates::publish(root, &folder, |staging| {
        templates::write(
            staging,
            &[("manifest.json", &manifest), ("main.js", &source)],
        )
        .map_err(|e| format!("Cannot write the presence: {e}"))
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::rc::Rc;

    use rpp_plugins::{Host, Plugin, Request};

    use super::*;

    #[derive(Debug)]
    struct NoHost;

    impl Host for NoHost {
        fn call(&self, _: Request) -> Result<Value, String> {
            Err("no host".into())
        }
    }

    fn request(name: &str, host: &str, first: &str) -> SiteRequest {
        SiteRequest {
            name: name.into(),
            host: host.into(),
            first: first.into(),
            second: Some("h2.sub".into()),
        }
    }

    fn load(folder: &Path) -> Plugin {
        let manifest = fs::read_to_string(folder.join("manifest.json")).unwrap();
        let manifest = Manifest::parse(&manifest).unwrap();
        let source = fs::read_to_string(folder.join("main.js")).unwrap();
        Plugin::load(manifest, &source, Rc::new(NoHost)).unwrap()
    }

    #[test]
    fn the_presence_reads_only_that_site_and_the_chosen_texts() {
        let root = tempfile::tempdir().unwrap();
        let folder = create(
            root.path(),
            &request("Blog", "blog.example.com", "h1"),
            Some("Me"),
        )
        .unwrap();
        let text = fs::read_to_string(folder.join("manifest.json")).unwrap();
        let manifest = Manifest::parse(&text).unwrap();
        let page = manifest.page.unwrap();
        assert_eq!(page.matches, ["*://blog.example.com/*"]);
        assert_eq!(page.extract.len(), 2);
        assert_eq!(manifest.domains, ["blog.example.com"]);
        assert_eq!(manifest.permissions, rpp_plugins::Permissions::default());
    }

    #[test]
    fn it_shows_what_the_page_says_and_nothing_when_it_says_nothing() {
        let root = tempfile::tempdir().unwrap();
        let folder = create(
            root.path(),
            &request("Blog", "blog.example.com", "h1"),
            None,
        )
        .unwrap();
        let mut plugin = load(&folder);
        let page = |values: Value| json!({"pages": [{"url": "https://blog.example.com/a", "title": "T", "values": values, "received_at": 1}]});

        let shown = plugin
            .tick(&page(
                json!({"first": "A long title", "second": "By someone"}),
            ))
            .unwrap()
            .unwrap();
        assert_eq!(shown.details.as_deref(), Some("A long title"));
        assert_eq!(shown.state.as_deref(), Some("By someone"));

        let alone = plugin
            .tick(&page(json!({"first": "A long title"})))
            .unwrap()
            .unwrap();
        assert_eq!(alone.state.as_deref(), Some("blog.example.com"));

        assert_eq!(plugin.tick(&page(json!({"first": ""}))).unwrap(), None);
        assert_eq!(plugin.tick(&json!({"pages": []})).unwrap(), None);
    }

    #[test]
    fn what_arrives_cannot_become_code() {
        let root = tempfile::tempdir().unwrap();
        let folder = create(
            root.path(),
            &request("Blog", "blog.example.com", r#"h1"];alert(1)//"#),
            None,
        )
        .unwrap();
        let source = fs::read_to_string(folder.join("main.js")).unwrap();
        assert!(
            !source.contains("alert"),
            "a selector stays in the manifest"
        );
    }

    #[test]
    fn bad_requests_are_refused_and_write_nothing() {
        let root = tempfile::tempdir().unwrap();
        for host in [
            "",
            "localhost",
            "*.example.com",
            "a..b",
            "a b.com",
            "-a.com",
            "example.com/x",
            "exa\"mple.com",
        ] {
            assert!(
                create(root.path(), &request("Blog", host, "h1"), None).is_err(),
                "{host}"
            );
        }
        assert!(create(root.path(), &request("Blog", "a.example", ""), None).is_err());
        assert!(
            create(
                root.path(),
                &request("Blog", "a.example", &"x".repeat(300)),
                None
            )
            .is_err()
        );
        assert!(create(root.path(), &request("../up", "a.example", "h1"), None).is_err());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }
}
