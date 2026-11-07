//! The bundled web presences, fed with pages as the extension reports them.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use rpp_plugins::{Host, Manifest, Plugin, Request, Tick};
use serde_json::{Value, json};

struct Silent;

impl Host for Silent {
    fn call(&self, _: Request) -> Result<Value, String> {
        Err("no host".into())
    }
}

fn folder(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../presences")
        .join(name)
}

fn texts(name: &str, tag: &str) -> BTreeMap<String, String> {
    let file = folder(name).join("locales").join(format!("{tag}.json"));
    serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap()
}

fn start(name: &str, tag: &str) -> Plugin {
    let root = folder(name);
    let manifest =
        Manifest::parse(&fs::read_to_string(root.join("manifest.json")).unwrap()).unwrap();
    let source = fs::read_to_string(root.join(&manifest.entry)).unwrap();
    let mut plugin = Plugin::load(manifest, &source, Rc::new(Silent)).unwrap();
    let mut strings = texts(name, "en");
    strings.extend(texts(name, tag));
    plugin.set_strings(&strings).unwrap();
    plugin
}

fn tick(plugin: &mut Plugin, pages: Value) -> Tick {
    plugin
        .poll(&json!({"now": 1_800_000_100, "pages": pages}))
        .unwrap()
}

/// Every presence folder has the same texts in every language it ships.
#[test]
fn every_presence_folder_loads() {
    for name in ["youtube-music", "netflix", "nepu", "dailymotion"] {
        let manifest =
            Manifest::parse(&fs::read_to_string(folder(name).join("manifest.json")).unwrap())
                .unwrap();
        assert!(manifest.page.is_some(), "{name} reads pages");
        start(name, "es");
    }
}

const RECEIVED: u64 = 1_800_000_000_000;

fn song(state: &str, current: f64) -> Value {
    json!({
        "browser": "chrome", "tab": 3, "presence": "YouTube Music",
        "url": "https://music.youtube.com/watch?v=abc",
        "title": "A Song - YouTube Music", "received_at": RECEIVED,
        "values": {
            "session": {
                "title": "A Song", "artist": "The Band", "album": "The Album",
                "artwork": "https://lh3.googleusercontent.com/cover=w512", "state": state
            },
            "player": {"paused": state != "playing", "current": current, "duration": 200.0, "rate": 1.0}
        }
    })
}

#[test]
fn a_playing_song_shows_artist_album_cover_and_a_progress_bar() {
    let mut plugin = start("youtube-music", "en");
    let tick = tick(&mut plugin, json!([song("playing", 50.0)]));
    let shown = tick.activity.unwrap();
    assert_eq!(shown.details.as_deref(), Some("A Song"));
    assert_eq!(shown.state.as_deref(), Some("The Band | The Album"));
    assert_eq!(
        shown.large_image.as_deref(),
        Some("https://lh3.googleusercontent.com/cover=w512")
    );
    // Position 50 s at the moment of the report: the song began 50 s earlier.
    assert_eq!(shown.start, Some(1_800_000_000 - 50));
    assert_eq!(shown.end, Some(1_800_000_000 - 50 + 200));
    assert_eq!(shown.small_text.as_deref(), Some("Playing"));
    assert!(!tick.meta.paused);
}

#[test]
fn a_paused_song_has_no_clock_and_is_marked_paused() {
    let mut plugin = start("youtube-music", "en");
    let tick = tick(&mut plugin, json!([song("paused", 50.0)]));
    let shown = tick.activity.unwrap();
    assert_eq!((shown.start, shown.end), (None, None));
    assert_eq!(shown.small_text.as_deref(), Some("Paused"));
    assert!(tick.meta.paused);
}

#[test]
fn nothing_playing_means_no_activity_and_a_long_cover_address_is_replaced() {
    let mut plugin = start("youtube-music", "en");
    assert!(tick(&mut plugin, json!([])).activity.is_none());
    let mut idle = song("playing", 1.0);
    idle["values"]["session"] = Value::Null;
    assert!(tick(&mut plugin, json!([idle])).activity.is_none());

    let mut long = song("playing", 1.0);
    long["values"]["session"]["artwork"] = json!(format!("https://a.io/{}", "x".repeat(600)));
    let shown = tick(&mut plugin, json!([long])).activity.unwrap();
    assert_eq!(shown.large_image.as_deref(), Some("youtube_music_logo"));
}

#[test]
fn long_titles_are_cut_instead_of_breaking_the_activity() {
    let mut plugin = start("youtube-music", "es");
    let mut page = song("playing", 1.0);
    page["values"]["session"]["title"] = json!("t".repeat(300));
    page["values"]["session"]["artist"] = json!("a".repeat(300));
    let shown = tick(&mut plugin, json!([page])).activity.unwrap();
    assert!(shown.details.unwrap().chars().count() <= 128);
    assert!(shown.state.unwrap().chars().count() <= 128);
}

