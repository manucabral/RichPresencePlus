//! A catalog that lives in memory: presences go in, a signed index comes out.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signer, SigningKey};
use rpp_catalog::{Channel, Error, Fetched, Keys, Source};
use serde_json::json;
use sha2::{Digest, Sha256};

/// Commit every published index points at.
pub const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

const SCRIPT: &str = "function read(){return null} function activity(){return null}";

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// What a maintainer would publish: files at a commit and a signed index.
#[derive(Default)]
pub struct Published {
    files: Mutex<HashMap<String, Vec<u8>>>,
    index: Mutex<Option<Vec<u8>>>,
    index_calls: AtomicUsize,
    installs: Mutex<Option<Vec<u8>>>,
}

impl Published {
    /// Creates an empty catalog.
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// The trusted keys that match the signer used by [`Published::publish`].
    pub fn keys() -> Keys {
        let public = STANDARD.encode(SigningKey::from_bytes(&[1; 32]).verifying_key().to_bytes());
        Keys::parse(&[("main", &public)]).unwrap()
    }

    /// Publishes presences and signs a new index.
    ///
    /// Args:
    ///     generated_at: Unix seconds.
    ///     presences: `(name, folder, version)` of each presence.
    pub fn publish(&self, generated_at: u64, presences: &[(&str, &str, &str)]) {
        self.publish_asking(generated_at, presences, "{}");
    }

    /// Like [`Self::publish`], with `permissions` as the JSON `permissions`
    /// object of every manifest.
    pub fn publish_asking(
        &self,
        generated_at: u64,
        presences: &[(&str, &str, &str)],
        permissions: &str,
    ) {
        let mut listed = Vec::new();
        for (name, folder, version) in presences {
            let manifest = format!(
                r#"{{"name":"{name}","version":"{version}","client_id":"12345678901234567","entry":"main.js","permissions":{permissions},"description":"{name} from the catalog"}}"#
            );
            let files: [(&str, &[u8]); 2] = [
                ("manifest.json", manifest.as_bytes()),
                ("main.js", SCRIPT.as_bytes()),
            ];
            let mut map = serde_json::Map::new();
            for (path, bytes) in files {
                map.insert(
                    path.to_owned(),
                    json!({"sha256": digest(bytes), "size": bytes.len()}),
                );
                self.files
                    .lock()
                    .unwrap()
                    .insert(format!("{COMMIT}/{folder}/{path}"), bytes.to_vec());
            }
            listed.push(json!({
                "name": name, "folder": folder, "version": version,
                "description": format!("{name} from the catalog"), "kind": "game",
                "files": map,
            }));
        }
        let payload = json!({
            "schema_version": 1, "channel": "stable", "commit": COMMIT,
            "generated_at": generated_at, "presences": listed,
        })
        .to_string();
        let signature = SigningKey::from_bytes(&[1; 32]).sign(payload.as_bytes());
        let envelope = json!({
            "key_id": "main", "payload": payload,
            "signature": STANDARD.encode(signature.to_bytes()),
        });
        *self.index.lock().unwrap() = Some(envelope.to_string().into_bytes());
    }

    /// Replaces a published file without touching the signed index.
    pub fn tamper(&self, folder: &str, path: &str, bytes: &[u8]) {
        self.files
            .lock()
            .unwrap()
            .insert(format!("{COMMIT}/{folder}/{path}"), bytes.to_vec());
    }

    /// Publishes install counts, as `(folder, installs)` of the stable channel.
    pub fn count_installs(&self, counts: &[(&str, u64)]) {
        let rows: Vec<_> = counts
            .iter()
            .map(|(folder, installs)| json!({"folder": folder, "installs": installs}))
            .collect();
        let stats = json!({"generated_at": 1, "stable": rows});
        *self.installs.lock().unwrap() = Some(stats.to_string().into_bytes());
    }

    /// Makes the index unreachable, as when the network is down.
    pub fn go_offline(&self) {
        *self.index.lock().unwrap() = None;
    }

    /// How many times the index was downloaded.
    pub fn index_downloads(&self) -> usize {
        self.index_calls.load(Ordering::SeqCst)
    }
}

/// A [`Source`] that reads from a shared [`Published`].
pub struct Server(pub Arc<Published>);

impl Source for Server {
    fn index(&self, _: Channel) -> Result<Vec<u8>, Error> {
        self.0.index_calls.fetch_add(1, Ordering::SeqCst);
        self.0
            .index
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| Error::Network("offline".into()))
    }

    fn file(&self, commit: &str, folder: &str, path: &str, _: u64) -> Result<Vec<u8>, Error> {
        let key = format!("{commit}/{folder}/{path}");
        self.0
            .files
            .lock()
            .unwrap()
            .get(&key)
            .cloned()
            .ok_or_else(|| Error::Network(format!("404 {key}")))
    }

    fn installs(&self, _: Option<&str>) -> Result<Fetched, Error> {
        let bytes = self.0.installs.lock().unwrap().clone();
        bytes
            .map(|bytes| Fetched::New { bytes, tag: None })
            .ok_or_else(|| Error::Network("404 stats.json".into()))
    }
}

/// Convenience: a catalog source and its handle for the test to publish through.
pub fn served() -> (Arc<Published>, Box<dyn Source>) {
    let published = Published::new();
    let source: Box<dyn Source> = Box::new(Server(published.clone()));
    (published, source)
}
