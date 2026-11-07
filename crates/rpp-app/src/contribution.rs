//! Getting a presence ready to propose for the catalog.

use std::fs;
use std::path::Path;

use rpp_plugins::is_known_tag;
use serde_json::Value;

use crate::files::{Files, read_folder};
use crate::templates::folder_name;
use crate::views::{PreparedView, ReviewCheck};

/// Most files, bytes in one file and bytes in all, as the catalog index allows.
const MAX_FILES: usize = 128;
const MAX_FILE: usize = 1024 * 1024;
const MAX_TOTAL: usize = 8 * 1024 * 1024;

/// A prefix and how many token characters must follow it.
const PREFIXES: [(&str, &str, usize); 7] = [
    ("ghp_", "a GitHub token", 20),
    ("gho_", "a GitHub token", 20),
    ("github_pat_", "a GitHub token", 20),
    ("AKIA", "an AWS key", 16),
    ("xoxb-", "a Slack token", 20),
    ("xoxp-", "a Slack token", 20),
    ("sk-", "an API key", 24),
];

/// Text whose presence alone is enough.
const MARKERS: [(&str, &str); 4] = [
    ("PRIVATE KEY-----", "a private key"),
    ("discord.com/api/webhooks/", "a Discord webhook"),
    ("discordapp.com/api/webhooks/", "a Discord webhook"),
    ("client_secret", "an OAuth client secret"),
];

fn token_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'
}

/// Length of the run of token characters at the start of a text.
fn token_length(text: &str) -> usize {
    text.bytes().take_while(|byte| token_byte(*byte)).count()
}

/// A Discord token: three parts joined by dots, of about 24, 6 and 27 characters.
fn has_discord_token(text: &str) -> bool {
    text.split(|c: char| !(c.is_ascii() && (token_byte(c as u8) || c == '.')))
        .any(|word| {
            let mut parts = word.split('.');
            match (parts.next(), parts.next(), parts.next(), parts.next()) {
                (Some(first), Some(second), Some(third), None) => {
                    first.len() >= 24 && second.len() == 6 && third.len() >= 27
                }
                _ => false,
            }
        })
}

/// What a text seems to hold, if it looks like a credential.
fn credential(text: &str) -> Option<&'static str> {
    // Whoever pastes a secret does not choose its case.
    let lower = text.to_lowercase();
    if let Some((_, kind)) = MARKERS
        .iter()
        .find(|(marker, _)| lower.contains(&marker.to_lowercase()))
    {
        return Some(kind);
    }
    for (prefix, kind, length) in PREFIXES {
        let found = text
            .match_indices(prefix)
            .any(|(at, _)| token_length(&text[at + prefix.len()..]) >= length);
        if found {
            return Some(kind);
        }
    }
    has_discord_token(text).then_some("a Discord token")
}

/// Fails on the first file that looks like it holds a credential.
fn refuse_credentials(files: &Files) -> Result<(), String> {
    for (path, bytes) in files {
        let Ok(text) = std::str::from_utf8(bytes) else {
            continue;
        };
        if let Some(kind) = credential(text) {
            return Err(format!(
                "{path} looks like it holds {kind}. Remove it before sending the presence."
            ));
        }
    }
    Ok(())
}

fn check(rule: &'static str, passed: bool, detail: Option<String>) -> ReviewCheck {
    ReviewCheck {
        rule,
        passed,
        detail: detail.filter(|_| !passed),
    }
}

fn text_of<'a>(manifest: &'a Value, field: &str) -> &'a str {
    manifest[field].as_str().map_or("", str::trim)
}

fn filled(rule: &'static str, manifest: &Value) -> ReviewCheck {
    check(rule, !text_of(manifest, rule).is_empty(), None)
}

