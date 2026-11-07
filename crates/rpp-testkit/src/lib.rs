//! Test doubles shared by the workspace: a host with no capabilities, a fake
//! Discord that records what it receives, and a polling helper.

pub mod catalog;
pub mod extension;

use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rpp_discord::Connector;
use rpp_discord::frame::{Opcode, encode, read_frame};
use rpp_plugins::{Host, Request};
use serde_json::{Value, json};
use tokio::io::{AsyncWriteExt, DuplexStream, duplex};
use tokio::time::sleep;

/// A host that offers no capability.
#[derive(Clone, Copy, Debug)]
pub struct Silent;

impl Host for Silent {
    fn call(&self, _: Request) -> Result<Value, String> {
        Err("no host".into())
    }
}

/// A host that answers the process list with fixed names.
#[derive(Clone, Debug)]
pub struct Processes(pub Vec<String>);

impl Host for Processes {
    fn call(&self, request: Request) -> Result<Value, String> {
        match request {
            Request::ProcessList => Ok(Value::from(self.0.clone())),
            _ => Err("no host".into()),
        }
    }
}

/// Like [`Processes`], and counts how many times the list was asked for. Listing
/// every process is what costs, so a test can say how often it happens.
#[derive(Clone, Debug)]
pub struct CountedProcesses {
    names: Vec<String>,
    lists: Arc<AtomicUsize>,
}

impl CountedProcesses {
    /// A host that answers with `names`.
    pub fn new(names: &[&str]) -> Self {
        Self {
            names: names.iter().map(|name| (*name).to_owned()).collect(),
            lists: Arc::default(),
        }
    }

    /// How many times the list was asked for.
    pub fn lists(&self) -> usize {
        self.lists.load(Ordering::SeqCst)
    }
}

impl Host for CountedProcesses {
    fn call(&self, request: Request) -> Result<Value, String> {
        match request {
            Request::ProcessList => {
                self.lists.fetch_add(1, Ordering::SeqCst);
                Ok(Value::from(self.names.clone()))
            }
            _ => Err("no host".into()),
        }
    }
}

/// A Discord that records the activities it is given; `null` is a clear.
#[derive(Clone, Default, Debug)]
pub struct Discord {
    /// The `activity` argument of every `SET_ACTIVITY` received.
    pub activities: Arc<Mutex<Vec<Value>>>,
}

impl Discord {
    /// Returns the `details` of every activity received, `None` for a clear.
    pub fn details(&self) -> Vec<Option<String>> {
        self.activities
            .lock()
            .map(|all| {
                all.iter()
                    .map(|activity| activity["details"].as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl Connector for Discord {
    type Io = DuplexStream;

    async fn connect(&self) -> io::Result<DuplexStream> {
        let (ours, mut theirs) = duplex(8192);
        let activities = Arc::clone(&self.activities);
        tokio::spawn(async move {
            if read_frame(&mut theirs).await.is_err() {
                return;
            }
            let ready = encode(
                Opcode::Frame,
                &json!({"evt": "READY", "data": {"user": {"username": "tester"}}}),
            );
            if theirs.write_all(&ready).await.is_err() {
                return;
            }
            while let Ok((_, command)) = read_frame(&mut theirs).await {
                if let Ok(mut all) = activities.lock() {
                    all.push(command["args"]["activity"].clone());
                }
                let reply = json!({"evt": null, "nonce": command["nonce"]});
                if theirs
                    .write_all(&encode(Opcode::Frame, &reply))
                    .await
                    .is_err()
                {
                    return;
                }
            }
        });
        Ok(ours)
    }
}

/// Checks a condition every 50 ms for up to ten seconds.
///
/// Panics:
///     If it does not hold in time; the message names `what`.
pub async fn eventually(what: &str, condition: impl Fn() -> bool) {
    for _ in 0..200 {
        if condition() {
            return;
        }
        sleep(Duration::from_millis(50)).await;
    }
    panic!("timed out waiting for {what}");
}