#[test]
fn the_playing_tab_wins_over_a_paused_one() {
    let mut plugin = start("youtube-music", "en");
    let mut other = song("paused", 9.0);
    other["tab"] = json!(4);
    other["values"]["session"]["title"] = json!("Other Song");
    let shown = tick(&mut plugin, json!([other, song("playing", 5.0)]))
        .activity
        .unwrap();
    assert_eq!(shown.details.as_deref(), Some("A Song"));
}

fn show(state: &str, current: f64, path: &str) -> Value {
    json!({
        "browser": "chrome", "tab": 1, "presence": "Netflix",
        "url": format!("https://www.netflix.com{path}"),
        "title": "Netflix", "received_at": RECEIVED,
        "values": {
            "title": "A Show", "episode": "E3", "episode_title": "The Third",
            "player": {"paused": state != "playing", "current": current, "duration": 3000.0, "rate": 1.0}
        }
    })
}

#[test]
fn a_netflix_episode_shows_title_episode_and_a_watch_button() {
    let mut plugin = start("netflix", "en");
    let shown = tick(
        &mut plugin,
        json!([show("playing", 100.0, "/watch/80100172?trackId=1")]),
    )
    .activity
    .unwrap();
    assert_eq!(shown.details.as_deref(), Some("A Show"));
    assert_eq!(shown.state.as_deref(), Some("E3 - The Third"));
    assert_eq!(shown.start, Some(1_800_000_000 - 100));
    assert_eq!(shown.buttons.len(), 1);
    assert_eq!(shown.buttons[0].label, "Watch on Netflix");
    // The query string, which can carry tracking parameters, is not shared.
    assert_eq!(
        shown.buttons[0].url,
        "https://www.netflix.com/watch/80100172"
    );
}

#[test]
fn netflix_menus_are_browsing_and_translate() {
    let mut plugin = start("netflix", "es");
    let shown = tick(&mut plugin, json!([show("paused", 0.0, "/browse")]))
        .activity
        .unwrap();
    assert_eq!(shown.details.as_deref(), Some("Explorando"));
    assert!(shown.buttons.is_empty());
}

fn nepu(path: &str, values: Value) -> Value {
    json!({
        "browser": "chrome", "tab": 1, "presence": "Nepu",
        "url": format!("https://nepu.to{path}"),
        "title": "Nepu", "received_at": RECEIVED, "values": values
    })
}

#[test]
fn a_nepu_movie_shows_title_aka_and_the_cover() {
    let mut plugin = start("nepu", "en");
    let page = nepu(
        "/movie/42-a-film",
        json!({
            "title": "A Film (2021)", "heading": "Another Name",
            "cover": "background-image: url(\"https://img.nepu.to/c.jpg\");"
        }),
    );
    let shown = tick(&mut plugin, json!([page])).activity.unwrap();
    assert_eq!(shown.details.as_deref(), Some("A Film (2021)"));
    assert_eq!(shown.state.as_deref(), Some("Another Name"));
    assert_eq!(
        shown.large_image.as_deref(),
        Some("https://img.nepu.to/c.jpg")
    );
    assert_eq!(shown.small_image.as_deref(), Some("logo"));
}

#[test]
fn a_nepu_episode_prefers_the_episode_line_and_unsafe_covers_are_dropped() {
    let mut plugin = start("nepu", "en");
    let page = nepu(
        "/season/9/episode/2",
        json!({
            "title": "A Series", "heading": "Episode Name", "episode": "S1 E2",
            "cover": "background-image: url(javascript:alert(1))"
        }),
    );
    let shown = tick(&mut plugin, json!([page])).activity.unwrap();
    assert_eq!(shown.state.as_deref(), Some("S1 E2"));
    assert_eq!(shown.large_image.as_deref(), Some("logo"));
    assert_eq!(shown.small_image, None);
}

#[test]
fn nepu_pages_without_content_are_browsing() {
    let mut plugin = start("nepu", "es");
    let shown = tick(&mut plugin, json!([nepu("/", json!({"title": null}))]))
        .activity
        .unwrap();
    assert_eq!(shown.details.as_deref(), Some("Navegando en Nepu"));
    assert!(tick(&mut plugin, json!([])).activity.is_none());
}

#[test]
fn the_album_can_be_hidden() {
    let mut plugin = start("youtube-music", "en");
    let shown = plugin
        .poll(&json!({"now": 1, "pages": [song("playing", 1.0)], "options": {"show_album": false}}))
        .unwrap()
        .activity
        .unwrap();
    assert_eq!(shown.state.as_deref(), Some("The Band"));
}

