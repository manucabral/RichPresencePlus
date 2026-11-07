//! Runs the real Roblox script against a simulated machine.

use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use rpp_plugins::{Host, Manifest, Plugin, Request};
use serde_json::{Value, json};

const LOGS: &str = "C:/Users/test/AppData/Local/Roblox/logs";

/// The machine the script sees; tests change it between polls.
#[derive(Default)]
struct Machine {
    processes: Vec<&'static str>,
    log: Vec<&'static str>,
    foreground: Option<&'static str>,
    games_ok: bool,
    requests: Vec<String>,
    log_searches: usize,
    /// `rootPlaceId` in the game details answer.
    place: Value,
}

struct Fake(Rc<RefCell<Machine>>);

impl Host for Fake {
    fn call(&self, request: Request) -> Result<Value, String> {
        let mut machine = self.0.borrow_mut();
        Ok(match request {
            Request::ProcessList => json!(machine.processes),
            Request::LatestFile { .. } => {
                machine.log_searches += 1;
                json!(format!("{LOGS}/session_last.log"))
            }
            Request::TailFile { .. } => json!(machine.log),
            Request::ForegroundWindow => match machine.foreground {
                Some(process) => json!({"process": process, "title": "Roblox"}),
                None => Value::Null,
            },
            Request::HttpGet { url } => {
                machine.requests.push(url.clone());
                if !machine.games_ok {
                    return Err("offline".into());
                }
                let body = if url.contains("thumbnails") {
                    json!({"data": [{"imageUrl": "https://t.rbxcdn.com/obby.png"}]})
                } else {
                    json!({"data": [{"name": "Obby World", "playing": 1200, "rootPlaceId": machine.place,
                                     "creator": {"name": "Builder"}}]})
                };
                json!({"status": 200, "body": body.to_string()})
            }
            Request::MediaSession | Request::ProcessArgs { .. } | Request::ServerQuery { .. } => {
                Value::Null
            }
        })
    }
}

fn start(machine: &Rc<RefCell<Machine>>) -> Plugin {
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../presences/roblox");
    let manifest = fs::read_to_string(folder.join("manifest.json")).unwrap();
    let mut manifest = Manifest::parse(&manifest).unwrap();
    manifest.permissions = manifest
        .permissions
        .resolve(|name| (name == "LOCALAPPDATA").then(|| "C:/Users/test/AppData/Local".into()));
    let source = fs::read_to_string(folder.join(manifest.entry.clone())).unwrap();
    let mut plugin = Plugin::load(manifest, &source, Rc::new(Fake(machine.clone()))).unwrap();
    let english = fs::read_to_string(folder.join("locales/en.json")).unwrap();
    plugin
        .set_strings(&serde_json::from_str(&english).unwrap())
        .unwrap();
    plugin
}

fn machine() -> Rc<RefCell<Machine>> {
    Rc::new(RefCell::new(Machine {
        processes: vec!["explorer.exe", "RobloxPlayerBeta.exe"],
        games_ok: true,
        place: json!(777),
        ..Machine::default()
    }))
}

#[test]
fn the_manifest_is_valid() {
    start(&machine());
}

#[test]
fn nothing_is_shown_when_the_player_is_closed() {
    let machine = machine();
    machine.borrow_mut().processes = vec!["explorer.exe"];
    let mut plugin = start(&machine);
    assert_eq!(plugin.tick(&json!({})).unwrap(), None);
}

#[test]
fn the_menu_is_shown_with_its_start_time() {
    let machine = machine();
    machine.borrow_mut().log = vec!["noise", "x returnToLuaAppInternal y"];
    let mut plugin = start(&machine);
    let shown = plugin.tick(&json!({})).unwrap().unwrap();
    assert_eq!(shown.details.as_deref(), Some("In menu"));
    assert!(shown.start.is_some());
}

#[test]
fn a_game_shows_its_name_creator_and_icon_and_is_requested_once() {
    let machine = machine();
    machine.borrow_mut().log = vec!["game_join_loadtime universeid:4242 more"];
    let mut plugin = start(&machine);
    let shown = plugin.tick(&json!({})).unwrap().unwrap();
    assert_eq!(shown.details.as_deref(), Some("Obby World"));
    assert_eq!(shown.state.as_deref(), Some("By Builder"));
    assert_eq!(shown.small_text.as_deref(), Some("1200 playing"));
    assert_eq!(
        shown.details_url.as_deref(),
        Some("https://www.roblox.com/games/777")
    );
    assert_eq!(
        shown.large_image.as_deref(),
        Some("https://t.rbxcdn.com/obby.png")
    );

    plugin.tick(&json!({})).unwrap();
    let requests = machine.borrow().requests.clone();
    assert_eq!(requests.len(), 2, "{requests:?}");
    assert!(requests[0].contains("universeIds=4242"));
}

