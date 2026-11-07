//! An index signed by `tools/catalog.mjs` is accepted by this crate.
//!
//! `fixtures/catalog.json` is the tool's output for the bundled presences,
//! signed with the test key in `fixtures/test-key.pem`, which the application
//! never trusts. To refresh it after changing a presence:
//!
//! ```text
//! node tools/catalog.mjs build --commit 0123456789abcdef0123456789abcdef01234567 \
//!   --key-file crates/rpp-catalog/tests/fixtures/test-key.pem --key-id test \
//!   --out crates/rpp-catalog/tests/fixtures/catalog.json
//! ```

use std::fs;
use std::path::PathBuf;

use rpp_catalog::{Channel, Error, Index, Keys};

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    fs::read_to_string(path).unwrap()
}

fn keys() -> Keys {
    let public = fixture("public-key.txt");
    Keys::parse(&[("test", public.trim())]).unwrap()
}

#[test]
fn the_tools_output_verifies_and_lists_the_bundled_presences() {
    let index =
        Index::verify(fixture("catalog.json").as_bytes(), &keys(), Channel::Stable).unwrap();
    let folders: Vec<_> = index.presences.iter().map(|p| p.folder.as_str()).collect();
    for expected in [
        "nepu",
        "netflix",
        "ready-or-not",
        "roblox",
        "youtube",
        "youtube-music",
    ] {
        assert!(folders.contains(&expected), "{expected} is missing");
    }
    for entry in &index.presences {
        assert!(
            entry.files.contains_key("manifest.json"),
            "{}",
            entry.folder
        );
        assert!(entry.files.contains_key("main.js"), "{}", entry.folder);
        assert!(
            entry.files.keys().any(|path| path == "locales/en.json"),
            "{} has no English texts",
            entry.folder
        );
    }
}

#[test]
fn the_same_file_is_refused_for_another_channel_or_key() {
    let text = fixture("catalog.json");
    assert!(matches!(
        Index::verify(text.as_bytes(), &keys(), Channel::Experimental),
        Err(Error::Invalid(_))
    ));
    let other = Keys::parse(&[("test", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=")]).unwrap();
    assert!(matches!(
        Index::verify(text.as_bytes(), &other, Channel::Stable),
        Err(Error::Signature)
    ));
}
