//! Runs presence scripts, chooses one activity and publishes it to Discord.

pub mod message;
pub mod publisher;
pub mod runner;

use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use rpp_core::{Candidate, Preferences};
use rpp_discord::Connector;
use rpp_plugins::{Host, Manifest};
use tokio::sync::{mpsc, watch};

pub use message::Message;
pub use publisher::{Publisher, Status};
pub use runner::Runner;

/// Supplies the pages the browser extension reported for a presence.
pub type PageSource = Arc<dyn Fn(&str) -> serde_json::Value + Send + Sync>;

/// A page source that always reports an empty list, for applications without
/// a browser extension.
pub fn no_pages() -> PageSource {
    Arc::new(|_| serde_json::Value::Array(Vec::new()))
}

/// Supplies the options the user chose for a presence.
pub type SettingsSource = Arc<dyn Fn(&str) -> serde_json::Value + Send + Sync>;

/// A settings source that always reports an empty object, for presences that
/// declare no options.
pub fn no_settings() -> SettingsSource {
    Arc::new(|_| serde_json::json!({}))
}

/// Where a script's `context` comes from at each read.
#[derive(Clone)]
pub struct Inputs {
    /// Source of `context.pages`.
    pub pages: PageSource,
    /// Source of `context.options`.
    pub settings: SettingsSource,
}

impl Inputs {
    /// Inputs that report no pages and no settings.
    pub fn none() -> Self {
        Self {
            pages: no_pages(),
            settings: no_settings(),
        }
    }
}

/// A presence ready to run: its manifest and script source.
#[derive(Debug, Clone)]
pub struct Presence {
    /// Parsed manifest.
    pub manifest: Manifest,
    /// Contents of the script file.
    pub source: String,
    /// Texts for `t(key)`, already in the language the user chose.
    pub strings: rpp_core::locale::Table,
    /// The options the user chose, with defaults filled in.
    pub settings: serde_json::Map<String, serde_json::Value>,
}

/// What a restarted publisher needs to pick up where the last one stopped.
#[derive(Default)]
struct Kept {
    preferences: Preferences,
    offered: HashMap<String, Candidate>,
    /// Whether the computer is idle; a restarted publisher must not show
    /// anything the user meant to hide while away.
    idle: bool,
    /// Whether a rule of the person hides everything; the same goes for it.
    quiet: bool,
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Longest pause before starting a publisher that keeps failing again.
const MAX_RESTART_WAIT: Duration = Duration::from_secs(30);
/// Longest wait for the publisher to clear the activity when the service stops.
pub const PUBLISHER_STOP_WAIT: Duration = Duration::from_secs(8);

/// A publisher that carries on from what the service kept.
fn restore<C: Connector>(connector: C, kept: &Kept, status: watch::Sender<Status>) -> Publisher<C> {
    let mut publisher = Publisher::new(connector, kept.preferences.clone(), status);
    publisher.apply(Message::Idle(kept.idle));
    publisher.apply(Message::Quiet(kept.quiet));
    for candidate in kept.offered.values() {
        publisher.apply(Message::Candidate(Box::new(candidate.clone())));
    }
    publisher
}

/// Runs publishers one after another until one ends normally.
fn supervise<C: Connector + Clone>(
    connector: C,
    mut receiver: mpsc::UnboundedReceiver<Message>,
    status: watch::Sender<Status>,
    kept: Arc<Mutex<Kept>>,
) {
    let mut wait = Duration::from_secs(1);
    loop {
        let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        else {
            return;
        };
        let publisher = restore(connector.clone(), &lock(&kept), status.clone());
        let ran = catch_unwind(AssertUnwindSafe(|| {
            runtime.block_on(publisher.run(&mut receiver));
        }));
        if ran.is_ok() {
            return;
        }
        tracing::error!("the publisher stopped unexpectedly; starting it again");
        drop(runtime);
        thread::sleep(wait);
        wait = (wait * 2).min(MAX_RESTART_WAIT);
    }
}

/// The running application core.
pub struct Service<H> {
    messages: mpsc::UnboundedSender<Message>,
    status: watch::Receiver<Status>,
    host: H,
    pages: PageSource,
    settings: Arc<Mutex<HashMap<String, serde_json::Value>>>,
    runners: Arc<Mutex<HashMap<String, Runner>>>,
    publisher: Mutex<Option<JoinHandle<()>>>,
    /// Disconnects when the publisher thread ends, so stopping can wait with a limit.
    publisher_done: Mutex<Option<std::sync::mpsc::Receiver<()>>>,
    kept: Arc<Mutex<Kept>>,
}

impl<H> std::fmt::Debug for Service<H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Service").finish_non_exhaustive()
    }
}

