//! A script reading a log through the real host: find the newest file, then
//! read its last lines. Both calls pass the permission check with the paths
//! the host itself returns.

use std::fs;
use std::rc::Rc;

use rpp_host::SystemHost;
use rpp_plugins::{Manifest, Plugin};
use serde_json::json;

const SCRIPT: &str = "
    function read() {
        const log = host.files.latest(host.files.roots[0], '_last.log');
        return { log, lines: log ? host.files.tail(log, 2) : [] };
    }
    function activity(s) { return { details: s.lines.join('|') }; }
";

fn manifest(folder: &str) -> Manifest {
    let document = json!({
        "name": "Chain", "version": "1", "client_id": "12345678901234567",
        "permissions": {"files": [folder]},
    });
    Manifest::parse(&document.to_string()).unwrap()
}

#[test]
fn latest_then_tail_works_with_the_paths_the_host_returns() {
    let logs = tempfile::tempdir().unwrap();
    let folder = fs::canonicalize(logs.path()).unwrap();
    let declared = folder
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .to_owned();
    fs::write(folder.join("session_last.log"), "one\ntwo\nthree\n").unwrap();

    let mut plugin = Plugin::load(manifest(&declared), SCRIPT, Rc::new(SystemHost)).unwrap();
    let shown = plugin.tick(&json!({})).unwrap().unwrap();
    assert_eq!(shown.details.as_deref(), Some("two|three"));
}

#[test]
fn the_returned_path_has_no_verbatim_prefix() {
    let logs = tempfile::tempdir().unwrap();
    let folder = fs::canonicalize(logs.path()).unwrap();
    let declared = folder
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .to_owned();
    fs::write(folder.join("a_last.log"), "x\n").unwrap();
    let source = "function read(){ return { log: host.files.latest(host.files.roots[0], '_last.log') } } function activity(s){ return { details: s.log } }";
    let mut plugin = Plugin::load(manifest(&declared), source, Rc::new(SystemHost)).unwrap();
    let path = plugin.tick(&json!({})).unwrap().unwrap().details.unwrap();
    assert!(!path.starts_with(r"\\?\"), "{path}");
    assert!(path.ends_with("a_last.log"));
}

#[test]
fn a_folder_without_logs_gives_no_path() {
    let logs = tempfile::tempdir().unwrap();
    let folder = fs::canonicalize(logs.path()).unwrap();
    let declared = folder
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .to_owned();
    let mut plugin = Plugin::load(manifest(&declared), SCRIPT, Rc::new(SystemHost)).unwrap();
    // An empty text is dropped by validation, so no details remain.
    assert_eq!(plugin.tick(&json!({})).unwrap().unwrap().details, None);
}
