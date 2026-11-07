//! Installing, replacing, removing and recovering, against an in-memory source.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::index::tests::{COMMIT, entry_for, keys, payload, sign};
use crate::index::{Channel, Index};

/// The key the tests seal with.
pub(crate) fn key() -> Key {
    Key::new(&"k".repeat(40)).unwrap()
}

/// Serves files from memory and records what was asked.
#[derive(Default)]
pub(crate) struct Memory {
    pub files: Mutex<HashMap<String, Vec<u8>>>,
    pub asked: Mutex<Vec<String>>,
    /// What `catalog.json` serves; nothing means a network failure.
    pub index_bytes: Mutex<Option<Vec<u8>>>,
    pub index_calls: AtomicUsize,
    /// Release bundles by name.
    pub bundles: Mutex<HashMap<String, Vec<u8>>>,
}

impl Memory {
    pub fn put(&self, folder: &str, path: &str, bytes: &[u8]) {
        self.files
            .lock()
            .unwrap()
            .insert(format!("{COMMIT}/{folder}/{path}"), bytes.to_vec());
    }
}

impl Source for Memory {
    fn index(&self, _: Channel) -> Result<Vec<u8>, Error> {
        self.index_calls.fetch_add(1, Ordering::SeqCst);
        self.index_bytes
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| Error::Network("offline".into()))
    }

    fn file(&self, commit: &str, folder: &str, path: &str, _: u64) -> Result<Vec<u8>, Error> {
        let key = format!("{commit}/{folder}/{path}");
        self.asked.lock().unwrap().push(key.clone());
        self.files
            .lock()
            .unwrap()
            .get(&key)
            .cloned()
            .ok_or_else(|| Error::Network(format!("404 {key}")))
    }

    fn bundle(&self, _: &str, name: &str, _: u64) -> Result<Vec<u8>, Error> {
        self.bundles
            .lock()
            .unwrap()
            .get(name)
            .cloned()
            .ok_or_else(|| Error::Network(format!("404 {name}")))
    }
}

fn manifest(name: &str, version: &str) -> Vec<u8> {
    format!(
        r#"{{"name":"{name}","version":"{version}","client_id":"12345678901234567","entry":"main.js"}}"#
    )
    .into_bytes()
}

const SCRIPT: &[u8] = b"function read(){return null} function activity(){return null}";

/// Builds a verified index whose presence asks for `permissions` and reads `sites`.
fn setup_asking(version: &str, permissions: &str, sites: &str) -> (Index, Memory) {
    let manifest = format!(
        r#"{{"name":"Demo","version":"{version}","client_id":"12345678901234567","entry":"main.js","permissions":{permissions},"page":{{"matches":[{sites}]}}}}"#
    )
    .into_bytes();
    let files: [(&str, &[u8]); 2] = [("manifest.json", &manifest), ("main.js", SCRIPT)];
    let text = payload("stable", vec![entry_for("Demo", "demo", version, &files)]);
    let index = Index::verify(&sign(&text, 1, "main"), &keys(), Channel::Stable).unwrap();
    let source = Memory::default();
    for (path, bytes) in files {
        source.put("demo", path, bytes);
    }
    (index, source)
}

/// Builds a verified index with one presence and a source that serves it.
fn setup(name: &str, version: &str) -> (Index, Memory) {
    let manifest = manifest(name, version);
    let files: [(&str, &[u8]); 2] = [("manifest.json", &manifest), ("main.js", SCRIPT)];
    let text = payload("stable", vec![entry_for(name, "demo", version, &files)]);
    let index = Index::verify(&sign(&text, 1, "main"), &keys(), Channel::Stable).unwrap();
    let source = Memory::default();
    for (path, bytes) in files {
        source.put("demo", path, bytes);
    }
    (index, source)
}