impl<H: Host + Clone + Send + 'static> Service<H> {
    /// Starts the publisher and one runner per presence.
    ///
    /// Args:
    ///     host: What answers the script capabilities; cloned for every script.
    ///     pages: Where scripts get the pages reported by the browser extension.
    pub fn start<C>(
        connector: C,
        host: H,
        presences: Vec<Presence>,
        pages: PageSource,
        preferences: Preferences,
    ) -> Self
    where
        C: Connector + Clone + Send + 'static,
    {
        let (messages, receiver) = mpsc::unbounded_channel();
        let (status_sender, status) = watch::channel(Status::default());
        let kept = Arc::new(Mutex::new(Kept {
            preferences,
            offered: HashMap::new(),
            idle: false,
            quiet: false,
        }));
        let shared = Arc::clone(&kept);
        let (finished, publisher_done) = std::sync::mpsc::channel::<()>();
        let publisher = thread::Builder::new()
            .name("publisher".into())
            .spawn(move || {
                let _finished = finished;
                supervise(connector, receiver, status_sender, shared);
            })
            .inspect_err(|error| tracing::error!("nothing will be published: {error}"))
            .ok();
        let service = Self {
            messages,
            status,
            host,
            pages,
            settings: Arc::new(Mutex::new(HashMap::new())),
            runners: Arc::new(Mutex::new(HashMap::new())),
            publisher: Mutex::new(publisher),
            publisher_done: Mutex::new(Some(publisher_done)),
            kept,
        };
        for presence in presences {
            service.start_presence(presence);
        }
        service
    }

    /// Starts a presence, replacing a running one with the same name.
    pub fn start_presence(&self, mut presence: Presence) {
        let name = presence.manifest.name.clone();
        let options = serde_json::Value::Object(std::mem::take(&mut presence.settings));
        self.settings
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(name.clone(), options);
        let shared = Arc::clone(&self.settings);
        let settings: SettingsSource = Arc::new(move |name| {
            shared
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .get(name)
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}))
        });
        let inputs = Inputs {
            pages: Arc::clone(&self.pages),
            settings,
        };
        let runner = Runner::spawn(presence, self.host.clone(), inputs, self.messages.clone());
        let replaced = self
            .runners
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(name, runner);
        // Dropping the old runner joins its thread, so do it outside the lock.
        drop(replaced);
    }

    /// Offers an activity that does not come from a script, such as one the user wrote.
    pub fn offer(&self, candidate: Candidate) {
        lock(&self.kept)
            .offered
            .insert(candidate.source.clone(), candidate.clone());
        let _ = self.messages.send(Message::Candidate(Box::new(candidate)));
    }

    /// Withdraws an activity offered with [`Service::offer`].
    pub fn withdraw(&self, source: &str) {
        lock(&self.kept).offered.remove(source);
        let _ = self.messages.send(Message::Clear {
            source: source.to_owned(),
        });
    }

    /// Changes the options a running presence reads.
    ///
    /// Args:
    ///     values: The complete set, defaults included.
    pub fn set_settings(&self, name: &str, values: serde_json::Map<String, serde_json::Value>) {
        if let Some(current) = self
            .settings
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get_mut(name)
        {
            *current = serde_json::Value::Object(values);
        }
        // The script reads them at once, not at its next interval.
        if let Some(runner) = lock(&self.runners).get(name) {
            runner.wake();
        }
    }

    /// A function that makes the named presence read now; an unknown name does
    /// nothing.
    pub fn waker(&self) -> impl Fn(&str) + Send + Sync + 'static {
        let runners = Arc::clone(&self.runners);
        move |name| {
            if let Some(runner) = lock(&runners).get(name) {
                runner.wake();
            }
        }
    }

    /// Stops a presence and removes its activity. True when it was running.
    pub fn stop_presence(&self, name: &str) -> bool {
        let stopped = self
            .runners
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(name);
        let was_running = stopped.is_some();
        drop(stopped);
        self.settings
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(name);
        // A presence that is off is not a broken one.
        let _ = self.messages.send(Message::Health {
            source: name.to_owned(),
            blank: false,
        });
        // The runner may have sent one last candidate before it stopped.
        let _ = self.messages.send(Message::Clear {
            source: name.to_owned(),
        });
        was_running
    }

    /// Returns the names of the presences whose script is running.
    pub fn running(&self) -> Vec<String> {
        let mut names: Vec<String> = lock(&self.runners)
            .iter()
            .filter(|(_, runner)| runner.is_alive())
            .map(|(name, _)| name.clone())
            .collect();
        names.sort();
        names
    }

    /// Returns the latest explanation of what is shown and why.
    pub fn status(&self) -> Status {
        self.status.borrow().clone()
    }

    /// Returns a receiver that wakes when the status changes.
    pub fn watch(&self) -> watch::Receiver<Status> {
        self.status.clone()
    }

    /// Replaces the publishing preferences from the next selection on.
    pub fn set_preferences(&self, preferences: Preferences) {
        lock(&self.kept).preferences = preferences.clone();
        let _ = self
            .messages
            .send(Message::Preferences(Box::new(preferences)));
    }

    /// Clears the activity and connects again at the next publication.
    pub fn reconnect(&self) {
        let _ = self.messages.send(Message::Reconnect);
    }

    /// Hides every activity while the computer is idle, or shows them again.
    pub fn set_idle(&self, idle: bool) {
        let mut kept = lock(&self.kept);
        if kept.idle == idle {
            return;
        }
        kept.idle = idle;
        let _ = self.messages.send(Message::Idle(idle));
    }

    /// Hides every activity while a quiet rule holds, or shows them again.
    pub fn set_quiet(&self, quiet: bool) {
        let mut kept = lock(&self.kept);
        if kept.quiet == quiet {
            return;
        }
        kept.quiet = quiet;
        let _ = self.messages.send(Message::Quiet(quiet));
    }
}