fn media_page(values: Value) -> Value {
    json!([{
        "browser": "chrome", "tab": 1, "presence": "Web Media",
        "url": "https://www.example.com/watch/1", "title": "A title", "values": values,
        "received_at": 1_000_000_u64
    }])
}

#[test]
fn web_media_shows_a_session_as_music_and_reports_the_site() {
    let mut plugin = start("web-media", "en");
    let values = json!({"session": {"title": "Song", "artist": "Band", "state": "playing"}});
    let result = tick(&mut plugin, media_page(values));
    let shown = result.activity.unwrap();
    assert_eq!(shown.details.as_deref(), Some("Song"));
    assert_eq!(shown.state.as_deref(), Some("Band"));
    assert_eq!(result.meta.domains, ["example.com"]);
    assert!(!result.meta.paused);
}

#[test]
fn web_media_falls_back_to_the_page_title_and_the_site_name() {
    let mut plugin = start("web-media", "en");
    let player = json!({"paused": false, "current": 5.0, "duration": 60.0, "rate": 1.0});
    let result = tick(
        &mut plugin,
        media_page(json!({"player": player, "site": "Example"})),
    );
    let shown = result.activity.unwrap();
    assert_eq!(shown.details.as_deref(), Some("A title"));
    assert_eq!(shown.state.as_deref(), Some("Example"));
}

#[test]
fn web_media_needs_something_playing() {
    let mut plugin = start("web-media", "en");
    let paused = json!({"paused": true, "current": 5.0, "duration": 60.0, "rate": 1.0});
    assert!(
        tick(&mut plugin, media_page(json!({"player": paused})))
            .activity
            .is_none()
    );
    assert!(tick(&mut plugin, media_page(json!({}))).activity.is_none());
}

#[test]
fn web_media_trusts_the_player_when_a_site_leaves_its_state_unset() {
    let mut plugin = start("web-media", "en");
    let session = json!({"title": "A video", "state": "none"});
    let running = json!({"paused": false, "current": 5.0, "duration": 60.0, "rate": 1.0});
    let stopped = json!({"paused": true, "current": 5.0, "duration": 60.0, "rate": 1.0});

    let values = json!({"session": session, "player": running});
    assert!(!tick(&mut plugin, media_page(values)).meta.paused);
    let values = json!({"session": session, "player": stopped});
    assert!(tick(&mut plugin, media_page(values)).meta.paused);
    let values = json!({"session": session});
    assert!(!tick(&mut plugin, media_page(values)).meta.paused);
}

#[test]
fn web_media_believes_a_site_that_says_it_paused() {
    let mut plugin = start("web-media", "en");
    let running = json!({"paused": false, "current": 5.0, "duration": 60.0, "rate": 1.0});
    let values = json!({"session": {"title": "A video", "state": "paused"}, "player": running});
    assert!(tick(&mut plugin, media_page(values)).meta.paused);
}

/// A Windows media session, as the host reports it.
struct Playing(Value);

impl Host for Playing {
    fn call(&self, _: Request) -> Result<Value, String> {
        Ok(self.0.clone())
    }
}

fn desktop_media(session: Value) -> Tick {
    let root = folder("desktop-media");
    let manifest =
        Manifest::parse(&fs::read_to_string(root.join("manifest.json")).unwrap()).unwrap();
    let source = fs::read_to_string(root.join(&manifest.entry)).unwrap();
    let mut plugin = Plugin::load(manifest, &source, Rc::new(Playing(session))).unwrap();
    plugin.set_strings(&texts("desktop-media", "en")).unwrap();
    plugin
        .poll(&json!({"now": 1000, "pages": [], "options": {}}))
        .unwrap()
}

#[test]
fn desktop_media_shows_a_song_from_a_program() {
    let tick = desktop_media(json!({
        "app": "Spotify.exe", "title": "Song", "artist": "Band", "album": "Album",
        "status": "playing", "position": 30.0, "duration": 200.0
    }));
    let shown = tick.activity.unwrap();
    assert_eq!(shown.details.as_deref(), Some("Song"));
    assert_eq!(shown.state.as_deref(), Some("Band - Album"));
    assert_eq!((shown.start, shown.end), (Some(970), Some(1170)));
    assert_eq!(shown.large_text.as_deref(), Some("Spotify"));
    assert_eq!(tick.meta.application, "Spotify.exe");
    assert!(!tick.meta.paused);
}

#[test]
fn desktop_media_has_no_clock_while_paused() {
    let tick = desktop_media(json!({
        "app": "vlc.exe", "title": "Film", "artist": "", "album": "",
        "status": "paused", "position": 30.0, "duration": 200.0
    }));
    assert!(tick.meta.paused);
    let shown = tick.activity.unwrap();
    assert_eq!((shown.start, shown.end), (None, None));
    assert_eq!(shown.state, None);
}

