//! Messages exchanged with the browser extension, as JSON text frames.

use std::collections::BTreeMap;

use rpp_plugins::{Extractor, FrameSpec, Manifest};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Browser an extension runs in, as the interface names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Browser {
    /// Google Chrome, and browsers built on it that say they are Chrome.
    Chrome,
    /// Microsoft Edge.
    Edge,
    /// Mozilla Firefox.
    Firefox,
    /// Opera and Opera GX.
    Opera,
    /// Brave.
    Brave,
    /// Vivaldi.
    Vivaldi,
    /// Yandex Browser.
    Yandex,
    /// A name this build does not know.
    Other,
}

impl Browser {
    /// Reads the name the extension sends in its hello.
    pub fn from_hello(name: &str) -> Self {
        match name {
            "chrome" => Self::Chrome,
            "edge" => Self::Edge,
            "firefox" => Self::Firefox,
            "opera" => Self::Opera,
            "brave" => Self::Brave,
            "vivaldi" => Self::Vivaldi,
            "yandex" => Self::Yandex,
            _ => Self::Other,
        }
    }
}

/// Version of this protocol; the extension must send the same number.
pub const VERSION: u32 = 2;

/// What the extension reads on pages that match, for one presence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Rule {
    /// Name of the presence that consumes the values.
    pub presence: String,
    /// Chrome match patterns.
    pub matches: Vec<String>,
    /// Values to read, by name.
    pub extract: BTreeMap<String, Extractor>,
    /// Embedded frames to read as well.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frames: Option<FrameSpec>,
}

impl Rule {
    /// One rule for each manifest with a `page` section.
    pub fn from_manifests<'a>(manifests: impl IntoIterator<Item = &'a Manifest>) -> Vec<Rule> {
        manifests
            .into_iter()
            .filter_map(|manifest| {
                // A presence whose person has not added a site yet reads nothing.
                let page = manifest
                    .page
                    .as_ref()
                    .filter(|page| !page.matches.is_empty())?;
                Some(Rule {
                    presence: manifest.name.clone(),
                    matches: page.matches.clone(),
                    extract: page.extract.clone(),
                    frames: page.frames.clone(),
                })
            })
            .collect()
    }
}

/// Messages from the application to the extension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToExtension {
    /// First message, sent in the clear: a random value the extension proves
    /// it knows the token against.
    Challenge {
        /// Protocol version the server speaks.
        version: u32,
        /// Random value, as base64.
        nonce: String,
    },
    /// The hello was accepted, sent in the clear. The proof shows the
    /// extension that it reached the real application.
    Welcome {
        /// Protocol version the server speaks.
        version: u32,
        /// HMAC of both random values keyed with the token, as base64.
        proof: String,
    },
    /// The complete set of rules; replaces any earlier set.
    Rules {
        /// Rules of the loaded presences.
        rules: Vec<Rule>,
    },
    /// Answer to a ping.
    Pong,
}

/// Messages from the extension to the application.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FromExtension {
    /// Answer to the challenge, sent in the clear: proves the extension
    /// knows the token without sending it.
    Hello {
        /// Browser name, such as `chrome`.
        browser: String,
        /// Browser version, such as `24.12.1.712`; older extensions omit it.
        #[serde(default)]
        browser_version: String,
        /// Protocol version.
        version: u32,
        /// The extension's own random value, as base64.
        nonce: String,
        /// HMAC of both random values keyed with the token, as base64.
        proof: String,
    },
    /// Values read from a tab.
    Page {
        /// Browser tab identifier.
        tab: i64,
        /// Presence whose rule produced the values.
        presence: String,
        /// Address of the page.
        url: String,
        /// Title of the page.
        title: String,
        /// Extracted values, by name.
        values: Map<String, Value>,
        /// Whether the tab is on screen; older extensions omit it.
        #[serde(default)]
        visible: bool,
        /// Whether the tab has the keyboard focus; older extensions omit it.
        #[serde(default)]
        focused: bool,
    },
    /// Someone clicked the texts of a new presence on a page.
    MakeSite {
        /// Name for the presence.
        name: String,
        /// Host of the page, taken from the tab.
        host: String,
        /// Selector of the first line.
        first: String,
        /// Selector of the second line, when one was chosen.
        #[serde(default)]
        second: Option<String>,
    },
    /// A tab closed or left the matching pages.
    Gone {
        /// Browser tab identifier.
        tab: i64,
    },
    /// Keep-alive.
    Ping,
}

/// A presence someone asked for from a page, waiting for the application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteRequest {
    /// Name for the presence.
    pub name: String,
    /// Host of the page.
    pub host: String,
    /// Selector of the first line.
    pub first: String,
    /// Selector of the second line.
    pub second: Option<String>,
}