impl<H> Service<H> {
    /// Stops every script, removes the activity and waits for the threads.
    pub fn shutdown(&self) {
        let mut runners: Vec<Runner> = self
            .runners
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .drain()
            .map(|(_, runner)| runner)
            .collect();
        // Tell every script to stop before waiting for any, and give them all
        // one deadline, so the waits overlap instead of adding up.
        for runner in &mut runners {
            runner.request_stop();
        }
        let deadline = std::time::Instant::now() + runner::STOP_WAIT;
        for runner in &mut runners {
            runner.stop_by(deadline);
        }
        drop(runners);
        let _ = self.messages.send(Message::Stop);
        let publisher = lock(&self.publisher).take();
        let done = lock(&self.publisher_done).take();
        if let Some(publisher) = publisher {
            let ended = done.is_none_or(|done| {
                matches!(
                    done.recv_timeout(PUBLISHER_STOP_WAIT),
                    Err(RecvTimeoutError::Disconnected)
                )
            });
            if ended {
                let _ = publisher.join();
            } else {
                tracing::warn!(
                    "Discord did not answer while the activity was cleared; closing without waiting"
                );
            }
        }
    }
}

impl<H> Drop for Service<H> {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod restore_tests {
    use super::*;
    use rpp_core::{Activity, Kind};
    use rpp_testkit::Discord;

    fn kept(idle: bool) -> Kept {
        let candidate = Candidate {
            source: ".manual".into(),
            label: "Studying".into(),
            client_id: "12345678901234567".into(),
            activity: Activity {
                details: Some("Reading".into()),
                ..Activity::default()
            },
            kind: Kind::Manual,
            foreground: false,
            paused: false,
            hide_when_paused: true,
            domains: Vec::new(),
            application: String::new(),
            expires_at: None,
        };
        Kept {
            preferences: Preferences::default(),
            offered: HashMap::from([(candidate.source.clone(), candidate)]),
            idle,
            quiet: false,
        }
    }

    #[tokio::test]
    async fn a_restarted_publisher_stays_hidden_while_idle() {
        let discord = Discord::default();
        let (status, _) = watch::channel(Status::default());
        let mut publisher = restore(discord.clone(), &kept(true), status.clone());
        publisher.refresh(std::time::Instant::now()).await;
        assert!(discord.details().is_empty());

        let mut publisher = restore(discord.clone(), &kept(false), status);
        publisher.refresh(std::time::Instant::now()).await;
        assert_eq!(discord.details(), [Some("Reading".to_owned())]);
    }

    /// A stream must not show the activity because the publisher started over.
    #[tokio::test]
    async fn a_restarted_publisher_stays_hidden_while_a_rule_holds() {
        let discord = Discord::default();
        let (status, _) = watch::channel(Status::default());
        let quiet = Kept {
            quiet: true,
            ..kept(false)
        };
        let mut publisher = restore(discord.clone(), &quiet, status);
        publisher.refresh(std::time::Instant::now()).await;
        assert!(discord.details().is_empty());
    }
}
