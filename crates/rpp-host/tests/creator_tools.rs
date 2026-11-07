//! The Creator Tools script against a simulated foreground window.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use rpp_plugins::{Host, Manifest, Plugin, Request, Tick};
use serde_json::{Value, json};

type Window = Rc<RefCell<Value>>;

struct Fake(Window);

impl Host for Fake {
    fn call(&self, request: Request) -> Result<Value, String> {
        match request {
            Request::ForegroundWindow => Ok(self.0.borrow().clone()),
            other => Err(format!("not simulated: {other:?}")),
        }
    }
}

fn start(window: &Window, tag: &str) -> Plugin {
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../presences/creator-tools");
    let manifest =
        Manifest::parse(&fs::read_to_string(folder.join("manifest.json")).unwrap()).unwrap();
    let source = fs::read_to_string(folder.join(&manifest.entry)).unwrap();
    let mut plugin = Plugin::load(manifest, &source, Rc::new(Fake(window.clone()))).unwrap();
    let file = folder.join("locales").join(format!("{tag}.json"));
    let strings: BTreeMap<String, String> =
        serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap();
    plugin.set_strings(&strings).unwrap();
    plugin
}

fn tick(plugin: &mut Plugin, now: u64) -> Tick {
    plugin.poll(&json!({"now": now, "pages": []})).unwrap()
}

fn window(process: &str) -> Value {
    json!({"process": process, "title": "secret-project - private.txt"})
}

#[test]
fn the_tool_in_front_shows_with_its_kind_of_work_and_never_its_title() {
    let shared = Rc::new(RefCell::new(window("Code.exe")));
    let mut plugin = start(&shared, "es");
    let first = tick(&mut plugin, 1000);
    let shown = first.activity.unwrap();
    assert_eq!(shown.name.as_deref(), Some("Visual Studio Code"));
    assert_eq!(shown.details.as_deref(), Some("Escribiendo código"));
    assert_eq!(shown.start, Some(1000));
    assert_eq!(first.meta.application, "code.exe");
    assert!(!format!("{shown:?}").contains("secret-project"));
}

#[test]
fn a_look_elsewhere_keeps_it_and_a_long_absence_ends_it() {
    let shared = Rc::new(RefCell::new(window("blender.exe")));
    let mut plugin = start(&shared, "en");
    tick(&mut plugin, 1000);

    *shared.borrow_mut() = window("chrome.exe");
    let kept = tick(&mut plugin, 1100).activity.unwrap();
    assert_eq!(kept.name.as_deref(), Some("Blender"));
    assert_eq!(kept.start, Some(1000));

    assert!(tick(&mut plugin, 1200).activity.is_none());
}

#[test]
fn another_tool_restarts_the_counter() {
    let shared = Rc::new(RefCell::new(window("figma.exe")));
    let mut plugin = start(&shared, "en");
    tick(&mut plugin, 1000);
    *shared.borrow_mut() = window("code.exe");
    let shown = tick(&mut plugin, 1010).activity.unwrap();
    assert_eq!(shown.name.as_deref(), Some("Visual Studio Code"));
    assert_eq!(shown.start, Some(1010));
}

#[test]
fn nothing_shows_for_other_programs_or_no_window() {
    let shared = Rc::new(RefCell::new(window("explorer.exe")));
    let mut plugin = start(&shared, "en");
    assert!(tick(&mut plugin, 1000).activity.is_none());
    *shared.borrow_mut() = Value::Null;
    assert!(tick(&mut plugin, 1010).activity.is_none());
    *shared.borrow_mut() = window("constructor");
    assert!(tick(&mut plugin, 1020).activity.is_none());
}