fn tags_check(manifest: &Value) -> ReviewCheck {
    let tags: Vec<&str> = manifest["tags"]
        .as_array()
        .map(|tags| tags.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let unknown: Vec<&str> = tags
        .iter()
        .copied()
        .filter(|tag| !is_known_tag(tag))
        .collect();
    check(
        "tags",
        !tags.is_empty() && unknown.is_empty(),
        (!unknown.is_empty()).then(|| unknown.join(", ")),
    )
}

fn version_check(manifest: &Value) -> ReviewCheck {
    let version = text_of(manifest, "version");
    check(
        "version",
        semver::Version::parse(version).is_ok_and(|parsed| parsed.build.is_empty()),
        Some(version.to_owned()),
    )
}

fn size_check(files: &Files) -> ReviewCheck {
    let large = files
        .iter()
        .find(|(_, bytes)| bytes.len() > MAX_FILE)
        .map(|(path, _)| path.clone());
    let total: usize = files.values().map(Vec::len).sum();
    check(
        "size",
        large.is_none() && files.len() <= MAX_FILES && total <= MAX_TOTAL,
        large,
    )
}

/// Checks a folder against the rules of the catalog.
///
/// Returns:
///     Every rule, with what to fix in the ones the presence does not follow.
pub(crate) fn review(folder: &Path) -> Result<Vec<ReviewCheck>, String> {
    let files = read_folder(folder)?;
    let manifest = files
        .get("manifest.json")
        .ok_or("The presence has no manifest.")?;
    let manifest: Value = serde_json::from_slice(manifest).map_err(|e| e.to_string())?;
    let key = folder
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .ok_or("The presence has no folder name.")?;
    let credential = refuse_credentials(&files).err();
    Ok(vec![
        check(
            "folder",
            folder_name(&key).as_deref() == Some(&key),
            Some(key),
        ),
        version_check(&manifest),
        filled("description", &manifest),
        filled("author", &manifest),
        tags_check(&manifest),
        check("english", files.contains_key("locales/en.json"), None),
        size_check(&files),
        check("secrets", credential.is_none(), credential),
    ])
}

/// Writes a clean copy of a presence, without the editor's files.
///
/// Raises:
///     String: The presence breaks a rule of the catalog, or a file cannot be
///         read or written.
pub(crate) fn prepare(folder: &Path, out: &Path) -> Result<PreparedView, String> {
    if let Some(failed) = review(folder)?.into_iter().find(|check| !check.passed) {
        let detail = failed.detail.unwrap_or_default();
        return Err(format!("The {} check failed. {detail}", failed.rule));
    }
    let files = read_folder(folder)?;
    let key = folder
        .file_name()
        .ok_or("The presence has no folder name.")?;
    let target = out.join(key);
    let temporary = out.join(format!(".{}.tmp", key.to_string_lossy()));
    let written = write_copy(&temporary, &files).and_then(|()| {
        if target.exists() {
            fs::remove_dir_all(&target)?;
        }
        fs::rename(&temporary, &target)
    });
    if let Err(error) = written {
        let _ = fs::remove_dir_all(&temporary);
        return Err(error.to_string());
    }
    Ok(PreparedView {
        path: target.to_string_lossy().into_owned(),
        files: files.len(),
        size: files.values().map(|bytes| bytes.len() as u64).sum(),
    })
}

fn write_copy(target: &Path, files: &Files) -> std::io::Result<()> {
    if target.exists() {
        fs::remove_dir_all(target)?;
    }
    for (path, bytes) in files {
        let file = target.join(path);
        if let Some(parent) = file.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(file, bytes)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const READY: &str = r#"{"name":"Demo","version":"1.2.0","client_id":"12345678901234567",
        "description":"Shows a demo.","author":"Ana","tags":["video"]}"#;

    /// A folder that follows every rule.
    fn ready(root: &Path) -> std::path::PathBuf {
        let folder = root.join("demo");
        fs::create_dir_all(folder.join("locales")).unwrap();
        fs::create_dir_all(folder.join(".vscode")).unwrap();
        fs::write(folder.join("manifest.json"), READY).unwrap();
        fs::write(folder.join("main.js"), "code").unwrap();
        fs::write(folder.join("locales/en.json"), "{}").unwrap();
        fs::write(folder.join("jsconfig.json"), "{}").unwrap();
        fs::write(folder.join(".vscode/settings.json"), "{}").unwrap();
        folder
    }

    fn failed(folder: &Path) -> Vec<&'static str> {
        review(folder)
            .unwrap()
            .into_iter()
            .filter(|check| !check.passed)
            .map(|check| check.rule)
            .collect()
    }

    /// Credentials are put together here so no scanner mistakes this file for
    /// one that holds real ones.
    fn fake(prefix: &str, length: usize) -> String {
        format!("{prefix}{}", "a1".repeat(length / 2 + 1))
    }

    #[test]
    fn a_secret_is_found_in_any_case() {
        assert!(credential("const CLIENT_SECRET = 1").is_some());
        assert!(credential("https://DISCORDAPP.com/api/webhooks/1/x").is_some());
        assert!(credential("a plain script").is_none());
    }

    #[test]
    fn common_credentials_are_recognized() {
        let discord = format!("{}.{}.{}", "a".repeat(24), "b".repeat(6), "c".repeat(27));
        let webhook = format!("https://discord.com/api/{}/1/abc", "webhooks");
        let key = format!("-----BEGIN RSA {}-----", "PRIVATE KEY");
        for text in [
            format!("const token = '{}';", fake("ghp_", 36)),
            format!("x={}", fake("github_pat_", 40)),
            format!("{} rest", fake("AKIA", 16).to_uppercase()),
            format!("Authorization: {}", fake("sk-", 30)),
            format!("token: \"{discord}\""),
            webhook,
            key,
            "client_secret: 123".to_owned(),
        ] {
            assert!(credential(&text).is_some(), "{text}");
        }
    }

    #[test]
    fn ordinary_code_passes() {
        for text in [
            "function read(c){ return { title: c.pages[0].title } }",
            "const sk = 'task-list'; // ghp_ is a prefix",
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
            "a.b.c",
            "1.2.3",
            "\"client_id\": \"12345678901234567\"",
            "www.example-domain-with-long-name.com",
        ] {
            assert_eq!(credential(text), None, "{text}");
        }
    }

    #[test]
    fn the_refusal_names_the_file_and_not_the_secret() {
        let secret = fake("ghp_", 36);
        let mut files = Files::new();
        files.insert("main.js".into(), secret.clone().into_bytes());
        let error = refuse_credentials(&files).unwrap_err();
        assert!(error.contains("main.js") && error.contains("GitHub"));
        assert!(!error.contains(&secret));
    }

    #[test]
    fn a_picture_is_not_scanned() {
        let mut files = Files::new();
        files.insert("logo.png".into(), vec![0xff, 0xfe, 0x00, 0x80]);
        assert!(refuse_credentials(&files).is_ok());
    }

    #[test]
    fn a_ready_presence_passes_every_rule() {
        let root = tempfile::tempdir().unwrap();
        assert!(failed(&ready(root.path())).is_empty());
    }

    #[test]
    fn each_rule_names_what_is_wrong() {
        let cases = [
            (r#""version":"1.2.0""#, r#""version":"1.2.0+7""#, "version"),
            (r#""description":"Shows a demo.","#, "", "description"),
            (r#""author":"Ana","#, "", "author"),
            (r#","tags":["video"]"#, "", "tags"),
            (r#""tags":["video"]"#, r#""tags":["invented"]"#, "tags"),
        ];
        for (old, new, rule) in cases {
            let root = tempfile::tempdir().unwrap();
            let folder = ready(root.path());
            let text = READY.replace(old, new);
            assert_ne!(text, READY, "{rule}");
            fs::write(folder.join("manifest.json"), text).unwrap();
            assert_eq!(failed(&folder), [rule]);
        }
    }

    #[test]
    fn a_tag_off_the_list_is_named() {
        let root = tempfile::tempdir().unwrap();
        let folder = ready(root.path());
        fs::write(
            folder.join("manifest.json"),
            READY.replace(r#"["video"]"#, r#"["video","invented"]"#),
        )
        .unwrap();
        let checks = review(&folder).unwrap();
        let tags = checks.iter().find(|check| check.rule == "tags").unwrap();
        assert!(tags.detail.as_deref().unwrap().contains("invented"));
    }

    #[test]
    fn missing_english_large_files_and_secrets_are_found() {
        let root = tempfile::tempdir().unwrap();
        let folder = ready(root.path());
        fs::remove_file(folder.join("locales/en.json")).unwrap();
        fs::write(folder.join("big.bin"), vec![0_u8; MAX_FILE + 1]).unwrap();
        fs::write(folder.join("main.js"), fake("ghp_", 36)).unwrap();
        assert_eq!(failed(&folder), ["english", "size", "secrets"]);
    }

    #[test]
    fn a_folder_with_another_shape_of_name_is_refused() {
        let root = tempfile::tempdir().unwrap();
        let folder = ready(root.path());
        let renamed = root.path().join("My Demo");
        fs::rename(&folder, &renamed).unwrap();
        assert_eq!(failed(&renamed), ["folder"]);
    }

    #[test]
    fn the_copy_leaves_the_editor_files_out_and_replaces_the_last_one() {
        let root = tempfile::tempdir().unwrap();
        let folder = ready(root.path());
        let out = root.path().join("exports");

        let view = prepare(&folder, &out).unwrap();
        assert_eq!(view.files, 3);
        assert_eq!(view.size, READY.len() as u64 + 6);
        let copy = out.join("demo");
        assert_eq!(view.path, copy.to_string_lossy());
        let copied = read_folder(&copy).unwrap();
        assert_eq!(
            copied.keys().map(String::as_str).collect::<Vec<_>>(),
            ["locales/en.json", "main.js", "manifest.json"]
        );
        assert!(!copy.join(".vscode").exists() && !copy.join("jsconfig.json").exists());

        fs::write(copy.join("left-over.js"), "old").unwrap();
        prepare(&folder, &out).unwrap();
        assert!(!copy.join("left-over.js").exists());
        assert_eq!(fs::read_dir(&out).unwrap().count(), 1, "a temporary stayed");
    }

    #[test]
    fn nothing_is_written_when_a_rule_fails() {
        let root = tempfile::tempdir().unwrap();
        let folder = ready(root.path());
        fs::write(folder.join("main.js"), fake("ghp_", 36)).unwrap();
        let out = root.path().join("exports");
        let error = prepare(&folder, &out).unwrap_err();
        assert!(error.contains("main.js"), "{error}");
        assert!(!out.exists());
    }
}
