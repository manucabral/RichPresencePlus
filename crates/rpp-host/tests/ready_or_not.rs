//! The Ready Or Not script against a simulated list of running programs.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use rpp_plugins::{Host, Manifest, Plugin, Request, Tick};
use serde_json::{Value, json};

struct Fake(Rc<RefCell<Vec<&'static str>>>);

impl Host for Fake {
    fn call(&self, request: Request) -> Result<Value, String> {
        match request {
            Request::ProcessList => Ok(json!(self.0.borrow().clone())),
            other => Err(format!("not simulated: {other:?}")),
        }
    }
}

fn start(processes: &Rc<RefCell<Vec<&'static str>>>, tag: &str) -> Plugin {
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../presences/ready-or-not");
    let manifest =
        Manifest::parse(&fs::read_to_string(folder.join("manifest.json")).unwrap()).unwrap();
    let source = fs::read_to_string(folder.join(&manifest.entry)).unwrap();
    let mut plugin = Plugin::load(manifest, &source, Rc::new(Fake(processes.clone()))).unwrap();
    let read = |tag: &str| -> BTreeMap<String, String> {
        let file = folder.join("locales").join(format!("{tag}.json"));
        serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap()
    };
    let mut strings = read("en");
    strings.extend(read(tag));
    plugin.set_strings(&strings).unwrap();
    plugin
}

fn tick(plugin: &mut Plugin) -> Tick {
    plugin.poll(&json!({"now": 1, "pages": []})).unwrap()
}

fn machine(names: &[&'static str]) -> Rc<RefCell<Vec<&'static str>>> {
    Rc::new(RefCell::new(names.to_vec()))
}

#[test]
fn the_game_running_shows_an_activity_with_the_time_since_it_was_seen() {
    let processes = machine(&["explorer.exe", "ReadyOrNot-Win64-Shipping.exe"]);
    let mut plugin = start(&processes, "en");
    let first = tick(&mut plugin);
    let shown = first.activity.unwrap();
    assert_eq!(shown.details.as_deref(), Some("Ready Or Not"));
    assert_eq!(shown.state.as_deref(), Some("Playing"));
    let started = shown.start.unwrap();
    assert!(started > 1_700_000_000);
    assert_eq!(first.meta.application, "readyornot-win64-shipping.exe");
    // The counter keeps its start while the game stays open.
    assert_eq!(tick(&mut plugin).activity.unwrap().start, Some(started));
}

#[test]
fn the_launcher_stub_counts_and_the_name_case_does_not_matter() {
    let processes = machine(&["READYORNOT.EXE"]);
    assert!(tick(&mut start(&processes, "en")).activity.is_some());
}

#[test]
fn without_the_game_there_is_nothing_to_show_and_the_clock_restarts() {
    let processes = machine(&["explorer.exe", "ReadyOrNotLauncherHelper.exe"]);
    let mut plugin = start(&processes, "en");
    assert!(tick(&mut plugin).activity.is_none());

    processes.borrow_mut().push("ReadyOrNot-Win64-Shipping.exe");
    let first = tick(&mut plugin).activity.unwrap().start;
    processes.borrow_mut().pop();
    assert!(tick(&mut plugin).activity.is_none());
    processes.borrow_mut().push("ReadyOrNot-Win64-Shipping.exe");
    assert!(tick(&mut plugin).activity.unwrap().start >= first);
}

#[test]
fn the_text_follows_the_language() {
    let processes = machine(&["ReadyOrNot-Win64-Shipping.exe"]);
    let shown = tick(&mut start(&processes, "es")).activity.unwrap();
    assert_eq!(shown.state.as_deref(), Some("Jugando"));
}
