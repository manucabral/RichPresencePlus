//! A catalog index. The text is signed here with a trusted key, so the search goes past
//! the signature into what the application checks of a signed index.
#![no_main]

use std::sync::LazyLock;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signer, SigningKey};
use libfuzzer_sys::fuzz_target;
use rpp_catalog::{Channel, Index, Keys};

static SIGNER: LazyLock<SigningKey> = LazyLock::new(|| SigningKey::from_bytes(&[1; 32]));
static KEYS: LazyLock<Keys> = LazyLock::new(|| {
    let public = STANDARD.encode(SIGNER.verifying_key().to_bytes());
    Keys::parse(&[("main", &public)]).unwrap()
});

fuzz_target!(|payload: &str| {
    let signature = STANDARD.encode(SIGNER.sign(payload.as_bytes()).to_bytes());
    let file = serde_json::json!({ "key_id": "main", "payload": payload, "signature": signature })
        .to_string();
    for channel in [Channel::Stable, Channel::Experimental] {
        let Ok(index) = Index::verify(file.as_bytes(), &KEYS, channel) else {
            continue;
        };
        for entry in &index.presences {
            assert!(entry.files.contains_key("manifest.json"));
            assert!(entry.tags.len() <= 8 && entry.other_names.len() <= 8);
            assert!(entry.descriptions.len() <= 16);
            assert!(channel == Channel::Experimental || entry.version.pre.is_empty());
        }
    }
    // The same text unsigned, as anyone on the way could send it.
    let _ = Index::verify(payload.as_bytes(), &KEYS, Channel::Stable);
});
