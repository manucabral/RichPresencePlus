//! The catalog end to end: signed index in, installed presence out.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::*;
use crate::index::tests::{entry_for, keys, payload, sign};
use crate::install::tests::Memory;

/// A source the test keeps a handle on while the catalog owns another.
struct Shared(Arc<Memory>);

impl Source for Shared {
    fn index(&self, channel: Channel) -> Result<Vec<u8>, Error> {
        self.0.index(channel)
    }

    fn file(&self, commit: &str, folder: &str, path: &str, size: u64) -> Result<Vec<u8>, Error> {
        self.0.file(commit, folder, path, size)
    }
}

const SCRIPT: &[u8] = b"function read(){return null} function activity(){return null}";

fn manifest(name: &str, version: &str, format: u32) -> Vec<u8> {
    format!(
        r#"{{"format":{format},"name":"{name}","version":"{version}","client_id":"12345678901234567","entry":"main.js"}}"#
    )
    .into_bytes()
}

/// Publishes presences to the in-memory source and signs the index.
fn publish(memory: &Memory, generated_at: u64, presences: &[(&str, &str, &str)]) {
    publish_format(memory, generated_at, presences, 1);
}

/// Publishes presences written for `format`.
fn publish_format(
    memory: &Memory,
    generated_at: u64,
    presences: &[(&str, &str, &str)],
    format: u32,
) {
    let mut listed = Vec::new();
    for (name, folder, version) in presences {
        let manifest = manifest(name, version, format);
        let files: [(&str, &[u8]); 2] = [("manifest.json", &manifest), ("main.js", SCRIPT)];
        let mut entry = entry_for(name, folder, version, &files);
        entry["format"] = format.into();
        listed.push(entry);
        for (path, bytes) in files {
            memory.put(folder, path, bytes);
        }
    }
    let mut text: serde_json::Value = serde_json::from_str(&payload("stable", listed)).unwrap();
    text["generated_at"] = generated_at.into();
    *memory.index_bytes.lock().unwrap() = Some(sign(&text.to_string(), 1, "main"));
}

struct Fixture {
    memory: Arc<Memory>,
    catalog: Catalog,
    presences: tempfile::TempDir,
    data: tempfile::TempDir,
}

fn fixture() -> Fixture {
    let memory = Arc::new(Memory::default());
    let presences = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let catalog = Catalog::new(
        Box::new(Shared(memory.clone())),
        keys(),
        Channel::Stable,
        presences.path(),
        data.path(),
        crate::install::tests::key(),
    );
    Fixture {
        memory,
        catalog,
        presences,
        data,
    }
}

#[test]
fn listing_shows_what_is_installed_and_what_can_be_updated() {
    let f = fixture();
    publish(
        &f.memory,
        100,
        &[("Alpha", "alpha", "1.0.0"), ("Beta", "beta", "1.0.0")],
    );
    let states: Vec<_> = f
        .catalog
        .listing(true)
        .unwrap()
        .into_iter()
        .map(|l| l.state)
        .collect();
    assert!(states.iter().all(|s| *s == State::Available));

    f.catalog.install("alpha", &[]).unwrap();
    publish(
        &f.memory,
        200,
        &[("Alpha", "alpha", "1.1.0"), ("Beta", "beta", "1.0.0")],
    );
    let listing = f.catalog.listing(true).unwrap();
    assert_eq!(
        listing[0].state,
        State::Update {
            installed: Version::new(1, 0, 0)
        }
    );
    assert_eq!(listing[1].state, State::Available);

    f.catalog.install("Alpha", &[]).unwrap();
    assert_eq!(
        f.catalog.listing(false).unwrap()[0].state,
        State::Installed {
            version: Version::new(1, 1, 0)
        }
    );
}

#[test]
fn a_presence_for_a_newer_application_is_not_installed_nor_offered_as_an_update() {
    let f = fixture();
    publish(&f.memory, 100, &[("Alpha", "alpha", "1.0.0")]);
    f.catalog.install("alpha", &[]).unwrap();
    let asked = f.memory.asked.lock().unwrap().len();
    let newer = rpp_plugins::FORMAT + 1;
    publish_format(
        &f.memory,
        200,
        &[("Alpha", "alpha", "1.1.0"), ("Beta", "beta", "1.0.0")],
        newer,
    );
    let listing = f.catalog.listing(true).unwrap();
    assert_eq!(
        listing[0].state,
        State::Installed {
            version: Version::new(1, 0, 0)
        },
        "the copy that works stays, with no update to break it"
    );
    assert!(listing[0].entry.too_new() && listing[1].entry.too_new());
    assert!(matches!(f.catalog.install("Beta", &[]), Err(Error::Newer(name)) if name == "Beta"));
    assert!(matches!(
        f.catalog.install("Alpha", &[]),
        Err(Error::Newer(_))
    ));
    assert!(!f.presences.path().join("beta").exists());
    assert_eq!(
        f.memory.asked.lock().unwrap().len(),
        asked,
        "refused before any download"
    );
}

