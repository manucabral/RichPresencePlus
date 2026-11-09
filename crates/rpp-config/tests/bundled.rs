//! Every presence of `presences/`, loaded the way the application loads a folder.
//!
//! It runs on each change, so a change to the format or the engine that would stop a
//! published presence fails here, before a release.

use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use rpp_config::load_presence;
use rpp_plugins::{Host, Plugin, Request};
use serde_json::Value;

struct Silent;

impl Host for Silent {
    fn call(&self, _: Request) -> Result<Value, String> {
        Err("no host".into())
    }
}

#[test]
fn every_presence_of_the_repository_loads_and_its_script_starts() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../presences");
    let mut loaded = 0;
    for entry in fs::read_dir(&root).unwrap() {
        let folder = entry.unwrap().path();
        if !folder.join("manifest.json").is_file() {
            continue;
        }
        let name = folder.file_name().unwrap().to_string_lossy().into_owned();
        let found = load_presence(&folder).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(found.warnings.is_empty(), "{name}: {:?}", found.warnings);
        let presence = found.presence;
        Plugin::load(presence.manifest, &presence.source, Rc::new(Silent))
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        loaded += 1;
    }
    assert!(
        loaded >= 10,
        "only {loaded} presences found in {}",
        root.display()
    );
}
