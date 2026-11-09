//! A bridge that reports what it receives, to try the extension's rules.
//!
//! Usage: `pages <seconds> [presence folder]...`. Prints `pair <code>`, then a
//! line of JSON each time the extensions or their pages change.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use rpp_bridge::{Bridge, Config, Rule, generate_token};
use rpp_plugins::{Extractor, FrameSpec, Manifest};
use serde_json::json;

fn rule(presence: &str, pattern: &str) -> Rule {
    Rule {
        presence: presence.to_owned(),
        matches: vec![pattern.to_owned()],
        extract: BTreeMap::new(),
        frames: None,
    }
}

/// Parses an extractor as a manifest writes it.
fn extractor(value: serde_json::Value) -> Extractor {
    serde_json::from_value(value).expect("an extractor")
}

/// The page with an embedded player, a list and a variable of `tools/try-extension.mjs`.
fn framed() -> Rule {
    let mut rule = rule("Framed", "http://127.0.0.1/framed*");
    rule.extract = BTreeMap::from([
        (
            "tags".to_owned(),
            extractor(json!({"from": "list", "selector": "li.tag"})),
        ),
        (
            "episode".to_owned(),
            extractor(json!({"from": "variable", "path": "app.video.episode"})),
        ),
        (
            "cast".to_owned(),
            extractor(json!({"from": "variable", "path": "app.video.cast"})),
        ),
        (
            "call".to_owned(),
            extractor(json!({"from": "variable", "path": "app.video.play"})),
        ),
        (
            "next_data".to_owned(),
            extractor(json!({"from": "json", "selector": "#data", "path": "video.title"})),
        ),
        (
            "stored".to_owned(),
            extractor(json!({"from": "storage", "key": "player", "path": "track"})),
        ),
        (
            "poster".to_owned(),
            extractor(json!({"from": "style", "selector": "#poster", "name": "color"})),
        ),
        (
            "inside".to_owned(),
            extractor(json!({"from": "text", "selector": "#host >>> h2"})),
        ),
        (
            "answer".to_owned(),
            extractor(
                json!({"from": "response", "url": "http://127.0.0.1/api/early", "path": "data.title"}),
            ),
        ),
        (
            "own".to_owned(),
            extractor(json!({"from": "response", "url": "/api/early", "path": "data.title"})),
        ),
        (
            "page_saw".to_owned(),
            extractor(json!({"from": "text", "selector": "#early"})),
        ),
        (
            "xhr".to_owned(),
            extractor(
                json!({"from": "response", "url": "http://127.0.0.1/api/xhr*", "path": "now.track"}),
            ),
        ),
    ]);
    rule.frames = Some(FrameSpec {
        matches: vec!["http://localhost/*".to_owned()],
        extract: BTreeMap::from([(
            "player_title".to_owned(),
            extractor(json!({"from": "text", "selector": "h1"})),
        )]),
    });
    rule
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let seconds: u64 = std::env::args()
        .nth(1)
        .expect("seconds")
        .parse()
        .expect("a number");
    let manifests: Vec<Manifest> = std::env::args()
        .skip(2)
        .map(|folder| {
            let path = std::path::Path::new(&folder).join("manifest.json");
            let text = std::fs::read_to_string(&path).expect("a manifest");
            Manifest::parse(&text).expect("a valid manifest")
        })
        .collect();
    let bridge = Bridge::start(Config::new(0, generate_token()))
        .await
        .expect("a free port");
    let rules = || {
        let mut rules = vec![
            rule("Web Media", "*://*/*"),
            rule("Video", "http://127.0.0.1/watch*"),
            framed(),
        ];
        rules.extend(Rule::from_manifests(&manifests));
        rules
    };
    bridge.set_rules(rules());
    println!("pair {}:{}", bridge.address().port(), bridge.token());

    let deadline = Instant::now() + Duration::from_secs(seconds);
    let mut last = String::new();
    let mut sent = Instant::now();
    while Instant::now() < deadline {
        for made in bridge.take_site_requests() {
            let json = json!({"name": made.name, "host": made.host, "first": made.first, "second": made.second});
            println!("made {json}");
        }
        if sent.elapsed() >= Duration::from_secs(2) {
            bridge.set_rules(rules());
            sent = Instant::now();
        }
        let pages: Vec<_> = bridge
            .pages()
            .iter()
            .map(|page| json!({"presence": page.presence, "url": page.url, "values": page.values}))
            .collect();
        let state = json!({"extensions": bridge.extensions(), "pages": pages}).to_string();
        if state != last {
            println!("{state}");
            last = state;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    bridge.shutdown().await;
}
