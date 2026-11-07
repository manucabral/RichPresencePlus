//! The one task that owns the Discord connection.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::{Duration, Instant};

use rpp_core::policy::Row;
use rpp_core::{Activity, Backoff, Candidate, Preferences, Quiet, Selector};
use rpp_discord::connection::PublishError;
use rpp_discord::{Connection, Connector, Error};
use serde::Serialize;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::watch;

use crate::message::Message;

/// How often the task wakes up without news, to expire silent sources and to
/// finish a delayed switch.
const TICK: Duration = Duration::from_millis(500);
/// Discord allows about five activity updates per twenty seconds.
const MIN_GAP: Duration = Duration::from_secs(4);
/// Message shown for a source that stopped reporting.
const SILENT: &str = "Presence stopped reporting";
/// How often a shown activity is sent again to learn that Discord is still there.
const CHECK_EVERY: Duration = Duration::from_secs(15);
/// Longest error text kept from a presence. Scripts write it, and it goes to
/// the log and to every status.
const MAX_ERROR: usize = 500;
/// Shortest time between two errors of one presence in the log.
const LOG_EVERY: Duration = Duration::from_secs(60);
/// What a source hears when its Discord application cannot be used.
const NO_APPLICATION: &str = "This presence needs a valid Discord application ID";

/// Tells whether an application ID was left unset. Discord would refuse it.
fn unset_application(client_id: &str) -> bool {
    client_id.bytes().all(|byte| byte == b'0')
}

/// What the interface shows about publishing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Status {
    /// Source on display.
    pub winner: Option<String>,
    /// What was sent to Discord for it, after the privacy rules.
    pub activity: Option<Activity>,
    /// Why each source was or was not chosen.
    pub rows: Vec<Row>,
    /// Last problem talking to Discord, cleared by the next success.
    pub delivery_error: Option<String>,
    /// Last problem of each source.
    pub source_errors: BTreeMap<String, String>,
    /// Sources whose open pages give them nothing to read, as a changed site does.
    pub blank_sources: BTreeSet<String>,
    /// User name of the Discord account, once a handshake told it.
    #[serde(skip)]
    pub discord_user: Option<String>,
}

/// Compare two candidates ignoring when they expire.
fn same_content(a: &Candidate, b: &Candidate) -> bool {
    // Compared in place: this runs for every report of every presence. The
    // pattern names each field, so adding one to `Candidate` fails to compile
    // here until it is compared too.
    let Candidate {
        source,
        label,
        client_id,
        activity,
        kind,
        foreground,
        paused,
        hide_when_paused,
        domains,
        application,
        expires_at: _,
    } = a;
    *source == b.source
        && *label == b.label
        && *client_id == b.client_id
        && *activity == b.activity
        && *kind == b.kind
        && *foreground == b.foreground
        && *paused == b.paused
        && *hide_when_paused == b.hide_when_paused
        && *domains == b.domains
        && *application == b.application
}

/// State and behavior of the publishing task.
pub struct Publisher<C: Connector> {
    connection: Connection<C>,
    candidates: HashMap<String, Candidate>,
    source_errors: BTreeMap<String, String>,
    /// Keeps a script that fails with a new text each time out of the log.
    logged: BTreeMap<String, Quiet>,
    blank_sources: BTreeSet<String>,
    /// Applications Discord said it does not know.
    refused: BTreeSet<String>,
    preferences: Preferences,
    selector: Selector,
    retry: Backoff,
    status: watch::Sender<Status>,
    delivery_error: Option<String>,
    shown: Option<(String, Activity)>,
    last_publish: Option<Instant>,
    dirty: bool,
    /// Set by [`Message::Reconnect`]; handled at the next refresh.
    reconnect: bool,
}

impl<C: Connector> Publisher<C> {
    /// Creates the task state; nothing is sent until `run`.
    ///
    /// Args:
    ///     status: Where the explanation of each selection is published.
    pub fn new(connector: C, preferences: Preferences, status: watch::Sender<Status>) -> Self {
        Self {
            connection: Connection::new(connector),
            candidates: HashMap::new(),
            source_errors: BTreeMap::new(),
            logged: BTreeMap::new(),
            blank_sources: BTreeSet::new(),
            refused: BTreeSet::new(),
            preferences,
            selector: Selector::default(),
            retry: Backoff::new(Duration::from_secs(2), Duration::from_secs(30)),
            status,
            delivery_error: None,
            shown: None,
            last_publish: None,
            dirty: true,
            reconnect: false,
        }
    }