#[test]
fn a_presence_is_downloaded_verified_and_marked() {
    let root = tempfile::tempdir().unwrap();
    let history = tempfile::tempdir().unwrap();
    let (index, source) = setup("Demo", "1.0.0");
    install(
        root.path(),
        history.path(),
        &key(),
        &index,
        &index.presences[0],
        &source,
        &[],
    )
    .unwrap();

    let folder = root.path().join("demo");
    assert_eq!(fs::read(folder.join("main.js")).unwrap(), SCRIPT);
    assert!(folder.join("manifest.json").is_file());
    let record = installed(root.path(), &key());
    assert_eq!(record.len(), 1);
    assert_eq!(record[0].name, "Demo");
    assert_eq!(record[0].commit, COMMIT);
    assert_eq!(record[0].standing, Standing::Intact);
    // Nothing is left beside it.
    let names: Vec<_> = fs::read_dir(root.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(names, ["demo"]);
}

#[test]
fn an_update_replaces_the_old_files_completely() {
    let root = tempfile::tempdir().unwrap();
    let history = tempfile::tempdir().unwrap();
    let (index, source) = setup("Demo", "1.0.0");
    install(
        root.path(),
        history.path(),
        &key(),
        &index,
        &index.presences[0],
        &source,
        &[],
    )
    .unwrap();
    fs::write(root.path().join("demo").join("stale.txt"), "old").unwrap();

    let (newer, source) = setup("Demo", "1.1.0");
    install(
        root.path(),
        history.path(),
        &key(),
        &newer,
        &newer.presences[0],
        &source,
        &[],
    )
    .unwrap();
    assert!(!root.path().join("demo").join("stale.txt").exists());
    assert_eq!(
        installed(root.path(), &key())[0].version.to_string(),
        "1.1.0"
    );
    assert!(!root.path().join(".previous-demo").exists());
}

#[test]
fn a_tampered_file_installs_nothing_and_keeps_the_old_copy() {
    let root = tempfile::tempdir().unwrap();
    let history = tempfile::tempdir().unwrap();
    let (index, source) = setup("Demo", "1.0.0");
    install(
        root.path(),
        history.path(),
        &key(),
        &index,
        &index.presences[0],
        &source,
        &[],
    )
    .unwrap();

    let (newer, source) = setup("Demo", "1.1.0");
    source.put("demo", "main.js", b"function read(){ steal() }");
    let result = install(
        root.path(),
        history.path(),
        &key(),
        &newer,
        &newer.presences[0],
        &source,
        &[],
    );
    assert!(matches!(result, Err(Error::Integrity(_))));
    assert_eq!(
        installed(root.path(), &key())[0].version.to_string(),
        "1.0.0"
    );
    assert_eq!(fs::read(root.path().join("demo/main.js")).unwrap(), SCRIPT);
    assert!(!root.path().join(".installing-demo").exists());
}

#[test]
fn a_file_of_the_wrong_length_or_a_missing_one_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let history = tempfile::tempdir().unwrap();
    let (index, source) = setup("Demo", "1.0.0");
    let mut longer = SCRIPT.to_vec();
    longer.push(b' ');
    source.put("demo", "main.js", &longer);
    assert!(matches!(
        install(
            root.path(),
            history.path(),
            &key(),
            &index,
            &index.presences[0],
            &source,
            &[]
        ),
        Err(Error::Integrity(_))
    ));
    source.files.lock().unwrap().clear();
    assert!(matches!(
        install(
            root.path(),
            history.path(),
            &key(),
            &index,
            &index.presences[0],
            &source,
            &[]
        ),
        Err(Error::Network(_))
    ));
    assert!(installed(root.path(), &key()).is_empty());
    assert!(!root.path().join("demo").exists());
}

#[test]
fn a_manifest_that_disagrees_with_the_index_is_refused() {
    // Signed files, but the manifest inside names another presence or version.
    for (name, version) in [("Other", "1.0.0"), ("Demo", "9.9.9")] {
        let root = tempfile::tempdir().unwrap();
        let history = tempfile::tempdir().unwrap();
        let wrong = manifest(name, version);
        let files: [(&str, &[u8]); 2] = [("manifest.json", &wrong), ("main.js", SCRIPT)];
        let text = payload("stable", vec![entry_for("Demo", "demo", "1.0.0", &files)]);
        let index = Index::verify(&sign(&text, 1, "main"), &keys(), Channel::Stable).unwrap();
        let source = Memory::default();
        for (path, bytes) in files {
            source.put("demo", path, bytes);
        }
        let result = install(
            root.path(),
            history.path(),
            &key(),
            &index,
            &index.presences[0],
            &source,
            &[],
        );
        assert!(matches!(result, Err(Error::Invalid(_))), "{name} {version}");
        assert!(!root.path().join("demo").exists());
    }
}

