//! Declarative description of what the browser extension reads from a page.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Most match patterns one presence may declare.
const MAX_MATCHES: usize = 16;
/// Most values one presence may extract.
const MAX_EXTRACTORS: usize = 32;
/// Most frame patterns one presence may declare.
const MAX_FRAMES: usize = 8;
/// Most items a list may hold.
pub const MAX_LIST_ITEMS: u8 = 32;
/// Longest path of a page variable.
const MAX_PATH: usize = 128;
/// Most parts of a page variable's path.
const MAX_PATH_PARTS: usize = 8;
/// Longest selector, attribute or meta name.
const MAX_TEXT: usize = 256;
/// Most addresses whose answers one presence reads.
const MAX_RESPONSES: usize = 8;
/// Most sites of their own a person adds to one presence.
pub const MAX_USER_SITES: usize = 8;

/// One value read from a page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "from", rename_all = "snake_case", deny_unknown_fields)]
pub enum Extractor {
    /// Visible text of the first element that matches the selector.
    Text {
        /// CSS selector.
        selector: String,
    },
    /// Value of an attribute of the first element that matches the selector.
    Attr {
        /// CSS selector.
        selector: String,
        /// Attribute name.
        name: String,
    },
    /// Content of a `<meta>` tag, found by `name` or `property`.
    Meta {
        /// Value of the `name` or `property` attribute.
        name: String,
    },
    /// Playback state of a media element: `{paused, current, duration, rate}`.
    Media {
        /// CSS selector; the first `<video>` or `<audio>` when omitted.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selector: Option<String>,
    },
    /// Every element that matches, as a list of texts, or of an attribute when
    /// `name` is given.
    List {
        /// CSS selector.
        selector: String,
        /// Attribute to read instead of the text.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        /// Most items to read; 16 when omitted, at most 32.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max: Option<u8>,
    },
    /// What the page keeps in a global variable, by its path, such as
    /// `app.video.title`: a text, a number, a boolean, or a small list or object.
    /// The path is read, and nothing of the page is called.
    Variable {
        /// Names separated by dots.
        path: String,
    },
    /// JSON the page writes in an element, such as `script#__NEXT_DATA__` or
    /// `script[type="application/ld+json"]`.
    Json {
        /// CSS selector.
        selector: String,
        /// Where to go inside it; the whole document when omitted.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<String>,
    },
    /// A value the site keeps in `localStorage` or `sessionStorage`.
    Storage {
        /// `local` or `session`.
        #[serde(default)]
        area: StorageArea,
        /// Name of the entry.
        key: String,
        /// Where to go inside it, when the entry holds JSON.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<String>,
    },
    /// A computed CSS property of the first element that matches, such as a
    /// `background-image` that holds the picture of a video.
    Style {
        /// CSS selector.
        selector: String,
        /// Property name.
        name: String,
    },
    /// What the page publishes through `navigator.mediaSession`:
    /// `{title, artist, album, artwork, state}`, or `null` when it publishes nothing.
    Session,
    /// The JSON answer the page last received from an address, such as
    /// `*://api.example.com/v1/player/*`. Only the value at `path` is kept;
    /// nothing is sent and the request itself is left as the page made it.
    Response {
        /// Match pattern of the address; it names a site.
        url: String,
        /// Where to go inside the answer; the whole answer when omitted.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<String>,
    },
}

/// Which storage of the site an entry lives in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageArea {
    /// Survives closing the browser.
    #[default]
    Local,
    /// Lasts while the tab does.
    Session,
}

/// Pages a presence reads and the values it takes from them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageSpec {
    /// Chrome match patterns such as `*://*.example.com/*`; may be empty with
    /// `user_sites`.
    #[serde(default)]
    pub matches: Vec<String>,
    /// The person adds the sites, such as a server of their own at home.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub user_sites: bool,
    /// Values to read, by the name the script will see.
    #[serde(default)]
    pub extract: BTreeMap<String, Extractor>,
    /// Embedded frames to read as well, such as a video player on another site.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frames: Option<FrameSpec>,
}

