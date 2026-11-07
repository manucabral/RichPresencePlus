//! From a browser extension message to an activity on Discord, end to end.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use rpp_bridge::{Bridge, Config, Rule, generate_token};
use rpp_core::Preferences;
use rpp_plugins::Manifest;
use rpp_service::{Presence, Service};
use rpp_testkit::extension::Extension;
use rpp_testkit::{Discord, Silent, eventually};
use serde_json::{Value, json};

#[tokio::test(flavor = "multi_thread")]
async fn a_page_report_becomes_a_discord_activity_and_leaving_clears_it() {
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../presences/youtube");
    let mut manifest =
        Manifest::parse(&fs::read_to_string(folder.join("manifest.json")).unwrap()).unwrap();
    let source = fs::read_to_string(folder.join(&manifest.entry)).unwrap();
    manifest.interval = 1;
    let strings =
        serde_json::from_str(&fs::read_to_string(folder.join("locales/en.json")).unwrap()).unwrap();

    let bridge = Bridge::start(Config::new(0, generate_token()))
        .await
        .unwrap();
    bridge.set_rules(Rule::from_manifests([&manifest]));
    let discord = Discord::default();
    let service = Service::start(
        discord.clone(),
        Silent,
        vec![Presence {
            manifest,
            source,
            strings,
            settings: Default::default(),
        }],
        Arc::new(bridge.page_source()),
        Preferences::default(),
    );
    bridge.on_page_changes(service.waker());

    // The extension pairs and reports a playing video.
    let mut extension = Extension::pair(bridge.address(), bridge.token()).await;
    let report = json!({
        "type": "page", "tab": 3, "presence": "YouTube",
        "url": "https://www.youtube.com/watch?v=abc123xyz", "title": "A video - YouTube",
        "values": {"title": "A video", "author": "A channel",
                   "player": {"paused": false, "current": 10.0, "duration": 100.0, "rate": 1.0}}
    });
    extension.send(report).await;

    eventually("the activity", || {
        !discord.activities.lock().unwrap().is_empty()
    })
    .await;
    let shown = discord.activities.lock().unwrap()[0].clone();
    assert_eq!(shown["details"], "A video");
    assert_eq!(shown["state"], "By A channel");
    assert_eq!(shown["type"], 3);
    assert_eq!(
        shown["timestamps"]["end"].as_u64().unwrap()
            - shown["timestamps"]["start"].as_u64().unwrap(),
        100
    );
    assert_eq!(service.status().winner.as_deref(), Some("YouTube"));

    // The tab closes: the activity is removed.
    extension.send(json!({"type": "gone", "tab": 3})).await;
    eventually("the activity to clear", || {
        discord.activities.lock().unwrap().last() == Some(&Value::Null)
    })
    .await;

    tokio::task::spawn_blocking(move || service.shutdown())
        .await
        .unwrap();
    bridge.shutdown().await;
}