#[test]
fn install_and_remove_work_on_the_presences_folder() {
    let f = fixture();
    publish(&f.memory, 100, &[("Alpha", "alpha", "1.0.0")]);
    let entry = f.catalog.install("Alpha", &[]).unwrap();
    assert_eq!(entry.folder, "alpha");
    assert!(f.presences.path().join("alpha/main.js").is_file());
    f.catalog.remove("alpha").unwrap();
    assert!(!f.presences.path().join("alpha").exists());
    assert!(matches!(
        f.catalog.remove("alpha"),
        Err(Error::NotManaged(_))
    ));
    assert!(matches!(
        f.catalog.install("nothing", &[]),
        Err(Error::Unknown(_))
    ));
}

#[test]
fn the_index_is_reused_for_a_while_and_refreshed_on_request() {
    let f = fixture();
    publish(&f.memory, 100, &[("Alpha", "alpha", "1.0.0")]);
    f.catalog.listing(false).unwrap();
    f.catalog.listing(false).unwrap();
    f.catalog.install("alpha", &[]).unwrap();
    assert_eq!(f.memory.index_calls.load(Ordering::SeqCst), 1);
    f.catalog.listing(true).unwrap();
    assert_eq!(f.memory.index_calls.load(Ordering::SeqCst), 2);
}

#[test]
fn a_replayed_old_index_is_refused_even_though_it_is_signed() {
    let f = fixture();
    publish(&f.memory, 200, &[("Alpha", "alpha", "1.1.0")]);
    f.catalog.listing(true).unwrap();
    publish(&f.memory, 100, &[("Alpha", "alpha", "1.0.0")]);
    assert!(matches!(f.catalog.listing(true), Err(Error::Stale)));
    assert!(f.data.path().join("catalog-seen.json").is_file());
}

#[test]
fn offline_and_unsigned_indexes_fail_cleanly() {
    let f = fixture();
    assert!(matches!(f.catalog.listing(true), Err(Error::Network(_))));
    *f.memory.index_bytes.lock().unwrap() = Some(b"{}".to_vec());
    assert!(matches!(f.catalog.listing(true), Err(Error::Signature)));
    assert!(matches!(
        f.catalog.install("alpha", &[]),
        Err(Error::Signature)
    ));
    assert!(f.presences.path().read_dir().unwrap().next().is_none());
}

#[test]
fn without_keys_the_catalog_is_disabled() {
    let memory = Arc::new(Memory::default());
    publish(&memory, 100, &[("Alpha", "alpha", "1.0.0")]);
    let dir = tempfile::tempdir().unwrap();
    let catalog = Catalog::new(
        Box::new(Shared(memory.clone())),
        Keys::default(),
        Channel::Stable,
        dir.path(),
        dir.path(),
        crate::install::tests::key(),
    );
    assert!(!catalog.enabled());
    assert!(matches!(catalog.listing(true), Err(Error::Disabled)));
    assert_eq!(
        memory.index_calls.load(Ordering::SeqCst),
        0,
        "no request is made"
    );
}

#[test]
fn starting_up_cleans_an_interrupted_install() {
    let presences = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(presences.path().join(".installing-alpha")).unwrap();
    let data = tempfile::tempdir().unwrap();
    let _ = Catalog::new(
        Box::new(Shared(Arc::new(Memory::default()))),
        keys(),
        Channel::Stable,
        presences.path(),
        data.path(),
        crate::install::tests::key(),
    );
    assert!(!presences.path().join(".installing-alpha").exists());
}

#[test]
fn an_update_can_be_undone_by_name() {
    let f = fixture();
    publish(&f.memory, 100, &[("Alpha", "alpha", "1.0.0")]);
    f.catalog.install("Alpha", &[]).unwrap();
    assert!(f.catalog.history("alpha").unwrap().is_empty());

    publish(&f.memory, 200, &[("Alpha", "alpha", "1.1.0")]);
    f.catalog.listing(true).unwrap();
    f.catalog.install("Alpha", &[]).unwrap();
    assert_eq!(f.catalog.history("ALPHA").unwrap(), [Version::new(1, 0, 0)]);

    let back = f.catalog.restore("alpha", &Version::new(1, 0, 0)).unwrap();
    assert_eq!(back.version, Version::new(1, 0, 0));
    assert_eq!(f.catalog.history("alpha").unwrap(), [Version::new(1, 1, 0)]);
    assert!(matches!(
        f.catalog.restore("alpha", &Version::new(7, 0, 0)),
        Err(Error::Unknown(_))
    ));
    assert!(matches!(
        f.catalog.history("nothing"),
        Err(Error::NotManaged(_))
    ));
    assert!(matches!(
        f.catalog.restore("nothing", &Version::new(1, 0, 0)),
        Err(Error::NotManaged(_))
    ));
}