/// Frames inside a page, and the values taken from them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameSpec {
    /// Chrome match patterns of the frames, each naming a site.
    pub matches: Vec<String>,
    /// Values to read, joined to those of the page.
    pub extract: BTreeMap<String, Extractor>,
}

/// Tells whether a text is a Chrome match pattern limited to web pages.
fn valid_pattern(pattern: &str) -> bool {
    let Some((scheme, rest)) = pattern.split_once("://") else {
        return false;
    };
    if !matches!(scheme, "*" | "http" | "https") {
        return false;
    }
    let Some((host, path)) = rest.split_once('/') else {
        return false;
    };
    let host_ok = host == "*"
        || host
            .strip_prefix("*.")
            .unwrap_or(host)
            .split('.')
            .all(|label| {
                !label.is_empty()
                    && label
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            });
    host_ok && !path.contains(['\0', ' ']) && pattern.len() <= MAX_TEXT
}

/// Tells whether a text is a path of names, with nothing that could reach the
/// objects every value shares.
fn valid_path(path: &str) -> bool {
    let parts: Vec<&str> = path.split('.').collect();
    path.len() <= MAX_PATH
        && parts.len() <= MAX_PATH_PARTS
        && parts.iter().enumerate().all(|(position, part)| {
            let word = !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'$')
                && !part.as_bytes()[0].is_ascii_digit();
            let index =
                position > 0 && !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
            (word || index) && !matches!(*part, "__proto__" | "prototype" | "constructor")
        })
}

/// Tells whether a name is one of those sites use for what proves who you are.
fn names_a_credential(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    [
        "token",
        "password",
        "passwd",
        "secret",
        "cookie",
        "credential",
        "apikey",
        "api_key",
        "authorization",
        "jwt",
        "csrf",
    ]
    .iter()
    .any(|word| name.contains(word))
}

fn valid_text(text: &str) -> bool {
    !text.is_empty() && text.len() <= MAX_TEXT && !text.contains('\0')
}

/// The host of an address a person typed, such as `http://media.home:8096/web`.
///
/// Returns:
///     The host in lower case, without scheme, port or path; `None` when it is not
///     one a match pattern can name.
pub fn site_host(address: &str) -> Option<String> {
    let text = address.trim().to_ascii_lowercase();
    let rest = text
        .strip_prefix("https://")
        .or_else(|| text.strip_prefix("http://"))
        .unwrap_or(&text);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.contains('@') {
        return None;
    }
    let host = match authority.rsplit_once(':') {
        Some((host, port)) if !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) => host,
        Some(_) => return None,
        None => authority,
    };
    let fine = !host.is_empty()
        && host.len() <= 253
        && host.split('.').all(|label| {
            !label.is_empty()
                && label
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        });
    fine.then(|| host.to_owned())
}

impl PageSpec {
    /// The match pattern of a host the person added.
    pub fn user_pattern(host: &str) -> String {
        format!("*://{host}/*")
    }