/// How every message travels once the handshake is over.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Envelope {
    /// A message sealed with the session key; see [`crate::secure`].
    Sealed {
        /// Position of the message in its direction, starting at 0.
        n: u64,
        /// The sealed message, as base64.
        data: String,
    },
}

/// A page the extension reported, as scripts see it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Page {
    /// Browser that reported the page.
    pub browser: String,
    /// Browser tab identifier.
    pub tab: i64,
    /// Presence whose rule produced the values.
    pub presence: String,
    /// Address of the page.
    pub url: String,
    /// Title of the page.
    pub title: String,
    /// Extracted values, by name.
    pub values: Map<String, Value>,
    /// Moment the application received the report, in Unix milliseconds.
    pub received_at: u64,
    /// Whether the tab is on screen: not behind another tab, minimized or covered.
    pub visible: bool,
    /// Whether the tab has the keyboard focus.
    pub focused: bool,
    /// Last report that had the focus, in Unix milliseconds; 0 if none had it.
    /// Among tabs on screen, the highest is the one the person used last.
    pub focused_at: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extension_messages_parse() {
        let hello: FromExtension = serde_json::from_value(
            json!({"type": "hello", "browser": "chrome", "version": 2, "nonce": "n", "proof": "p"}),
        )
        .unwrap();
        assert!(matches!(hello, FromExtension::Hello { version: 2, .. }));
        // The token itself is never part of a message.
        assert!(
            serde_json::from_value::<FromExtension>(
                json!({"type": "hello", "token": "t", "browser": "chrome", "version": 2})
            )
            .is_err()
        );
        let page: FromExtension = serde_json::from_value(json!({
            "type": "page", "tab": 7, "presence": "Video", "url": "https://a.io/",
            "title": "T", "values": {"title": "x"}
        }))
        .unwrap();
        // An extension from before the fields says nothing about the screen.
        assert!(matches!(
            page,
            FromExtension::Page {
                tab: 7,
                visible: false,
                focused: false,
                ..
            }
        ));
        let seen: FromExtension = serde_json::from_value(json!({
            "type": "page", "tab": 7, "presence": "Video", "url": "https://a.io/",
            "title": "T", "values": {}, "visible": true, "focused": true
        }))
        .unwrap();
        assert!(matches!(
            seen,
            FromExtension::Page {
                visible: true,
                focused: true,
                ..
            }
        ));
        assert!(
            serde_json::from_value::<FromExtension>(json!({"type": "eval", "code": "1"})).is_err()
        );
    }

    #[test]
    fn server_messages_serialize_with_a_type_tag() {
        assert_eq!(
            serde_json::to_value(ToExtension::Welcome {
                version: VERSION,
                proof: "p".into()
            })
            .unwrap(),
            json!({"type": "welcome", "version": 2, "proof": "p"})
        );
        assert_eq!(
            serde_json::to_value(ToExtension::Pong).unwrap(),
            json!({"type": "pong"})
        );
    }

    #[test]
    fn only_manifests_with_pages_make_rules() {
        let with = Manifest::parse(
            r#"{"name":"Video","version":"1","client_id":"12345678901234567",
                "page":{"matches":["*://a.io/*"],"extract":{"t":{"from":"meta","name":"og:title"}}}}"#,
        )
        .unwrap();
        let without =
            Manifest::parse(r#"{"name":"Game","version":"1","client_id":"12345678901234567"}"#)
                .unwrap();
        // Sites of one's own that nobody added yet: nothing to read.
        let waiting = Manifest::parse(
            r#"{"name":"Server","version":"1","client_id":"12345678901234567",
                "page":{"user_sites":true,"extract":{"t":{"from":"meta","name":"og:title"}}}}"#,
        )
        .unwrap();
        let rules = Rule::from_manifests([&with, &without, &waiting]);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].presence, "Video");
        assert_eq!(
            serde_json::to_value(&rules[0]).unwrap()["extract"]["t"],
            json!({"from": "meta", "name": "og:title"})
        );
    }
}

#[cfg(test)]
mod browser_tests {
    use super::Browser;

    #[test]
    fn hello_names_map_to_a_fixed_set() {
        assert_eq!(Browser::from_hello("chrome"), Browser::Chrome);
        assert_eq!(Browser::from_hello("edge"), Browser::Edge);
        assert_eq!(Browser::from_hello("firefox"), Browser::Firefox);
        assert_eq!(Browser::from_hello("yandex"), Browser::Yandex);
        assert_eq!(Browser::from_hello("opera"), Browser::Opera);
        assert_eq!(Browser::from_hello("brave"), Browser::Brave);
        assert_eq!(Browser::from_hello("vivaldi"), Browser::Vivaldi);
        for other in ["", "Chrome", "<b>x</b>", "netscape"] {
            assert_eq!(Browser::from_hello(other), Browser::Other, "{other}");
        }
        assert_eq!(serde_json::to_value(Browser::Edge).unwrap(), "edge");
    }
}