#[test]
fn a_copy_changed_by_hand_is_listed_as_modified_and_a_reinstall_repairs_it() {
    let f = fixture();
    publish(&f.memory, 100, &[("Alpha", "alpha", "1.0.0")]);
    f.catalog.install("Alpha", &[]).unwrap();
    std::fs::write(f.presences.path().join("alpha/main.js"), "steal()").unwrap();

    let state = |catalog: &Catalog| catalog.listing(false).unwrap()[0].state.clone();
    assert_eq!(
        state(&f.catalog),
        State::Modified {
            installed: Version::new(1, 0, 0)
        }
    );
    f.catalog.install("Alpha", &[]).unwrap();
    assert_eq!(
        state(&f.catalog),
        State::Installed {
            version: Version::new(1, 0, 0)
        }
    );
    assert_ne!(
        std::fs::read(f.presences.path().join("alpha/main.js")).unwrap(),
        b"steal()"
    );
}

#[test]
fn only_a_catalog_folder_that_changed_is_refused() {
    let f = fixture();
    publish(&f.memory, 100, &[("Alpha", "alpha", "1.0.0")]);
    f.catalog.install("Alpha", &[]).unwrap();
    let alpha = f.presences.path().join("alpha");
    assert_eq!(f.catalog.refusal(&alpha), None);

    std::fs::write(alpha.join("main.js"), "steal()").unwrap();
    assert!(f.catalog.refusal(&alpha).unwrap().contains("Alpha"));

    // A folder the user made has no marker; one outside the presences folder is not the catalog's.
    let mine = f.presences.path().join("mine");
    std::fs::create_dir_all(&mine).unwrap();
    std::fs::write(mine.join("main.js"), "x").unwrap();
    assert_eq!(f.catalog.refusal(&mine), None);
    let elsewhere = tempfile::tempdir().unwrap();
    let moved = elsewhere.path().join("alpha");
    std::fs::rename(&alpha, &moved).unwrap();
    assert_eq!(f.catalog.refusal(&moved), None);
}

#[test]
fn a_marker_for_another_folder_or_name_does_not_attach_to_an_entry() {
    let f = fixture();
    publish(&f.memory, 100, &[("Alpha", "alpha", "1.0.0")]);
    f.catalog.install("Alpha", &[]).unwrap();
    // A folder that carries a marker naming `alpha` but sits elsewhere is ignored.
    let copy = f.presences.path().join("copy");
    std::fs::create_dir_all(&copy).unwrap();
    std::fs::copy(
        f.presences.path().join("alpha/.catalog.json"),
        copy.join(".catalog.json"),
    )
    .unwrap();
    assert_eq!(f.catalog.listing(false).unwrap().len(), 1);
    assert_eq!(
        f.catalog.listing(false).unwrap()[0].state,
        State::Installed {
            version: Version::new(1, 0, 0)
        }
    );
}

/// A second catalog on the same folders, as after restarting the application.
fn restarted(f: &Fixture) -> Catalog {
    Catalog::new(
        Box::new(Shared(f.memory.clone())),
        keys(),
        Channel::Stable,
        f.presences.path(),
        f.data.path(),
        crate::install::tests::key(),
    )
}

#[test]
fn without_github_the_last_verified_index_still_lists_the_catalog() {
    let f = fixture();
    publish(&f.memory, 100, &[("Alpha", "alpha", "1.0.0")]);
    f.catalog.listing(true).unwrap();

    // GitHub stops answering; a restarted application still lists the catalog.
    *f.memory.index_bytes.lock().unwrap() = None;
    let later = restarted(&f);
    let listed = later.listing(true).unwrap();
    assert_eq!(listed[0].entry.name, "Alpha");
}

#[test]
fn a_saved_index_that_was_changed_or_replaced_is_not_used() {
    let f = fixture();
    publish(&f.memory, 100, &[("Alpha", "alpha", "1.0.0")]);
    f.catalog.listing(true).unwrap();
    let saved = f.data.path().join("catalog.json");

    // Edited by hand: the signature no longer matches.
    let text = std::fs::read_to_string(&saved).unwrap();
    std::fs::write(&saved, text.replace("Alpha", "Evil")).unwrap();
    *f.memory.index_bytes.lock().unwrap() = None;
    assert!(matches!(
        restarted(&f).listing(true),
        Err(Error::Network(_))
    ));

    // An older signed index put in its place is refused as a replay.
    let f = fixture();
    publish(&f.memory, 100, &[("Alpha", "alpha", "1.0.0")]);
    let old = f.memory.index_bytes.lock().unwrap().clone().unwrap();
    publish(&f.memory, 200, &[("Alpha", "alpha", "1.1.0")]);
    f.catalog.listing(true).unwrap();
    std::fs::write(f.data.path().join("catalog.json"), old).unwrap();
    *f.memory.index_bytes.lock().unwrap() = None;
    assert!(matches!(
        restarted(&f).listing(true),
        Err(Error::Network(_))
    ));
}