    /// Checks the sizes and shapes the extension relies on.
    ///
    /// Raises:
    ///     &'static str: A short reason when a pattern, name or selector is not valid.
    pub fn validate(&self) -> Result<(), &'static str> {
        if (self.matches.is_empty() && !self.user_sites) || self.matches.len() > MAX_MATCHES {
            return Err("expected 1 to 16 match patterns, or none with user_sites");
        }
        if self.user_sites && self.matches.iter().any(|pattern| pattern.contains("://*/")) {
            return Err("a presence for every site has no sites of its own");
        }
        if !self.matches.iter().all(|pattern| valid_pattern(pattern)) {
            return Err("match patterns must look like *://*.example.com/*");
        }
        if self
            .matches
            .iter()
            .any(|pattern| pattern.starts_with("*://*/"))
            && self.frames.is_some()
        {
            return Err("a presence for every site cannot read frames");
        }
        let frames = self
            .frames
            .as_ref()
            .map_or(0, |frames| frames.extract.len());
        if self.extract.len() + frames > MAX_EXTRACTORS {
            return Err("extract at most 32 values");
        }
        check_extractors(&self.extract)?;
        let responses = self
            .extract
            .values()
            .filter(|extractor| matches!(extractor, Extractor::Response { .. }))
            .count();
        if responses > MAX_RESPONSES {
            return Err("read the answers of at most 8 addresses");
        }
        let Some(frames) = &self.frames else {
            return Ok(());
        };
        if frames.matches.is_empty() || frames.matches.len() > MAX_FRAMES {
            return Err("expected 1 to 8 frame patterns");
        }
        // A frame can be on any site, so each pattern has to say which one.
        let named = |pattern: &String| valid_pattern(pattern) && !pattern.contains("://*");
        if !frames.matches.iter().all(named) {
            return Err("frame patterns must name a site, like *://player.example.com/*");
        }
        check_extractors(&frames.extract)?;
        if frames.extract.is_empty() {
            return Err("frames need at least one value");
        }
        if frames
            .extract
            .keys()
            .any(|name| self.extract.contains_key(name))
        {
            return Err("a value of a frame cannot use a name of the page");
        }
        if frames.extract.values().any(|extractor| {
            matches!(
                extractor,
                Extractor::Variable { .. } | Extractor::Response { .. }
            )
        }) {
            return Err("variables and answers can only be read from the page");
        }
        Ok(())
    }

    /// Every pattern this presence reads: its pages and its frames.
    pub fn sites(&self) -> impl Iterator<Item = &String> {
        let frames = self.frames.iter().flat_map(|frames| &frames.matches);
        self.matches.iter().chain(frames)
    }
}

/// Why a name that looks like a login is refused.
const CREDENTIAL: &str = "a presence cannot read tokens, passwords, secrets or cookies";

/// Checks a path inside a value.
fn check_path(path: &str) -> Result<(), &'static str> {
    if !valid_path(path) {
        return Err("a path is names with dots, like app.video.title");
    }
    if names_a_credential(path) {
        return Err(CREDENTIAL);
    }
    Ok(())
}