#[test]
fn desktop_media_leaves_browsers_to_the_extension() {
    for app in ["chrome.exe", "MSEdge", "308046B0AF4A39CB", "Brave.exe"] {
        let tick = desktop_media(json!({
            "app": app, "title": "A video", "artist": "", "album": "",
            "status": "playing", "position": 1.0, "duration": 9.0
        }));
        assert!(tick.activity.is_none(), "{app}");
    }
}

#[test]
fn desktop_media_needs_a_session_with_a_title() {
    assert!(desktop_media(Value::Null).activity.is_none());
    assert!(
        desktop_media(json!({"app": "x.exe", "title": ""}))
            .activity
            .is_none()
    );
}

/// A Dailymotion page as the extension sends it: the title and channel come from
/// the page, `player` from the frame of geo.dailymotion.com.
fn dailymotion(path: &str, values: Value) -> Value {
    json!({
        "browser": "chrome", "tab": 4, "presence": "Dailymotion",
        "url": format!("https://www.dailymotion.com{path}"),
        "title": "A Video - Dailymotion", "received_at": RECEIVED, "values": values
    })
}

fn dailymotion_player(paused: bool) -> Value {
    json!({"paused": paused, "current": 30.0, "duration": 280.0, "rate": 1.0})
}

#[test]
fn a_dailymotion_video_shows_title_channel_thumbnail_and_buttons() {
    let mut plugin = start("dailymotion", "en");
    let page = dailymotion(
        "/video/xbtlhty",
        json!({
            "title": "A Video", "channel": "A Channel",
            "channel_url": "https://www.dailymotion.com/user/achannel",
            "player": dailymotion_player(false)
        }),
    );
    let result = tick(&mut plugin, json!([page]));
    let shown = result.activity.unwrap();
    assert_eq!(shown.name.as_deref(), Some("Dailymotion"));
    assert_eq!(shown.details.as_deref(), Some("A Video"));
    assert_eq!(shown.state.as_deref(), Some("By A Channel"));
    assert_eq!(shown.start, Some(1_800_000_000 - 30));
    assert_eq!(shown.end, Some(1_800_000_000 - 30 + 280));
    assert_eq!(
        shown.large_image.as_deref(),
        Some("https://www.dailymotion.com/thumbnail/video/xbtlhty")
    );
    assert_eq!(shown.buttons.len(), 2);
    assert_eq!(
        shown.buttons[0].url,
        "https://www.dailymotion.com/video/xbtlhty"
    );
    assert_eq!(
        shown.buttons[1].url,
        "https://www.dailymotion.com/user/achannel"
    );
    assert!(!result.meta.paused);
}

#[test]
fn an_old_dailymotion_address_keeps_only_the_video_id() {
    let mut plugin = start("dailymotion", "en");
    let page = dailymotion(
        "/video/x7tgad0_a-long-slug?playlist=x6hynp",
        json!({"title": "A Video", "player": dailymotion_player(false)}),
    );
    let shown = tick(&mut plugin, json!([page])).activity.unwrap();
    assert_eq!(
        shown.buttons[0].url,
        "https://www.dailymotion.com/video/x7tgad0"
    );
    // Without a channel there is no second line and no channel button.
    assert_eq!(shown.state, None);
    assert_eq!(shown.buttons.len(), 1);
}

#[test]
fn a_paused_dailymotion_video_has_no_clock_and_translates() {
    let mut plugin = start("dailymotion", "es");
    let page = dailymotion(
        "/video/xbtlhty",
        json!({"title": "Un video", "channel": "Un canal", "player": dailymotion_player(true)}),
    );
    let result = tick(&mut plugin, json!([page]));
    let shown = result.activity.unwrap();
    assert_eq!((shown.start, shown.end), (None, None));
    assert_eq!(shown.state.as_deref(), Some("De Un canal"));
    assert_eq!(shown.small_text.as_deref(), Some("En pausa"));
    assert!(result.meta.paused);
}

#[test]
fn dailymotion_is_browsing_until_the_frame_reports_a_player() {
    let mut plugin = start("dailymotion", "es");
    // The page arrived before its frame: a video page with no player yet.
    let early = dailymotion("/video/xbtlhty", json!({"title": "Un video"}));
    let result = tick(&mut plugin, json!([early]));
    let shown = result.activity.unwrap();
    assert_eq!(shown.details.as_deref(), Some("Navegando en Dailymotion"));
    assert!(shown.buttons.is_empty());
    assert!(!result.meta.paused);

    // A player outside a video page, such as a preview on the home page.
    let home = dailymotion("/", json!({"player": dailymotion_player(false)}));
    let shown = tick(&mut plugin, json!([home])).activity.unwrap();
    assert_eq!(shown.details.as_deref(), Some("Navegando en Dailymotion"));
    assert_eq!(shown.large_image, None);
}
