//! Runs the SA-MP presence against a simulated host: what the game was
//! launched with, and what the server answers.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use rpp_plugins::{Host, Manifest, Plugin, Request};
use serde_json::{Value, json};

/// What the fake computer and server say, and what the presence asked.
struct Machine {
    /// Options of gta_sa.exe as the host would return them; null when not running.
    args: Value,
    /// The server's answer; `Err` when the host refuses the question.
    info: Result<Value, String>,
    foreground: Option<&'static str>,
    asked: Vec<Request>,
}

struct Fake(Rc<RefCell<Machine>>);

impl Host for Fake {
    fn call(&self, request: Request) -> Result<Value, String> {
        let mut machine = self.0.borrow_mut();
        machine.asked.push(request.clone());
        match request {
            Request::ProcessArgs { .. } => Ok(machine.args.clone()),
            Request::ServerQuery { .. } => machine.info.clone(),
            Request::ForegroundWindow => Ok(match machine.foreground {
                Some(process) => json!({"process": process, "title": "GTA:SA:MP"}),
                None => Value::Null,
            }),
            other => Err(format!("not simulated: {other:?}")),
        }
    }
}

fn start(machine: &Rc<RefCell<Machine>>, tag: &str) -> Plugin {
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../presences/sa-mp");
    let manifest =
        Manifest::parse(&fs::read_to_string(folder.join("manifest.json")).unwrap()).unwrap();
    let source = fs::read_to_string(folder.join(&manifest.entry)).unwrap();
    let mut plugin = Plugin::load(manifest, &source, Rc::new(Fake(machine.clone()))).unwrap();
    let read = |tag: &str| -> BTreeMap<String, String> {
        let file = folder.join("locales").join(format!("{tag}.json"));
        serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap()
    };
    let mut strings = read("en");
    strings.extend(read(tag));
    plugin.set_strings(&strings).unwrap();
    plugin
}

fn machine(args: Value, info: Result<Value, String>) -> Rc<RefCell<Machine>> {
    Rc::new(RefCell::new(Machine {
        args,
        info,
        foreground: Some("gta_sa.exe"),
        asked: Vec::new(),
    }))
}

fn launched() -> Value {
    json!({"-h": "144.217.0.10", "-p": "7778"})
}

fn answer(players: u16, max: u16) -> Value {
    json!({
        "hostname": "Los Santos Roleplay",
        "gamemode": "Heavy RP",
        "language": "Spanish",
        "players": players,
        "max_players": max,
        "password": false
    })
}

#[test]
fn the_manifest_reads_only_the_address_and_port() {
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../presences/sa-mp");
    let manifest =
        Manifest::parse(&fs::read_to_string(folder.join("manifest.json")).unwrap()).unwrap();
    let declared = manifest.permissions.args_of("gta_sa.exe").unwrap();
    assert_eq!(declared, ["-h", "-p"]);
    assert_eq!(manifest.permissions.query, ["samp"]);
    assert!(!manifest.permissions.process);
}

#[test]
fn nothing_is_shown_without_a_server_on_the_command_line() {
    for args in [
        Value::Null,
        json!({}),
        json!({"-h": true}),
        json!({"-h": "  "}),
    ] {
        let machine = machine(args.clone(), Ok(answer(10, 50)));
        let mut plugin = start(&machine, "en");
        assert_eq!(plugin.tick(&json!({})).unwrap(), None, "{args}");
    }
}

#[test]
fn the_server_name_mode_and_group_size_are_shown() {
    let machine = machine(launched(), Ok(answer(42, 200)));
    let mut plugin = start(&machine, "en");
    let tick = plugin.poll(&json!({})).unwrap();
    let shown = tick.activity.unwrap();
    assert_eq!(shown.details.as_deref(), Some("Los Santos Roleplay"));
    assert_eq!(shown.state.as_deref(), Some("Heavy RP"));
    assert_eq!(shown.party_size, Some([42, 200]));
    assert_eq!(shown.small_text.as_deref(), Some("Spanish"));
    assert!(shown.start.is_some());
    assert!(tick.meta.foreground);
    assert_eq!(tick.meta.application, "gta_sa.exe");
    // The host was asked for the declared options and the launched server only.
    let asked = machine.borrow().asked.clone();
    assert!(asked.contains(&Request::ProcessArgs {
        name: "gta_sa.exe".into(),
        flags: vec!["-h".into(), "-p".into()],
    }));
    assert!(asked.contains(&Request::ServerQuery {
        protocol: "samp".into(),
        host: "144.217.0.10".into(),
        port: 7778,
    }));
}

#[test]
fn counts_discord_cannot_group_go_in_the_text_and_the_address_is_opt_in() {
    // An empty server, and one bigger than Discord's group limit.
    for (players, max, state) in [
        (0, 50, "Heavy RP - 0/50 jugadores"),
        (1500, 2000, "Heavy RP - 1500/2000 jugadores"),
    ] {
        let machine = machine(launched(), Ok(answer(players, max)));
        let mut plugin = start(&machine, "es");
        let shown = plugin
            .tick(&json!({"options": {"show_players": true, "show_address": true}}))
            .unwrap()
            .unwrap();
        assert_eq!(shown.party_size, None);
        assert_eq!(shown.state.as_deref(), Some(state));
        assert_eq!(shown.small_text.as_deref(), Some("144.217.0.10:7778"));
    }
    let machine = machine(launched(), Ok(answer(42, 200)));
    let mut plugin = start(&machine, "es");
    let hidden = plugin
        .tick(&json!({"options": {"show_players": false, "show_address": false}}))
        .unwrap()
        .unwrap();
    assert_eq!(hidden.party_size, None);
    assert_eq!(hidden.state.as_deref(), Some("Heavy RP"));
}

#[test]
fn a_server_that_does_not_answer_still_shows_the_game() {
    for info in [Ok(Value::Null), Err("too many server queries".to_owned())] {
        let machine = machine(json!({"-h": "play.example.com"}), info);
        let mut plugin = start(&machine, "es");
        let shown = plugin.tick(&json!({})).unwrap().unwrap();
        assert_eq!(shown.details.as_deref(), Some("Jugando SA-MP"));
        assert_eq!(shown.state, None);
        assert_eq!(shown.party_size, None);
        // No port on the command line means the default one.
        let asked = machine.borrow().asked.clone();
        assert!(asked.contains(&Request::ServerQuery {
            protocol: "samp".into(),
            host: "play.example.com".into(),
            port: 7777,
        }));
    }
}