/// Checks the names and fields of a set of extractors.
fn check_extractors(extract: &BTreeMap<String, Extractor>) -> Result<(), &'static str> {
    for (name, extractor) in extract {
        let name_ok = !name.is_empty()
            && name.len() <= 64
            && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
        if !name_ok {
            return Err("value names use letters, digits and underscores");
        }
        let fields_ok = match extractor {
            Extractor::Text { selector } => valid_text(selector),
            Extractor::Attr { selector, name } => valid_text(selector) && valid_text(name),
            Extractor::Meta { name } => valid_text(name),
            Extractor::Media { selector } => selector.as_deref().is_none_or(valid_text),
            Extractor::List {
                selector,
                name,
                max,
            } => {
                if max.is_some_and(|max| max == 0 || max > MAX_LIST_ITEMS) {
                    return Err("a list holds 1 to 32 items");
                }
                valid_text(selector) && name.as_deref().is_none_or(valid_text)
            }
            Extractor::Variable { path } => {
                check_path(path)?;
                true
            }
            Extractor::Json { selector, path } => {
                if let Some(path) = path {
                    check_path(path)?;
                }
                valid_text(selector)
            }
            Extractor::Storage { key, path, .. } => {
                if let Some(path) = path {
                    check_path(path)?;
                }
                if names_a_credential(key) {
                    return Err(CREDENTIAL);
                }
                valid_text(key)
            }
            Extractor::Style { selector, name } => valid_text(selector) && valid_text(name),
            Extractor::Session => true,
            Extractor::Response { url, path } => {
                // The address has to say which site answers, as a frame does, or start
                // with `/` for the site of the page itself.
                let same_site = url.starts_with('/') && valid_text(url) && !url.contains(' ');
                if !same_site && (!valid_pattern(url) || url.contains("://*")) {
                    return Err("an address names a site, like *://api.example.com/v1/*");
                }
                if names_a_credential(url)
                    || ["login", "oauth", "auth/"]
                        .iter()
                        .any(|w| url.to_ascii_lowercase().contains(w))
                {
                    return Err(CREDENTIAL);
                }
                if let Some(path) = path {
                    check_path(path)?;
                }
                true
            }
        };
        if !fields_ok {
            return Err("selectors and names must be 1 to 256 characters");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn spec(value: serde_json::Value) -> PageSpec {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn a_complete_specification_round_trips() {
        let original = spec(json!({
            "matches": ["*://www.example.com/watch*"],
            "extract": {
                "title": {"from": "text", "selector": "h1"},
                "channel": {"from": "attr", "selector": "a.owner", "name": "href"},
                "site": {"from": "meta", "name": "og:site_name"},
                "player": {"from": "media"},
                "playing": {"from": "session"}
            }
        }));
        assert!(original.validate().is_ok());
        let copy: PageSpec =
            serde_json::from_value(serde_json::to_value(&original).unwrap()).unwrap();
        assert_eq!(copy, original);
    }

    #[test]
    fn unknown_kinds_and_fields_are_refused() {
        for value in [
            json!({"matches": ["*://a.io/*"], "extract": {"x": {"from": "script", "code": "1"}}}),
            json!({"matches": ["*://a.io/*"], "extract": {"x": {"from": "text", "selector": "p", "extra": 1}}}),
            json!({"matches": ["*://a.io/*"], "surprise": true}),
        ] {
            assert!(serde_json::from_value::<PageSpec>(value).is_err());
        }
    }

    #[test]
    fn match_patterns_are_limited_to_web_pages() {
        for good in [
            "*://a.io/*",
            "https://www.a.io/x/*",
            "*://*.a.io/*",
            "http://localhost/*",
            "*://*/*",
        ] {
            assert!(valid_pattern(good), "{good}");
        }
        for bad in [
            "file:///etc/*",
            "chrome://settings/*",
            "a.io",
            "*://a.io",
            "ftp://a.io/*",
            "*://A.io/*",
            "*://a*.io/*",
        ] {
            assert!(!valid_pattern(bad), "{bad}");
        }
    }

    #[test]
    fn sizes_and_names_are_checked() {
        let mut page = spec(json!({"matches": ["*://a.io/*"]}));
        assert!(page.validate().is_ok());
        page.matches.clear();
        assert!(page.validate().is_err());

        let page = spec(
            json!({"matches": ["*://a.io/*"], "extract": {"bad name": {"from": "meta", "name": "x"}}}),
        );
        assert!(page.validate().is_err());
        let page = spec(
            json!({"matches": ["*://a.io/*"], "extract": {"x": {"from": "text", "selector": ""}}}),
        );
        assert!(page.validate().is_err());
    }

    fn page_with(frames: serde_json::Value) -> serde_json::Value {
        json!({"matches": ["*://www.example.com/*"], "extract": {"title": {"from": "text", "selector": "h1"}}, "frames": frames})
    }

    #[test]
    fn frames_name_their_sites_and_their_values() {
        let good = page_with(
            json!({"matches": ["*://player.example.com/*"], "extract": {"player": {"from": "media"}}}),
        );
        let parsed = spec(good);
        parsed.validate().unwrap();
        let sites: Vec<_> = parsed.sites().map(String::as_str).collect();
        assert_eq!(sites, ["*://www.example.com/*", "*://player.example.com/*"]);

        let media = json!({"player": {"from": "media"}});
        for (frames, reason) in [
            (json!({"matches": [], "extract": media}), "no patterns"),
            (
                json!({"matches": ["*://*/*"], "extract": media}),
                "every site",
            ),
            (
                json!({"matches": ["*://*.example.com/*"], "extract": media}),
                "a wildcard host",
            ),
            (
                json!({"matches": ["*://a.example/*"], "extract": {}}),
                "no values",
            ),
            (
                json!({"matches": ["*://a.example/*"], "extract": {"title": {"from": "media"}}}),
                "a name of the page",
            ),
            (
                json!({"matches": ["*://a.example/*"], "extract": {"v": {"from": "variable", "path": "a.b"}}}),
                "a variable",
            ),
        ] {
            assert!(spec(page_with(frames)).validate().is_err(), "{reason}");
        }
    }

    #[test]
    fn a_presence_for_every_site_reads_no_frames() {
        let page = json!({"matches": ["*://*/*"], "frames": {"matches": ["*://a.example/*"], "extract": {"p": {"from": "media"}}}});
        assert!(spec(page).validate().is_err());
    }

    fn extractor(value: serde_json::Value) -> Result<(), &'static str> {
        spec(json!({"matches": ["*://a.example/*"], "extract": {"x": value}})).validate()
    }

    #[test]
    fn an_answer_is_read_from_an_address_that_names_a_site() {
        let page = |extract: serde_json::Value| {
            serde_json::from_value::<PageSpec>(
                json!({"matches": ["*://www.example.com/*"], "extract": extract}),
            )
            .unwrap()
            .validate()
        };
        let read = |url: &str, path: Option<&str>| {
            let mut spec = json!({"from": "response", "url": url});
            if let Some(path) = path {
                spec["path"] = json!(path);
            }
            page(json!({ "v": spec }))
        };
        assert!(read("*://api.example.com/v1/player/*", Some("data.title")).is_ok());
        assert!(read("https://www.example.com/api/now?*", None).is_ok());
        for (url, path) in [
            ("*://*/*", None),
            ("*://*.example.com/*", None),
            ("api.example.com/v1", None),
            ("*://api.example.com/oauth/*", None),
            ("*://api.example.com/v1/token", None),
            ("*://api.example.com/LOGIN/*", None),
            ("*://api.example.com/v1/*", Some("user.password")),
            ("*://api.example.com/v1/*", Some("a()")),
        ] {
            assert!(read(url, path).is_err(), "{url} {path:?}");
        }
        let many: serde_json::Map<String, serde_json::Value> = (0..9)
            .map(|n| {
                (
                    format!("v{n}"),
                    json!({"from": "response", "url": "*://api.example.com/*"}),
                )
            })
            .collect();
        assert!(page(serde_json::Value::Object(many)).is_err());
        let framed = json!({
            "matches": ["*://www.example.com/*"],
            "frames": {"matches": ["*://player.example.com/*"],
                       "extract": {"v": {"from": "response", "url": "*://api.example.com/*"}}}
        });
        assert!(
            serde_json::from_value::<PageSpec>(framed)
                .unwrap()
                .validate()
                .is_err()
        );
    }

    #[test]
    fn a_presence_may_leave_its_sites_to_the_person() {
        let spec = |value: serde_json::Value| {
            serde_json::from_value::<PageSpec>(value)
                .unwrap()
                .validate()
        };
        let media = json!({"player": {"from": "media"}});
        assert!(spec(json!({"user_sites": true, "extract": media})).is_ok());
        assert!(
            spec(
                json!({"matches": ["*://demo.example.com/*"], "user_sites": true, "extract": media})
            )
            .is_ok()
        );
        assert!(spec(json!({"extract": media})).is_err());
        assert!(
            spec(json!({"matches": ["*://*/*"], "user_sites": true, "extract": media})).is_err()
        );
        let own = json!({"v": {"from": "response", "url": "/api/now?*", "path": "item.name"}});
        assert!(spec(json!({"user_sites": true, "extract": own})).is_ok());
        let spaced = json!({"v": {"from": "response", "url": "/api/a b"}});
        assert!(spec(json!({"user_sites": true, "extract": spaced})).is_err());
        let login = json!({"v": {"from": "response", "url": "/auth/me"}});
        assert!(spec(json!({"user_sites": true, "extract": login})).is_err());
    }

    #[test]
    fn a_typed_address_becomes_a_host() {
        for (typed, host) in [
            ("media.home.lan", "media.home.lan"),
            ("http://192.168.1.5:8096/web/index.html", "192.168.1.5"),
            ("  HTTPS://Jelly.Example.com/  ", "jelly.example.com"),
            ("nas:32400", "nas"),
            ("https://music.example.com?x=1", "music.example.com"),
        ] {
            assert_eq!(site_host(typed).as_deref(), Some(host), "{typed}");
        }
        for typed in [
            "",
            "ftp://a.b",
            "user@host.com",
            "a..b",
            "host:port",
            "*.example.com",
            "a b.com",
            "[::1]",
        ] {
            assert_eq!(site_host(typed), None, "{typed}");
        }
        assert_eq!(PageSpec::user_pattern("nas"), "*://nas/*");
    }

    #[test]
    fn a_variable_is_a_path_and_nothing_else() {
        for good in ["title", "app.video.title", "$state.items.0.name", "_data"] {
            assert!(
                extractor(json!({"from": "variable", "path": good})).is_ok(),
                "{good}"
            );
        }
        for bad in [
            "",
            "a..b",
            "a.",
            "0.a",
            "a()",
            "a[0]",
            "a b",
            "alert(1)",
            "__proto__.x",
            "a.constructor",
            "a.prototype.b",
            "a.b.c.d.e.f.g.h.i",
        ] {
            assert!(
                extractor(json!({"from": "variable", "path": bad})).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn a_list_has_a_limit() {
        assert!(extractor(json!({"from": "list", "selector": "li"})).is_ok());
        assert!(
            extractor(json!({"from": "list", "selector": "a", "name": "href", "max": 32})).is_ok()
        );
        assert!(extractor(json!({"from": "list", "selector": "li", "max": 0})).is_err());
        assert!(extractor(json!({"from": "list", "selector": "li", "max": 33})).is_err());
        assert!(extractor(json!({"from": "list", "selector": ""})).is_err());
    }

    #[test]
    fn json_storage_and_style_read_what_they_name() {
        for good in [
            json!({"from": "json", "selector": "script#__NEXT_DATA__", "path": "props.pageProps.title"}),
            json!({"from": "json", "selector": "script[type='application/ld+json']"}),
            json!({"from": "storage", "key": "player-state", "path": "track.title"}),
            json!({"from": "storage", "area": "session", "key": "queue"}),
            json!({"from": "style", "selector": ".poster", "name": "background-image"}),
        ] {
            assert!(extractor(good.clone()).is_ok(), "{good}");
        }
        for bad in [
            json!({"from": "json", "selector": ""}),
            json!({"from": "json", "selector": "script", "path": "a()"}),
            json!({"from": "storage", "key": ""}),
            json!({"from": "style", "selector": "a", "name": ""}),
        ] {
            assert!(extractor(bad.clone()).is_err(), "{bad}");
        }
    }

    /// A review can miss a line; the manifest cannot be loaded with one of these.
    #[test]
    fn nothing_that_proves_who_someone_is_can_be_read() {
        for bad in [
            json!({"from": "storage", "key": "accessToken"}),
            json!({"from": "storage", "key": "user_password"}),
            json!({"from": "storage", "key": "state", "path": "user.apikey"}),
            json!({"from": "variable", "path": "app.session.jwt"}),
            json!({"from": "variable", "path": "window.csrfValue"}),
            json!({"from": "json", "selector": "script", "path": "auth.authorization"}),
            json!({"from": "variable", "path": "document.cookie"}),
        ] {
            assert_eq!(extractor(bad.clone()), Err(CREDENTIAL), "{bad}");
        }
        assert!(extractor(json!({"from": "variable", "path": "video.author.name"})).is_ok());
    }
}
