//! The examples of the developer documentation, run as written there.
//!
//! The site teaches with these scripts, so they have to work.

use std::rc::Rc;

use rpp_plugins::{Host, Manifest, Plugin, Request};
use serde_json::{Value, json};

/// The video presence that `docs/developers/en/02-first-presence.md` builds, as its last block.
const WEB: &str = r#"
function read(context) {
  const page = bestPage(context.pages);
  if (!page) return null;

  const values = page.values;
  return {
    title: values.title || page.title,
    channel: values.channel,
    channel_url: values.channel_url,
    url: page.url,
    ...playback(values.player, page.received_at),
  };
}

function activity(video) {
  const buttons = [{ label: t("watch"), url: video.url }];
  if (video.channel_url) buttons.push({ label: t("channel"), url: video.channel_url });

  return {
    name: "My Video",
    details: cut(video.title),
    state: video.channel || t("watching"),
    kind: "watching",
    start: video.start,
    end: video.end,
    buttons,
  };
}

function meta(video) {
  return { paused: !video.playing };
}
"#;

/// The game presence that `docs/developers/en/04-desktop.md` builds, all steps together.
const DESKTOP: &str = r#"
const PROGRAM = "game.exe";
let since = 0;

function read(context) {
  const running = host.process.list().some((name) => name.toLowerCase() === PROGRAM);
  if (!running) {
    since = 0;
    return null;
  }
  if (since === 0) since = context.now;
  const window = host.window.foreground();
  const log = host.files.latest(host.files.roots[0], ".log");
  const lines = host.files.tail(log, 50);
  const level = lines.map((line) => /Entered level (.+)/.exec(line)).findLast(Boolean);
  return {
    since,
    focused: Boolean(window) && window.process.toLowerCase() === PROGRAM,
    level: level ? level[1] : null,
  };
}

function activity(game) {
  return {
    details: "My Game",
    state: game.level || t("playing"),
    kind: "playing",
    start: game.since,
    large_image: "logo",
  };
}

function meta(game) {
  return { application: PROGRAM, foreground: game.focused };
}
"#;

struct Fake;

impl Host for Fake {
    fn call(&self, request: Request) -> Result<Value, String> {
        match request {
            Request::ProcessList => Ok(json!(["explorer.exe", "Game.exe"])),
            Request::ForegroundWindow => Ok(json!({"process": "game.exe", "title": "My Game"})),
            Request::LatestFile { .. } => Ok(json!("C:/logs/game.log")),
            Request::TailFile { .. } => Ok(json!([
                "Entered level Docks",
                "Saved",
                "Entered level Harbor",
                "Saved"
            ])),
            other => Err(format!("not simulated: {other:?}")),
        }
    }
}

fn plugin(manifest: Value, source: &str) -> Plugin {
    let manifest = Manifest::parse(&manifest.to_string()).unwrap();
    Plugin::load(manifest, source, Rc::new(Fake)).unwrap()
}

/// The last JavaScript block of a page, where the tutorial shows the whole file.
fn last_script(page: &str) -> String {
    let page = page.replace("\r\n", "\n");
    let start = page.rfind("```js\n").expect("a js block") + "```js\n".len();
    let end = start + page[start..].find("```").expect("the end of the block");
    page[start..end].trim().to_owned()
}

#[test]
fn the_tutorial_shows_the_code_this_file_runs() {
    let english = include_str!("../../../docs/developers/en/02-first-presence.md");
    let spanish = include_str!("../../../docs/developers/es/02-first-presence.md");
    assert_eq!(last_script(english), WEB.trim());
    assert_eq!(
        last_script(spanish),
        WEB.trim().replace("My Video", "Mi Video"),
        "the Spanish page differs only in the name"
    );
}

#[test]
fn the_web_example_works_as_the_page_explains() {
    let manifest = json!({
        "name": "Video", "version": "1.0.0", "client_id": "12345678901234567",
        "page": {"matches": ["*://manucabral.github.io/*practice/*"], "extract": {
            "title": {"from": "text", "selector": "h1.video-title"},
            "channel": {"from": "text", "selector": "a.channel-name"},
            "channel_url": {"from": "attr", "selector": "a.channel-name", "name": "href"},
            "player": {"from": "media", "selector": "video"}
        }}
    });
    let mut plugin = plugin(manifest, WEB);
    let page = |paused: bool| {
        json!({"now": 1000, "pages": [{
            "url": "https://www.example.com/watch?v=1", "title": "Tab", "received_at": 1_000_000,
            "values": {
                "title": "A video", "channel": "A channel", "channel_url": "https://www.example.com/c",
                "player": {"paused": paused, "current": 10.0, "duration": 100.0, "rate": 1.0}
            }
        }]})
    };

    let playing = plugin.poll(&page(false)).unwrap();
    let shown = playing.activity.unwrap();
    assert_eq!(shown.name.as_deref(), Some("My Video"));
    assert_eq!(shown.details.as_deref(), Some("A video"));
    assert_eq!(shown.state.as_deref(), Some("A channel"));
    assert_eq!((shown.start, shown.end), (Some(990), Some(1090)));
    assert_eq!(shown.buttons.len(), 2);
    assert!(!playing.meta.paused);

    let paused = plugin.poll(&page(true)).unwrap();
    let shown = paused.activity.unwrap();
    assert_eq!(
        (shown.start, shown.end),
        (None, None),
        "the bar goes by itself"
    );
    assert!(paused.meta.paused);

    assert!(
        plugin
            .poll(&json!({"now": 1, "pages": []}))
            .unwrap()
            .activity
            .is_none()
    );
}

#[test]
fn the_desktop_example_works_as_the_page_explains() {
    let manifest = json!({
        "name": "My Game", "version": "1.0.0", "client_id": "12345678901234567",
        "permissions": {"process": true, "window": true, "files": ["C:/logs"]}
    });
    let mut plugin = plugin(manifest, DESKTOP);
    let first = plugin.poll(&json!({"now": 1000, "pages": []})).unwrap();
    let shown = first.activity.unwrap();
    assert_eq!(
        shown.state.as_deref(),
        Some("Harbor"),
        "the last level of the log"
    );
    assert_eq!(shown.start, Some(1000));
    assert!(first.meta.foreground);
    assert_eq!(first.meta.application, "game.exe");

    let later = plugin.poll(&json!({"now": 1050, "pages": []})).unwrap();
    assert_eq!(
        later.activity.unwrap().start,
        Some(1000),
        "the counter keeps its start"
    );
}

#[test]
fn the_time_helpers_read_what_players_show() {
    let source = r#"
      function read() { return {}; }
      function activity() {
        const position = parseTime("1:02:03");
        const bar = progress(parseTime("4:05"), parseTime("10:00"), 1000);
        return {
          details: `${position} ${parseTime("30")} ${parseTime("not a time")} ${parseTime(5)}`,
          start: bar.start,
          end: bar.end,
        };
      }
    "#;
    let manifest = json!({"name": "Clock", "version": "1.0.0", "client_id": "12345678901234567"});
    let shown = plugin(manifest, source)
        .poll(&json!({"now": 1, "pages": []}))
        .unwrap()
        .activity
        .unwrap();
    assert_eq!(shown.details.as_deref(), Some("3723 30 null null"));
    assert_eq!((shown.start, shown.end), (Some(755), Some(1355)));
}
