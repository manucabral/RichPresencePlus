//! Sending a folder to the real Recycle Bin.
#![cfg(windows)]

use rpp_host::to_recycle_bin;

#[test]
fn a_folder_that_does_not_exist_is_an_error() {
    let folder = tempfile::tempdir().unwrap();
    assert!(to_recycle_bin(&folder.path().join("missing")).is_err());
}

/// Ignored in the usual run: each pass leaves a small folder in the Recycle Bin.
#[test]
#[ignore = "leaves a folder in the Recycle Bin"]
fn a_folder_leaves_its_place() {
    let parent = tempfile::tempdir().unwrap();
    let folder = parent.path().join("rpp recycle test ñ");
    std::fs::create_dir(&folder).unwrap();
    std::fs::write(folder.join("main.js"), "// test").unwrap();

    to_recycle_bin(&folder).unwrap();
    assert!(!folder.exists());
}