    /// Records one message. False when the task should end.
    pub fn apply(&mut self, message: Message) -> bool {
        match message {
            Message::Candidate(candidate)
                if unset_application(&candidate.client_id)
                    || self.refused.contains(&candidate.client_id) =>
            {
                self.withhold(candidate.source);
            }
            Message::Candidate(candidate) => {
                self.dirty |= self.source_errors.remove(&candidate.source).is_some();
                match self.candidates.get_mut(&candidate.source) {
                    Some(known) if same_content(known, &candidate) => {
                        known.expires_at = candidate.expires_at;
                    }
                    _ => {
                        self.candidates.insert(candidate.source.clone(), *candidate);
                        self.dirty = true;
                    }
                }
            }
            Message::Clear { source } => {
                // A cleared error has to leave the status too, not only a candidate.
                let had_error = self.source_errors.remove(&source).is_some();
                let had_candidate = self.candidates.remove(&source).is_some();
                self.dirty |= had_error || had_candidate;
            }
            Message::Failed { source, error } => {
                // A script that keeps failing reports the same error at every
                // retry; only a change needs a new selection.
                let error: String = error.chars().take(MAX_ERROR).collect();
                let removed = self.candidates.remove(&source).is_some();
                let repeated = self.source_errors.get(&source) == Some(&error);
                let quiet = self
                    .logged
                    .entry(source.clone())
                    .or_insert(Quiet::new(LOG_EVERY));
                if !repeated && quiet.ready(Instant::now()) {
                    tracing::warn!(presence = %source, "{error}");
                }
                self.source_errors.insert(source, error);
                self.dirty |= removed || !repeated;
            }
            Message::Health { source, blank } => {
                self.dirty |= if blank {
                    self.blank_sources.insert(source)
                } else {
                    self.blank_sources.remove(&source)
                };
            }
            Message::Preferences(preferences) => {
                self.preferences = *preferences;
                self.dirty = true;
            }
            Message::Idle(idle) => {
                self.dirty |= self.selector.set_idle(idle);
            }
            Message::Quiet(quiet) => {
                self.dirty |= self.selector.set_quiet(quiet);
            }
            Message::Reconnect => {
                self.reconnect = true;
                self.dirty = true;
            }
            Message::Stop => return false,
        }
        true
    }

    /// Drop sources that stopped reporting.
    fn expire(&mut self, now: Instant) {
        let before = self.candidates.len();
        let mut silent = Vec::new();
        self.candidates.retain(|source, candidate| {
            let alive = candidate.expires_at.is_none_or(|at| now < at);
            if !alive {
                silent.push(source.clone());
            }
            alive
        });
        for source in silent {
            tracing::info!(presence = %source, "{SILENT}");
            self.source_errors.insert(source, SILENT.into());
        }
        self.dirty |= self.candidates.len() != before;
    }

    /// Keeps a source out of the selection, with its reason on show.
    fn withhold(&mut self, source: String) {
        let removed = self.candidates.remove(&source).is_some();
        let new = self.source_errors.get(&source).map(String::as_str) != Some(NO_APPLICATION);
        if new {
            tracing::warn!(presence = %source, "{NO_APPLICATION}");
            self.source_errors.insert(source, NO_APPLICATION.to_owned());
        }
        self.dirty |= removed || new;
    }

