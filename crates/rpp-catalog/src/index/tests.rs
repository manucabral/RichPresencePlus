//! Index verification, plus the helpers other modules use to build signed indexes.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signer, SigningKey};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::*;

/// A signing key fixed for the tests.
pub(crate) fn signer(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

/// The trusted keys that match [`signer`] `1`, under the identifier `main`.
pub(crate) fn keys() -> Keys {
    let public = STANDARD.encode(signer(1).verifying_key().to_bytes());
    Keys::parse(&[("main", &public)]).unwrap()
}

/// Signs payload text and wraps it as the downloaded file.
pub(crate) fn sign(payload: &str, seed: u8, key_id: &str) -> Vec<u8> {
    let signature = signer(seed).sign(payload.as_bytes());
    json!({
        "key_id": key_id,
        "payload": payload,
        "signature": STANDARD.encode(signature.to_bytes()),
    })
    .to_string()
    .into_bytes()
}

pub(crate) const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

pub(crate) fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// An entry that lists the given files with their real digests.
pub(crate) fn entry_for(name: &str, folder: &str, version: &str, files: &[(&str, &[u8])]) -> Value {
    let listed: serde_json::Map<String, Value> = files
        .iter()
        .map(|(path, bytes)| {
            (
                (*path).to_owned(),
                json!({"sha256": digest(bytes), "size": bytes.len()}),
            )
        })
        .collect();
    json!({"name": name, "folder": folder, "version": version, "files": listed})
}

pub(crate) fn payload(channel: &str, presences: Vec<Value>) -> String {
    json!({
        "schema_version": 1, "channel": channel, "commit": COMMIT,
        "generated_at": 1_790_000_000_u64, "presences": presences,
    })
    .to_string()
}

fn one(files: &[(&str, &[u8])]) -> Vec<Value> {
    vec![entry_for("Demo", "demo", "1.2.0", files)]
}

const MANIFEST: &[u8] = b"{}";

#[test]
fn a_signed_index_is_accepted() {
    let text = payload(
        "stable",
        one(&[("manifest.json", MANIFEST), ("main.js", b"x")]),
    );
    let index = Index::verify(&sign(&text, 1, "main"), &keys(), Channel::Stable).unwrap();
    assert_eq!(index.commit, COMMIT);
    assert_eq!(index.presences[0].version, Version::new(1, 2, 0));
    assert_eq!(index.find("DEMO").unwrap().folder, "demo");
    assert!(index.find("missing").is_none());
}

#[test]
fn nothing_is_accepted_without_trusted_keys() {
    let text = payload("stable", one(&[("manifest.json", MANIFEST)]));
    let result = Index::verify(&sign(&text, 1, "main"), &Keys::default(), Channel::Stable);
    assert!(matches!(result, Err(Error::Disabled)));
}

#[test]
fn a_bad_signature_is_refused_whatever_the_reason() {
    let text = payload("stable", one(&[("manifest.json", MANIFEST)]));
    // Signed by a key that is not the trusted one.
    assert!(matches!(
        Index::verify(&sign(&text, 2, "main"), &keys(), Channel::Stable),
        Err(Error::Signature)
    ));
    // Signed by an unknown identifier.
    assert!(matches!(
        Index::verify(&sign(&text, 1, "other"), &keys(), Channel::Stable),
        Err(Error::Signature)
    ));
    // The payload was edited after signing.
    let mut envelope: Value = serde_json::from_slice(&sign(&text, 1, "main")).unwrap();
    envelope["payload"] = Value::String(text.replace("1.2.0", "9.9.9"));
    assert!(matches!(
        Index::verify(envelope.to_string().as_bytes(), &keys(), Channel::Stable),
        Err(Error::Signature)
    ));
    // Not an envelope at all, or a signature that is not base64.
    for junk in [&b"{}"[..], b"not json", b"[]", b""] {
        assert!(matches!(
            Index::verify(junk, &keys(), Channel::Stable),
            Err(Error::Signature)
        ));
    }
    envelope["signature"] = Value::String("***".into());
    assert!(matches!(
        Index::verify(envelope.to_string().as_bytes(), &keys(), Channel::Stable),
        Err(Error::Signature)
    ));
}

fn refused(presences: Vec<Value>, channel: &str, requested: Channel) -> bool {
    let text = payload(channel, presences);
    matches!(
        Index::verify(&sign(&text, 1, "main"), &keys(), requested),
        Err(Error::Invalid(_))
    )
}

#[test]
fn structure_and_limits_are_enforced_even_when_signed() {
    let ok = || entry_for("Demo", "demo", "1.0.0", &[("manifest.json", MANIFEST)]);
    assert!(!refused(vec![ok()], "stable", Channel::Stable));

    // Wrong channel on either side.
    assert!(refused(vec![ok()], "experimental", Channel::Stable));
    assert!(refused(vec![ok()], "stable", Channel::Experimental));

    // Paths that leave the presence folder, or hide.
    for path in [
        "../x.js", "/etc/x", "a//b", "a/../b", ".hidden", "a\\b", "x y.js", "",
    ] {
        let mut entry = ok();
        entry["files"][path] = json!({"sha256": "0".repeat(64), "size": 1});
        assert!(refused(vec![entry], "stable", Channel::Stable), "{path:?}");
    }

    // Names Windows treats as devices, as a folder or as a file.
    for device in ["con", "nul", "com1", "lpt9"] {
        let mut entry = ok();
        entry["folder"] = json!(device);
        assert!(refused(vec![entry], "stable", Channel::Stable), "{device}");
    }
    for path in ["nul.js", "sub/con.txt", "aux", "COM3.json"] {
        let mut entry = ok();
        entry["files"][path] = json!({"sha256": "0".repeat(64), "size": 1});
        assert!(refused(vec![entry], "stable", Channel::Stable), "{path:?}");
    }
    let mut similar = ok();
    similar["folder"] = json!("console");
    similar["files"]["connect.js"] = json!({"sha256": "0".repeat(64), "size": 1});
    assert!(!refused(vec![similar], "stable", Channel::Stable));

    // Folder names that are not one plain segment.
    for folder in ["../x", "A", "a/b", "", "-a", &"a".repeat(65)] {
        let mut entry = ok();
        entry["folder"] = json!(folder);
        assert!(
            refused(vec![entry], "stable", Channel::Stable),
            "{folder:?}"
        );
    }

    // Digests, versions, manifest, duplicates, sizes.
    let mut short = ok();
    short["files"]["manifest.json"]["sha256"] = json!("abc");
    let mut upper = ok();
    upper["files"]["manifest.json"]["sha256"] = json!("A".repeat(64));
    let mut huge = ok();
    huge["files"]["manifest.json"]["size"] = json!(MAX_FILE + 1);
    let mut loose = ok();
    loose["version"] = json!("1");
    let mut pre = ok();
    pre["version"] = json!("2.0.0-rc.1");
    let mut no_manifest = ok();
    no_manifest["files"] = json!({"main.js": {"sha256": "0".repeat(64), "size": 1}});
    let mut empty = ok();
    empty["files"] = json!({});
    for bad in [short, upper, huge, loose, pre, no_manifest, empty] {
        assert!(refused(vec![bad], "stable", Channel::Stable));
    }
    let twin = entry_for("demo", "other", "1.0.0", &[("manifest.json", MANIFEST)]);
    assert!(refused(vec![ok(), twin], "stable", Channel::Stable));
    let same_folder = entry_for("Other", "demo", "1.0.0", &[("manifest.json", MANIFEST)]);
    assert!(refused(vec![ok(), same_folder], "stable", Channel::Stable));
}

#[test]
fn pre_releases_are_allowed_on_the_experimental_channel() {
    let mut beta = entry_for("Demo", "demo", "2.0.0-rc.1", &[("manifest.json", MANIFEST)]);
    beta["description"] = json!("A beta");
    let text = payload("experimental", vec![beta]);
    let index = Index::verify(&sign(&text, 1, "main"), &keys(), Channel::Experimental).unwrap();
    assert!(!index.presences[0].version.pre.is_empty());
}

#[test]
fn schema_commit_and_size_are_checked() {
    let entries = one(&[("manifest.json", MANIFEST)]);
    let reshape = |edit: &dyn Fn(&mut Value)| {
        let mut value: Value = serde_json::from_str(&payload("stable", entries.clone())).unwrap();
        edit(&mut value);
        matches!(
            Index::verify(
                &sign(&value.to_string(), 1, "main"),
                &keys(),
                Channel::Stable
            ),
            Err(Error::Invalid(_))
        )
    };
    assert!(reshape(&|v| v["schema_version"] = json!(2)));
    assert!(reshape(&|v| v["commit"] = json!("main")));
    assert!(reshape(&|v| v["commit"] = json!("A".repeat(40))));
    assert!(reshape(&|v| v["generated_at"] = json!("yesterday")));
    assert!(reshape(&|v| v["presences"] = json!("none")));

    let big = vec![b' '; (MAX_INDEX + 1) as usize];
    assert!(matches!(
        Index::verify(&big, &keys(), Channel::Stable),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn keys_must_be_valid() {
    assert!(Keys::parse(&[("a", "not base64!")]).is_err());
    assert!(Keys::parse(&[("a", "AAAA")]).is_err());
    assert!(Keys::parse(&[]).unwrap().is_empty());
}

#[test]
fn what_is_shown_next_to_a_presence_is_cleaned_not_refused() {
    let mut entry = entry_for("Demo", "demo", "1.0.0", &[("manifest.json", MANIFEST)]);
    entry["author"] = json!("Ana");
    entry["color"] = json!("#A56786");
    entry["tags"] = json!(["video", "Bad Tag", "invented", "music"]);
    entry["icon"] = json!("https://img.example.com/a.png");
    let text = payload("stable", vec![entry]);
    let index = Index::verify(&sign(&text, 1, "main"), &keys(), Channel::Stable).unwrap();
    let shown = &index.presences[0];
    assert_eq!(shown.author.as_deref(), Some("Ana"));
    assert_eq!(shown.color.as_deref(), Some("#A56786"));
    assert_eq!(shown.tags, ["video", "music"]);
    assert_eq!(shown.icon.as_deref(), Some("https://img.example.com/a.png"));

    // Unusable values are dropped and the presence is still listed.
    let mut messy = entry_for("Demo", "demo", "1.0.0", &[("manifest.json", MANIFEST)]);
    messy["author"] = json!("a\u{7}b");
    messy["color"] = json!("red");
    messy["icon"] = json!("http://insecure.example.com/a.png");
    let text = payload("stable", vec![messy]);
    let index = Index::verify(&sign(&text, 1, "main"), &keys(), Channel::Stable).unwrap();
    let shown = &index.presences[0];
    assert_eq!(
        (&shown.author, &shown.color, &shown.icon),
        (&None, &None, &None)
    );
}

#[test]
fn bundles_are_spread_over_sixteen_releases_per_channel() {
    // The same values tools/catalog.mjs computes (first hex digit of SHA-256).
    assert_eq!(Channel::Stable.release("youtube"), "catalog-files-2");
    assert_eq!(Channel::Stable.release("netflix"), "catalog-files-1");
    assert_eq!(
        Channel::Experimental.release("sa-mp"),
        "catalog-files-experimental-2"
    );
    let tags: std::collections::BTreeSet<String> = (0..2000)
        .map(|n| Channel::Stable.release(&format!("p{n}")))
        .collect();
    assert_eq!(tags.len(), 16);
}

mod random_input {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn a_tampered_response_is_refused(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
            prop_assert!(Index::verify(&bytes, &keys(), Channel::Stable).is_err());
        }

        #[test]
        fn a_made_up_signature_is_refused(payload in "\\PC{0,200}", signature in "[A-Za-z0-9+/=]{0,120}") {
            let envelope = serde_json::json!({
                "key_id": "main",
                "payload": payload,
                "signature": signature,
            });
            prop_assert!(Index::verify(envelope.to_string().as_bytes(), &keys(), Channel::Stable).is_err());
        }

        #[test]
        fn no_contained_path_leaves_the_folder(path in "[a-zA-Z0-9._/\\\\:-]{0,40}") {
            if contained(&path) {
                prop_assert!(!path.split('/').any(|part| part == ".." || part.is_empty()));
                prop_assert!(!path.contains('\\') && !path.contains(':'));
                prop_assert!(!path.starts_with('/'));
            }
        }
    }
}