#[test]
fn leaving_the_game_clears_the_activity() {
    let machine = machine();
    machine.borrow_mut().log = vec!["game_join_loadtime universeid:4242"];
    let mut plugin = start(&machine);
    assert!(plugin.tick(&json!({})).unwrap().is_some());
    machine.borrow_mut().log = vec!["shutDown: (stage:UGCGame)"];
    assert_eq!(plugin.tick(&json!({})).unwrap(), None);
}

#[test]
fn a_failing_api_falls_back_and_is_not_retried_at_once() {
    let machine = machine();
    {
        let mut state = machine.borrow_mut();
        state.log = vec!["game_join_loadtime universeid:7"];
        state.games_ok = false;
    }
    let mut plugin = start(&machine);
    let shown = plugin.tick(&json!({})).unwrap().unwrap();
    assert_eq!(shown.state.as_deref(), Some("Playing"));
    assert_eq!(shown.details, None);
    let after_first = machine.borrow().requests.len();
    plugin.tick(&json!({})).unwrap();
    assert_eq!(machine.borrow().requests.len(), after_first);
}

#[test]
fn foreground_is_reported_for_the_ranking() {
    let machine = machine();
    machine.borrow_mut().log = vec!["returnToLuaAppInternal"];
    machine.borrow_mut().foreground = Some("RobloxPlayerBeta.exe");
    let mut plugin = start(&machine);
    let tick = plugin.poll(&json!({})).unwrap();
    assert!(tick.meta.foreground);
    assert_eq!(tick.meta.application, "RobloxPlayerBeta.exe");

    machine.borrow_mut().foreground = Some("chrome.exe");
    assert!(!plugin.poll(&json!({})).unwrap().meta.foreground);
}

#[test]
fn an_old_transition_in_the_log_does_not_restart_the_clock() {
    let machine = machine();
    // The window holds a past game and the return to the menu after it.
    machine.borrow_mut().log = vec![
        "game_join_loadtime universeid:4242",
        "shutDown: (stage:UGCGame)",
        "returnToLuaAppInternal",
    ];
    let mut plugin = start(&machine);
    let first = plugin.tick(&json!({})).unwrap().unwrap();
    assert_eq!(first.details.as_deref(), Some("In menu"));
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let second = plugin.tick(&json!({})).unwrap().unwrap();
    assert_eq!(second.start, first.start);
    // Only the last state matters, so the past game was never looked up.
    assert!(machine.borrow().requests.is_empty());
}

#[test]
fn the_log_folder_is_searched_once_and_again_after_the_player_restarts() {
    let machine = machine();
    machine.borrow_mut().log = vec!["returnToLuaAppInternal"];
    let mut plugin = start(&machine);
    for _ in 0..3 {
        plugin.tick(&json!({})).unwrap();
    }
    assert_eq!(machine.borrow().log_searches, 1);

    machine.borrow_mut().processes = vec!["explorer.exe"];
    plugin.tick(&json!({})).unwrap();
    machine.borrow_mut().processes = vec!["RobloxPlayerBeta.exe"];
    plugin.tick(&json!({})).unwrap();
    assert_eq!(machine.borrow().log_searches, 2);
}

#[test]
fn the_player_count_can_be_hidden() {
    let machine = machine();
    machine.borrow_mut().log = vec!["game_join_loadtime universeid:4242"];
    let mut plugin = start(&machine);
    let shown = plugin
        .tick(&json!({"options": {"show_players": false}}))
        .unwrap()
        .unwrap();
    assert_eq!(shown.details.as_deref(), Some("Obby World"));
    assert_eq!(shown.small_text, None);
}

#[test]
fn the_game_link_needs_a_whole_place_number() {
    for place in [
        json!(null),
        json!(0),
        json!(-5),
        json!(1.5),
        json!("777/../../evil"),
        json!(9007199254740993_u64),
    ] {
        let machine = machine();
        machine.borrow_mut().log = vec!["game_join_loadtime universeid:4242 more"];
        machine.borrow_mut().place = place.clone();
        let mut plugin = start(&machine);
        let shown = plugin.tick(&json!({})).unwrap().unwrap();
        assert_eq!(shown.details.as_deref(), Some("Obby World"), "{place}");
        assert_eq!(shown.details_url, None, "{place}");
    }
}