    /// Chooses and publishes if anything changed or a retry is due.
    pub async fn refresh(&mut self, now: Instant) {
        if std::mem::take(&mut self.reconnect) {
            // The old client must not keep showing an activity nobody updates.
            if let Some((client_id, _)) = self.shown.take()
                && let Err(error) = self.connection.publish(&client_id, None, now).await
            {
                tracing::debug!("could not clear the activity before reconnecting: {error}");
            }
            self.connection.disconnect();
            self.last_publish = None;
        }
        self.expire(now);
        self.confirm_shown(now).await;
        let retrying = self.delivery_error.is_some() && self.retry.ready(now);
        if !(self.dirty || self.selector.has_pending() || retrying) {
            return;
        }
        self.dirty = false;

        let mut offered: Vec<Candidate> = self.candidates.values().cloned().collect();
        offered.sort_by(|a, b| a.source.cmp(&b.source));
        let selection = self.selector.select(&offered, &self.preferences, now);

        let shown = selection.activity.clone();
        let wanted = match (&selection.winner, selection.activity) {
            (Some(winner), Some(activity)) => {
                let client_id = self.candidates[winner].client_id.clone();
                Some((client_id, activity))
            }
            _ => None,
        };
        self.deliver(wanted, now).await;
        if selection.winner != self.status.borrow().winner {
            tracing::debug!(winner = ?selection.winner, "the activity on display changed");
        }
        self.status.send_replace(Status {
            winner: selection.winner,
            activity: shown,
            rows: selection.rows,
            delivery_error: self.delivery_error.clone(),
            source_errors: self.source_errors.clone(),
            blank_sources: self.blank_sources.clone(),
            discord_user: self.connection.user().map(str::to_owned),
        });
    }

    /// Checks that Discord still shows the activity, when a check is due.
    async fn confirm_shown(&mut self, now: Instant) {
        let due = self.last_publish.is_some_and(|at| now >= at + CHECK_EVERY);
        // With a change waiting, the delivery itself is the check.
        if self.dirty || self.shown.is_none() || self.delivery_error.is_some() || !due {
            return;
        }
        self.last_publish = Some(now);
        if let Err(error) = self.connection.confirm(now).await {
            let message = error.to_string();
            tracing::warn!("Discord: {message}");
            self.retry.failure(now);
            self.delivery_error = Some(message);
            self.dirty = true;
        }
    }

    async fn deliver(&mut self, wanted: Option<(String, Activity)>, now: Instant) {
        if wanted == self.shown && self.delivery_error.is_none() {
            return;
        }
        let target = wanted
            .as_ref()
            .map(|(client_id, activity)| (client_id.clone(), Some(activity.clone())))
            .or_else(|| {
                self.shown
                    .as_ref()
                    .map(|(client_id, _)| (client_id.clone(), None))
            });
        let Some((client_id, activity)) = target else {
            return;
        };
        if activity.is_some() && self.last_publish.is_some_and(|at| now < at + MIN_GAP) {
            self.dirty = true;
            return;
        }
        match self
            .connection
            .publish(&client_id, activity.as_ref(), now)
            .await
        {
            Ok(()) => {
                self.retry.success();
                if self.delivery_error.take().is_some() {
                    tracing::info!("Discord accepts activities again");
                }
                self.shown = wanted;
                // Clearing is never delayed, but it counts toward Discord's limit.
                self.last_publish = Some(now);
            }
            Err(PublishError::Waiting(_)) => self.dirty = true,
            Err(PublishError::Discord(Error::InvalidClient)) => {
                let sources: Vec<String> = self
                    .candidates
                    .values()
                    .filter(|candidate| candidate.client_id == client_id)
                    .map(|candidate| candidate.source.clone())
                    .collect();
                self.refused.insert(client_id);
                for source in sources {
                    self.withhold(source);
                }
                self.delivery_error = None;
                self.dirty = true;
            }
            Err(error) => {
                self.retry.failure(now);
                let message = error.to_string();
                if self.delivery_error.as_ref() != Some(&message) {
                    tracing::warn!("Discord: {message}");
                }
                self.delivery_error = Some(message);
            }
        }
    }

    /// Processes messages until told to stop, then removes the activity.
    pub async fn run(mut self, messages: &mut UnboundedReceiver<Message>) {
        let mut ticker = tokio::time::interval(TICK);
        loop {
            tokio::select! {
                message = messages.recv() => {
                    let Some(message) = message else { break };
                    if !self.apply(message) {
                        break;
                    }
                }
                _ = ticker.tick() => {}
            }
            while let Ok(message) = messages.try_recv() {
                if !self.apply(message) {
                    self.finish().await;
                    return;
                }
            }
            self.refresh(Instant::now()).await;
        }
        self.finish().await;
    }

    async fn finish(&mut self) {
        self.candidates.clear();
        self.dirty = true;
        self.refresh(Instant::now()).await;
    }
}

#[cfg(test)]
mod tests;