#[test]
fn the_script_must_be_one_of_the_signed_files() {
    let root = tempfile::tempdir().unwrap();
    let history = tempfile::tempdir().unwrap();
    let text = manifest("Demo", "1.0.0");
    // The manifest points at main.js but the index lists only the manifest.
    let files: [(&str, &[u8]); 1] = [("manifest.json", &text)];
    let signed = payload("stable", vec![entry_for("Demo", "demo", "1.0.0", &files)]);
    let index = Index::verify(&sign(&signed, 1, "main"), &keys(), Channel::Stable).unwrap();
    let source = Memory::default();
    source.put("demo", "manifest.json", &text);
    assert!(matches!(
        install(
            root.path(),
            history.path(),
            &key(),
            &index,
            &index.presences[0],
            &source,
            &[]
        ),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn only_catalog_presences_can_be_removed() {
    let root = tempfile::tempdir().unwrap();
    let history = tempfile::tempdir().unwrap();
    let (index, source) = setup("Demo", "1.0.0");
    install(
        root.path(),
        history.path(),
        &key(),
        &index,
        &index.presences[0],
        &source,
        &[],
    )
    .unwrap();

    // A folder made by hand, and a copy of a managed folder under another name.
    fs::create_dir_all(root.path().join("mine")).unwrap();
    fs::write(root.path().join("mine/manifest.json"), "{}").unwrap();
    fs::create_dir_all(root.path().join("copy")).unwrap();
    fs::copy(
        root.path().join("demo/.catalog.json"),
        root.path().join("copy/.catalog.json"),
    )
    .unwrap();
    assert_eq!(
        installed(root.path(), &key()).len(),
        1,
        "the copy is not trusted"
    );
    assert!(matches!(
        remove(root.path(), &key(), "mine"),
        Err(Error::NotManaged(_))
    ));
    assert!(matches!(
        remove(root.path(), &key(), "copy"),
        Err(Error::NotManaged(_))
    ));
    assert!(root.path().join("mine").is_dir());

    remove(root.path(), &key(), "demo").unwrap();
    assert!(!root.path().join("demo").exists());
    assert!(root.path().join("mine").is_dir());
    assert!(matches!(
        remove(root.path(), &key(), "Demo"),
        Err(Error::NotManaged(_))
    ));
}

#[test]
fn leftovers_are_cleaned_and_an_interrupted_replace_is_undone() {
    let root = tempfile::tempdir().unwrap();
    for name in [".installing-a", ".removing-b"] {
        fs::create_dir_all(root.path().join(name)).unwrap();
    }
    // The new copy never arrived: the old one comes back.
    fs::create_dir_all(root.path().join(".previous-lost")).unwrap();
    fs::write(root.path().join(".previous-lost/main.js"), "kept").unwrap();
    // The new copy arrived: the old one goes.
    fs::create_dir_all(root.path().join(".previous-done")).unwrap();
    fs::create_dir_all(root.path().join("done")).unwrap();

    clean_leftovers(root.path());
    assert!(!root.path().join(".installing-a").exists());
    assert!(!root.path().join(".removing-b").exists());
    assert!(!root.path().join(".previous-done").exists());
    assert!(!root.path().join(".previous-lost").exists());
    assert_eq!(fs::read(root.path().join("lost/main.js")).unwrap(), b"kept");
    assert!(root.path().join("done").is_dir());
}

#[test]
fn an_unsafe_folder_never_reaches_the_disk() {
    let root = tempfile::tempdir().unwrap();
    let history = tempfile::tempdir().unwrap();
    let (index, source) = setup("Demo", "1.0.0");
    let mut entry = index.presences[0].clone();
    entry.folder = "../escape".into();
    assert!(matches!(
        install(
            root.path(),
            history.path(),
            &key(),
            &index,
            &entry,
            &source,
            &[]
        ),
        Err(Error::Invalid(_))
    ));
    assert!(source.asked.lock().unwrap().is_empty());
}

/// Installs a version of Demo into `root`, keeping replaced copies in `history`.
fn put(root: &Path, history: &Path, version: &str) {
    let (index, source) = setup("Demo", version);
    install(
        root,
        history,
        &key(),
        &index,
        &index.presences[0],
        &source,
        &[],
    )
    .unwrap();
}

fn kept(history: &Path) -> Vec<String> {
    crate::history::versions(history, "demo")
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn replaced_copies_are_kept_two_at_a_time_and_can_be_brought_back() {
    let root = tempfile::tempdir().unwrap();
    let history = tempfile::tempdir().unwrap();
    for version in ["1.0.0", "1.1.0", "1.2.0"] {
        put(root.path(), history.path(), version);
    }
    assert_eq!(kept(history.path()), ["1.1.0", "1.0.0"]);

    let record = installed(root.path(), &key()).remove(0);
    let back = restore(
        root.path(),
        history.path(),
        &key(),
        &record,
        &"1.0.0".parse().unwrap(),
    )
    .unwrap();
    assert_eq!(back.version.to_string(), "1.0.0");
    assert_eq!(
        installed(root.path(), &key())[0].version.to_string(),
        "1.0.0"
    );
    // The version that was replaced is kept in turn; nothing is left behind.
    assert_eq!(kept(history.path()), ["1.2.0", "1.1.0"]);
    let names: Vec<_> = fs::read_dir(root.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(names, ["demo"]);
}

#[test]
fn restoring_a_version_that_is_not_kept_changes_nothing() {
    let root = tempfile::tempdir().unwrap();
    let history = tempfile::tempdir().unwrap();
    put(root.path(), history.path(), "1.0.0");
    let record = installed(root.path(), &key()).remove(0);
    let result = restore(
        root.path(),
        history.path(),
        &key(),
        &record,
        &"0.9.0".parse().unwrap(),
    );
    assert!(matches!(result, Err(Error::Unknown(_))));
    assert_eq!(
        installed(root.path(), &key())[0].version.to_string(),
        "1.0.0"
    );
    assert!(kept(history.path()).is_empty());
}

#[test]
fn a_failed_update_leaves_the_history_alone() {
    let root = tempfile::tempdir().unwrap();
    let history = tempfile::tempdir().unwrap();
    put(root.path(), history.path(), "1.0.0");
    put(root.path(), history.path(), "1.1.0");
    assert_eq!(kept(history.path()), ["1.0.0"]);

    let (newer, source) = setup("Demo", "1.2.0");
    source.put("demo", "main.js", b"function read(){ steal() }");
    let result = install(
        root.path(),
        history.path(),
        &key(),
        &newer,
        &newer.presences[0],
        &source,
        &[],
    );
    assert!(matches!(result, Err(Error::Integrity(_))));
    assert_eq!(kept(history.path()), ["1.0.0"]);
    assert_eq!(
        installed(root.path(), &key())[0].version.to_string(),
        "1.1.0"
    );
}

#[test]
fn a_folder_the_user_made_is_never_replaced() {
    let root = tempfile::tempdir().unwrap();
    let history = tempfile::tempdir().unwrap();
    let mine = root.path().join("demo");
    fs::create_dir_all(&mine).unwrap();
    fs::write(mine.join("main.js"), "my own script").unwrap();

    let (index, source) = setup("Demo", "1.0.0");
    let result = install(
        root.path(),
        history.path(),
        &key(),
        &index,
        &index.presences[0],
        &source,
        &[],
    );
    assert!(matches!(result, Err(Error::Invalid(_))));
    assert_eq!(
        fs::read_to_string(mine.join("main.js")).unwrap(),
        "my own script"
    );
    assert!(
        source.asked.lock().unwrap().is_empty(),
        "nothing was downloaded"
    );
}

/// The user can create a presence while the download runs; the check before
/// it is not enough.
#[test]
fn a_folder_made_during_the_download_is_not_replaced() {
    let root = tempfile::tempdir().unwrap();
    let history = tempfile::tempdir().unwrap();
    let staged = Staging::create(root.path(), "demo").unwrap();
    fs::write(staged.path().join("main.js"), "from the catalog").unwrap();

    let mine = root.path().join("demo");
    fs::create_dir_all(&mine).unwrap();
    fs::write(mine.join("main.js"), "my own script").unwrap();

    let result = swap(root.path(), history.path(), &key(), staged, "demo");
    assert!(matches!(result, Err(Error::Invalid(_))));
    assert_eq!(
        fs::read_to_string(mine.join("main.js")).unwrap(),
        "my own script"
    );
    let left: Vec<_> = fs::read_dir(root.path()).unwrap().collect();
    assert_eq!(left.len(), 1, "the staged copy stayed behind");
}

fn standing(root: &Path) -> Standing {
    installed(root, &key())[0].standing
}

fn marker_path(root: &Path) -> PathBuf {
    root.join("demo").join(MARKER)
}

#[test]
fn a_fresh_install_is_intact_and_hidden_or_system_files_do_not_change_that() {
    let root = tempfile::tempdir().unwrap();
    let history = tempfile::tempdir().unwrap();
    put(root.path(), history.path(), "1.0.0");
    assert_eq!(standing(root.path()), Standing::Intact);

    fs::write(root.path().join("demo/.hidden"), "x").unwrap();
    fs::write(root.path().join("demo/Thumbs.db"), "x").unwrap();
    fs::write(root.path().join("demo/desktop.ini"), "x").unwrap();
    assert_eq!(standing(root.path()), Standing::Intact);
}

#[test]
fn changing_adding_or_deleting_a_file_makes_the_copy_modified() {
    type Change = fn(&Path);
    let tamper: [(&str, Change); 5] = [
        ("edited script", |f| {
            fs::write(f.join("main.js"), "steal()").unwrap()
        }),
        ("edited manifest", |f| {
            let text = fs::read_to_string(f.join("manifest.json")).unwrap();
            fs::write(f.join("manifest.json"), text.replace("Demo", "Demo2")).unwrap();
        }),
        ("extra script", |f| {
            fs::write(f.join("extra.js"), "x").unwrap()
        }),
        ("extra translation", |f| {
            fs::create_dir_all(f.join("locales")).unwrap();
            fs::write(f.join("locales/en.json"), "{}").unwrap();
        }),
        ("deleted script", |f| {
            fs::remove_file(f.join("main.js")).unwrap()
        }),
    ];
    for (what, change) in tamper {
        let root = tempfile::tempdir().unwrap();
        let history = tempfile::tempdir().unwrap();
        put(root.path(), history.path(), "1.0.0");
        change(&root.path().join("demo"));
        assert_eq!(standing(root.path()), Standing::Modified, "{what}");
    }
}

#[test]
fn a_marker_that_is_forged_old_or_made_with_another_key_is_modified() {
    let root = tempfile::tempdir().unwrap();
    let history = tempfile::tempdir().unwrap();
    put(root.path(), history.path(), "1.0.0");
    let other = Key::new(&"o".repeat(40)).unwrap();
    assert_eq!(
        inspect(&root.path().join("demo"), &other).unwrap().standing,
        Standing::Modified,
        "another installation's key"
    );

    let original = fs::read_to_string(marker_path(root.path())).unwrap();
    let mut document: serde_json::Value = serde_json::from_str(&original).unwrap();
    // A marker that lists the right files but carries a made-up seal.
    document["seal"] = serde_json::json!("00".repeat(32));
    fs::write(marker_path(root.path()), document.to_string()).unwrap();
    assert_eq!(standing(root.path()), Standing::Modified, "made-up seal");
    // A marker that claims another version keeps the old seal.
    document["seal"] =
        serde_json::from_str::<serde_json::Value>(&original).unwrap()["seal"].clone();
    document["version"] = serde_json::json!("9.9.9");
    fs::write(marker_path(root.path()), document.to_string()).unwrap();
    assert_eq!(standing(root.path()), Standing::Modified, "edited record");
    // The format before seals existed.
    let old = serde_json::json!({
        "name": "Demo", "folder": "demo", "version": "1.0.0", "commit": COMMIT,
    });
    fs::write(marker_path(root.path()), old.to_string()).unwrap();
    assert_eq!(standing(root.path()), Standing::Modified, "no seal");
}

#[test]
fn reinstalling_over_a_modified_copy_repairs_it_and_does_not_keep_it() {
    let root = tempfile::tempdir().unwrap();
    let history = tempfile::tempdir().unwrap();
    put(root.path(), history.path(), "1.0.0");
    fs::write(root.path().join("demo/main.js"), "steal()").unwrap();
    // The same version again repairs it.
    put(root.path(), history.path(), "1.0.0");
    assert_eq!(standing(root.path()), Standing::Intact);
    assert_eq!(fs::read(root.path().join("demo/main.js")).unwrap(), SCRIPT);
    assert!(
        kept(history.path()).is_empty(),
        "the changed copy is not kept"
    );

    // An update over a modified copy does not keep it either.
    fs::write(root.path().join("demo/extra.js"), "x").unwrap();
    put(root.path(), history.path(), "1.1.0");
    assert_eq!(standing(root.path()), Standing::Intact);
    assert!(kept(history.path()).is_empty());
}

#[test]
fn a_kept_copy_changed_in_the_history_is_not_restored() {
    for tamper in ["edit", "add"] {
        let root = tempfile::tempdir().unwrap();
        let history = tempfile::tempdir().unwrap();
        put(root.path(), history.path(), "1.0.0");
        put(root.path(), history.path(), "1.1.0");
        let slot = crate::history::slot(history.path(), "demo", &"1.0.0".parse().unwrap());
        match tamper {
            "edit" => fs::write(slot.join("main.js"), "steal()").unwrap(),
            _ => fs::write(slot.join("extra.js"), "x").unwrap(),
        }
        let record = installed(root.path(), &key()).remove(0);
        let result = restore(
            root.path(),
            history.path(),
            &key(),
            &record,
            &"1.0.0".parse().unwrap(),
        );
        assert!(matches!(result, Err(Error::Integrity(_))), "{tamper}");
        assert_eq!(
            installed(root.path(), &key())[0].version.to_string(),
            "1.1.0"
        );
        assert_eq!(standing(root.path()), Standing::Intact);
    }
}

#[test]
fn restoring_over_a_modified_copy_works_and_drops_it() {
    let root = tempfile::tempdir().unwrap();
    let history = tempfile::tempdir().unwrap();
    put(root.path(), history.path(), "1.0.0");
    put(root.path(), history.path(), "1.1.0");
    fs::write(root.path().join("demo/main.js"), "steal()").unwrap();
    let record = installed(root.path(), &key()).remove(0);
    let back = restore(
        root.path(),
        history.path(),
        &key(),
        &record,
        &"1.0.0".parse().unwrap(),
    )
    .unwrap();
    assert_eq!(
        (back.version.to_string(), back.standing),
        ("1.0.0".into(), Standing::Intact)
    );
    assert!(kept(history.path()).is_empty());
}

#[test]
fn a_modified_copy_can_still_be_removed() {
    let root = tempfile::tempdir().unwrap();
    let history = tempfile::tempdir().unwrap();
    put(root.path(), history.path(), "1.0.0");
    fs::write(root.path().join("demo/main.js"), "steal()").unwrap();
    remove(root.path(), &key(), "Demo").unwrap();
    assert!(!root.path().join("demo").exists());
}

#[test]
fn a_hand_made_folder_with_a_stray_marker_is_never_replaced() {
    // Copied by hand from a catalog folder: the marker names another folder.
    let root = tempfile::tempdir().unwrap();
    let history = tempfile::tempdir().unwrap();
    let mine = root.path().join("demo");
    fs::create_dir_all(&mine).unwrap();
    fs::write(mine.join("main.js"), "my own script").unwrap();
    let stray = serde_json::json!({
        "name": "Demo", "folder": "other", "version": "1.0.0", "commit": COMMIT,
    });
    fs::write(mine.join(MARKER), stray.to_string()).unwrap();

    let (index, source) = setup("Demo", "1.1.0");
    let result = install(
        root.path(),
        history.path(),
        &key(),
        &index,
        &index.presences[0],
        &source,
        &[],
    );
    assert!(matches!(result, Err(Error::Invalid(_))));
    assert_eq!(
        fs::read_to_string(mine.join("main.js")).unwrap(),
        "my own script"
    );
    assert!(installed(root.path(), &key()).is_empty());
}

const BUNDLE: &str = "demo_1.0.0_0123456789ab.json";

/// Like [`setup`], with the index naming a release bundle.
fn setup_bundled(name: &str, version: &str) -> (Index, Memory, Vec<(String, Vec<u8>)>) {
    let manifest = manifest(name, version);
    let files: Vec<(String, Vec<u8>)> = vec![
        ("manifest.json".into(), manifest),
        ("main.js".into(), SCRIPT.to_vec()),
    ];
    let borrowed: Vec<(&str, &[u8])> = files
        .iter()
        .map(|(p, b)| (p.as_str(), b.as_slice()))
        .collect();
    let mut listed = entry_for(name, "demo", version, &borrowed);
    listed["bundle"] = serde_json::json!(BUNDLE);
    let text = payload("stable", vec![listed]);
    let index = Index::verify(&sign(&text, 1, "main"), &keys(), Channel::Stable).unwrap();
    let source = Memory::default();
    for (path, bytes) in &files {
        source.put("demo", path, bytes);
    }
    (index, source, files)
}

fn bundle_of(version: &str, files: &[(String, Vec<u8>)]) -> Vec<u8> {
    use base64::Engine;
    let encoded: BTreeMap<&str, String> = files
        .iter()
        .map(|(path, bytes)| {
            (
                path.as_str(),
                base64::engine::general_purpose::STANDARD.encode(bytes),
            )
        })
        .collect();
    serde_json::json!({"format": crate::bundle::FORMAT, "folder": "demo", "version": version, "files": encoded})
        .to_string()
        .into_bytes()
}

fn install_demo(root: &Path, history: &Path, index: &Index, source: &Memory) -> Result<(), Error> {
    install(
        root,
        history,
        &key(),
        index,
        &index.presences[0],
        source,
        &[],
    )
}

#[test]
fn a_named_bundle_is_one_download_and_installs_the_signed_files() {
    let root = tempfile::tempdir().unwrap();
    let history = tempfile::tempdir().unwrap();
    let (index, source, files) = setup_bundled("Demo", "1.0.0");
    assert_eq!(index.presences[0].bundle.as_deref(), Some(BUNDLE));
    source
        .bundles
        .lock()
        .unwrap()
        .insert(BUNDLE.into(), bundle_of("1.0.0", &files));

    install_demo(root.path(), history.path(), &index, &source).unwrap();
    assert!(
        source.asked.lock().unwrap().is_empty(),
        "no file was fetched one by one"
    );
    assert_eq!(fs::read(root.path().join("demo/main.js")).unwrap(), SCRIPT);
    assert_eq!(installed(root.path(), &key())[0].standing, Standing::Intact);
}

#[test]
fn a_missing_or_tampered_bundle_falls_back_to_the_signed_files() {
    for served in [None, Some(b"function read(){ steal() }".to_vec())] {
        let root = tempfile::tempdir().unwrap();
        let history = tempfile::tempdir().unwrap();
        let (index, source, mut files) = setup_bundled("Demo", "1.0.0");
        if let Some(script) = served {
            files[1].1 = script;
            source
                .bundles
                .lock()
                .unwrap()
                .insert(BUNDLE.into(), bundle_of("1.0.0", &files));
        }
        install_demo(root.path(), history.path(), &index, &source).unwrap();
        assert_eq!(source.asked.lock().unwrap().len(), 2);
        assert_eq!(fs::read(root.path().join("demo/main.js")).unwrap(), SCRIPT);
    }
}

#[test]
fn a_bundle_name_that_does_not_fit_the_entry_is_ignored() {
    for name in [
        "demo_9.9.9_0123456789ab.json",
        "other_1.0.0_0123456789ab.json",
        "../x.json",
    ] {
        let manifest = manifest("Demo", "1.0.0");
        let files: [(&str, &[u8]); 2] = [("manifest.json", &manifest), ("main.js", SCRIPT)];
        let mut listed = entry_for("Demo", "demo", "1.0.0", &files);
        listed["bundle"] = serde_json::json!(name);
        let text = payload("stable", vec![listed]);
        let index = Index::verify(&sign(&text, 1, "main"), &keys(), Channel::Stable).unwrap();
        assert_eq!(index.presences[0].bundle, None, "{name}");
    }
}

/// Installs what `setup_asking` gives, and returns what the second version got.
fn update_asking(
    first: (&str, &str),
    second: (&str, &str),
    accepted: &[Grant],
) -> (tempfile::TempDir, Result<(), Error>) {
    let root = tempfile::tempdir().unwrap();
    let history = tempfile::tempdir().unwrap();
    let (index, source) = setup_asking("1.0.0", first.0, first.1);
    let entry = &index.presences[0];
    install(
        root.path(),
        history.path(),
        &key(),
        &index,
        entry,
        &source,
        &[],
    )
    .unwrap();
    let (newer, source) = setup_asking("1.1.0", second.0, second.1);
    let entry = &newer.presences[0];
    let result = install(
        root.path(),
        history.path(),
        &key(),
        &newer,
        entry,
        &source,
        accepted,
    );
    (root, result)
}

const SITE: &str = r#""*://www.example.com/*""#;

#[test]
fn an_update_that_asks_for_more_leaves_the_installed_copy_alone() {
    let (root, result) = update_asking(
        ("{}", SITE),
        (r#"{"process":true,"http":["a.example"]}"#, SITE),
        &[],
    );

    let Err(Error::NewAccess(new)) = result else {
        panic!("the update went through: {result:?}");
    };
    assert_eq!(new, [Grant::Http("a.example".into()), Grant::Process]);
    assert_eq!(
        installed(root.path(), &key())[0].version.to_string(),
        "1.0.0"
    );
    // Nothing is left beside it.
    let names: Vec<_> = fs::read_dir(root.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(names, ["demo"]);
}

#[test]
fn a_new_site_counts_as_more_access() {
    let wider = r#""*://*.example.com/*""#;
    let (_root, result) = update_asking(("{}", SITE), ("{}", wider), &[]);
    let Err(Error::NewAccess(new)) = result else {
        panic!("the update went through: {result:?}");
    };
    assert_eq!(new, [Grant::Site("*://*.example.com/*".into())]);
}

#[test]
fn an_update_with_the_same_or_less_access_asks_nothing() {
    let (root, result) = update_asking(
        (r#"{"process":true,"media":true}"#, SITE),
        (r#"{"media":true}"#, SITE),
        &[],
    );
    result.unwrap();
    assert_eq!(
        installed(root.path(), &key())[0].version.to_string(),
        "1.1.0"
    );
}

#[test]
fn accepted_access_lets_the_update_in() {
    let (root, result) = update_asking(
        ("{}", SITE),
        (r#"{"process":true}"#, SITE),
        &[Grant::Process],
    );
    result.unwrap();
    assert_eq!(
        installed(root.path(), &key())[0].version.to_string(),
        "1.1.0"
    );
}

/// A yes covers what the person saw; what appears after it is asked again.
#[test]
fn a_yes_does_not_cover_what_the_person_never_saw() {
    let (_root, result) = update_asking(
        ("{}", SITE),
        (r#"{"process":true,"window":true}"#, SITE),
        &[Grant::Process],
    );
    let Err(Error::NewAccess(new)) = result else {
        panic!("the update went through: {result:?}");
    };
    assert_eq!(new, [Grant::Window]);
}

/// The way to pass this check must not be breaking the installed manifest.
#[test]
fn a_broken_installed_manifest_counts_as_reaching_nothing() {
    let root = tempfile::tempdir().unwrap();
    let history = tempfile::tempdir().unwrap();
    let (index, source) = setup_asking("1.0.0", r#"{"process":true}"#, SITE);
    let entry = &index.presences[0];
    install(
        root.path(),
        history.path(),
        &key(),
        &index,
        entry,
        &source,
        &[],
    )
    .unwrap();
    fs::write(root.path().join("demo").join("manifest.json"), "not json").unwrap();

    let (newer, source) = setup_asking("1.1.0", r#"{"process":true}"#, SITE);
    let entry = &newer.presences[0];
    let result = install(
        root.path(),
        history.path(),
        &key(),
        &newer,
        entry,
        &source,
        &[],
    );
    assert!(matches!(result, Err(Error::NewAccess(_))), "{result:?}");
}
