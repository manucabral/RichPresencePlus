//! The real YouTube script, fed with pages as the extension reports them.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use rpp_plugins::{Host, Manifest, Plugin, Request};
use serde_json::{Value, json};

struct Silent;

impl Host for Silent {
    fn call(&self, _: Request) -> Result<Value, String> {
        Err("no host".into())
    }
}

fn folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../presences/youtube")
}

fn texts(tag: &str) -> BTreeMap<String, String> {
    let file = folder().join("locales").join(format!("{tag}.json"));
    serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap()
}

fn start() -> Plugin {
    start_in("en")
}

fn start_in(tag: &str) -> Plugin {
    let folder = folder();
    let manifest =
        Manifest::parse(&fs::read_to_string(folder.join("manifest.json")).unwrap()).unwrap();
    let source = fs::read_to_string(folder.join(&manifest.entry)).unwrap();
    let mut plugin = Plugin::load(manifest, &source, Rc::new(Silent)).unwrap();
    let mut strings = texts("en");
    strings.extend(texts(tag));
    plugin.set_strings(&strings).unwrap();
    plugin
}

const RECEIVED: u64 = 1_800_000_000_000;

fn watch(title: &str, current: f64, paused: bool) -> Value {
    json!({
        "browser": "chrome", "tab": 1, "presence": "YouTube",
        "url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ&list=PLprivate&t=42s",
        "title": format!("{title} - YouTube"),
        "received_at": RECEIVED,
        "values": {
            "title": title,
            "author": "Some Channel",
            "author_url": "https://www.youtube.com/@somechannel",
            "player": {"paused": paused, "current": current, "duration": 212.0, "rate": 1.0}
        }
    })
}

fn tick(plugin: &mut Plugin, pages: Value) -> rpp_plugins::Tick {
    plugin
        .poll(&json!({"now": 1_800_000_100, "pages": pages}))
        .unwrap()
}

#[test]
fn a_playing_video_shows_title_channel_times_image_and_clean_buttons() {
    let mut plugin = start();
    let shown = tick(&mut plugin, json!([watch("A Song", 60.0, false)]))
        .activity
        .unwrap();
    assert_eq!(shown.details.as_deref(), Some("A Song"));
    assert_eq!(shown.state.as_deref(), Some("By Some Channel"));
    // Position 60 s at the moment of the report: the video began 60 s earlier.
    assert_eq!(shown.start, Some(1_800_000_000 - 60));
    assert_eq!(shown.end, Some(1_800_000_000 - 60 + 212));
    assert_eq!(
        shown.large_image.as_deref(),
        Some("https://i.ytimg.com/vi/dQw4w9WgXcQ/hqdefault.jpg")
    );
    assert_eq!(
        shown.buttons[0].url,
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
    );
    assert_eq!(shown.buttons[1].label, "Author");
    assert_eq!(shown.small_text.as_deref(), Some("Playing"));
}

#[test]
fn the_playlist_and_time_parameters_never_leave_the_machine() {
    let mut plugin = start();
    let shown = tick(&mut plugin, json!([watch("A Song", 1.0, false)]))
        .activity
        .unwrap();
    for button in &shown.buttons {
        assert!(!button.url.contains("list="), "{}", button.url);
        assert!(!button.url.contains("t=42"), "{}", button.url);
    }
}

#[test]
fn a_paused_video_is_flagged_and_has_no_counters() {
    let mut plugin = start();
    let tick = tick(&mut plugin, json!([watch("A Song", 60.0, true)]));
    let shown = tick.activity.unwrap();
    assert!(tick.meta.paused);
    assert_eq!((shown.start, shown.end), (None, None));
    assert_eq!(shown.small_text.as_deref(), Some("Paused"));
}

#[test]
fn the_playing_tab_wins_over_a_paused_one() {
    let mut plugin = start();
    let mut other = watch("Paused one", 5.0, true);
    other["tab"] = json!(2);
    let shown = tick(
        &mut plugin,
        json!([other, watch("Playing one", 5.0, false)]),
    )
    .activity
    .unwrap();
    assert_eq!(shown.details.as_deref(), Some("Playing one"));
}

#[test]
fn without_pages_there_is_nothing_to_show() {
    let mut plugin = start();
    assert!(tick(&mut plugin, json!([])).activity.is_none());
}

#[test]
fn pages_without_a_player_show_browsing() {
    let mut plugin = start();
    let home = json!({
        "browser": "chrome", "tab": 1, "presence": "YouTube", "url": "https://www.youtube.com/",
        "title": "YouTube", "received_at": RECEIVED, "values": {"player": null}
    });
    let shown = tick(&mut plugin, json!([home])).activity.unwrap();
    assert_eq!(shown.details.as_deref(), Some("Browsing YouTube"));
    assert!(shown.buttons.is_empty());
}

#[test]
fn shorts_use_their_own_title_and_channel() {
    let mut plugin = start();
    let shorts = json!({
        "browser": "chrome", "tab": 1, "presence": "YouTube",
        "url": "https://www.youtube.com/shorts/abc123", "title": "Short - YouTube",
        "received_at": RECEIVED,
        "values": {"shorts_title": "Quick clip", "shorts_author": "Clipper",
                   "player": {"paused": false, "current": 1.0, "duration": 30.0, "rate": 1.0}}
    });
    let shown = tick(&mut plugin, json!([shorts])).activity.unwrap();
    assert_eq!(shown.details.as_deref(), Some("Quick clip"));
    assert_eq!(shown.state.as_deref(), Some("By Clipper"));
    assert_eq!(
        shown.buttons[0].url,
        "https://www.youtube.com/watch?v=abc123"
    );
}

#[test]
fn very_long_titles_are_shortened_to_fit_discord() {
    let mut plugin = start();
    let long = "x".repeat(300);
    let shown = tick(&mut plugin, json!([watch(&long, 1.0, false)]))
        .activity
        .unwrap();
    assert_eq!(shown.details.as_deref().unwrap().chars().count(), 128);
}

#[test]
fn texts_follow_the_language_and_buttons_are_translated() {
    let mut plugin = start_in("es");
    let shown = tick(&mut plugin, json!([watch("A Song", 60.0, false)]))
        .activity
        .unwrap();
    assert_eq!(shown.state.as_deref(), Some("De Some Channel"));
    assert_eq!(shown.small_text.as_deref(), Some("Reproduciendo"));
    assert_eq!(shown.buttons[0].label, "Ver en YouTube");
}

#[test]
fn an_ad_keeps_the_video_but_not_the_clock_of_the_ad() {
    let mut plugin = start_in("es");
    let mut page = watch("A Video", 3.0, false);
    page["values"]["ad"] = json!("movie_player");
    page["values"]["player"]["duration"] = json!(15.0);
    let tick = tick(&mut plugin, json!([page]));
    let shown = tick.activity.unwrap();
    assert_eq!(shown.details.as_deref(), Some("A Video"));
    assert_eq!((shown.start, shown.end), (None, None));
    assert_eq!(shown.small_text.as_deref(), Some("Anuncio en curso"));
    assert!(
        !tick.meta.paused,
        "an ad does not give way to a paused video elsewhere"
    );
}

#[test]
fn the_logo_can_replace_the_thumbnail() {
    let mut plugin = start();
    let shown = plugin
        .poll(&json!({"now": 1, "pages": [watch("A Song", 60.0, false)], "options": {"image": "logo"}}))
        .unwrap()
        .activity
        .unwrap();
    assert_eq!(shown.large_image.as_deref(), Some("youtube_logo"));
}
