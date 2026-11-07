//! Publisher behavior against a Discord that records what it receives.

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use rpp_core::policy::PresencePreferences;
use rpp_core::{Activity, Kind};
use rpp_discord::frame::{Opcode, encode, read_frame};
use serde_json::{Value, json};
use tokio::io::{AsyncWriteExt, DuplexStream, duplex};

use super::*;

/// What the fake Discord received: the application id and the activity.
type Received = Arc<Mutex<Vec<(String, Value)>>>;

#[derive(Clone)]
struct Wire {
    received: Received,
    up: Arc<AtomicBool>,
    /// The open connection is gone: the next command gets no answer.
    cut: Arc<AtomicBool>,
}

/// An application the fake Discord does not know.
const UNKNOWN_APPLICATION: &str = "11111111111111111";

impl Wire {
    fn new(up: bool) -> Self {
        Self {
            received: Received::default(),
            up: Arc::new(AtomicBool::new(up)),
            cut: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Discord closes, as when the user quits it or the computer sleeps.
    fn close(&self) {
        self.up.store(false, Ordering::SeqCst);
        self.cut.store(true, Ordering::SeqCst);
    }

    /// Discord starts again; connections opened from now on work.
    fn open(&self) {
        self.up.store(true, Ordering::SeqCst);
        self.cut.store(false, Ordering::SeqCst);
    }

    fn sent(&self) -> Vec<(String, Value)> {
        self.received.lock().unwrap().clone()
    }

    fn details(&self) -> Vec<Option<String>> {
        self.sent()
            .iter()
            .map(|(_, activity)| activity["details"].as_str().map(str::to_owned))
            .collect()
    }
}

impl Connector for Wire {
    type Io = DuplexStream;

    async fn connect(&self) -> io::Result<DuplexStream> {
        if !self.up.load(Ordering::SeqCst) {
            return Err(io::ErrorKind::NotFound.into());
        }
        let (ours, mut theirs) = duplex(8192);
        let received = Arc::clone(&self.received);
        let cut = Arc::clone(&self.cut);
        tokio::spawn(async move {
            let Ok((_, hello)) = read_frame(&mut theirs).await else {
                return;
            };
            let client_id = hello["client_id"].as_str().unwrap_or_default().to_owned();
            if client_id == UNKNOWN_APPLICATION {
                let refusal = json!({"code": 4000, "message": "Invalid Client ID"});
                let _ = theirs.write_all(&encode(Opcode::Close, &refusal)).await;
                return;
            }
            let ready = encode(Opcode::Frame, &json!({"evt": "READY"}));
            if theirs.write_all(&ready).await.is_err() {
                return;
            }
            while let Ok((_, command)) = read_frame(&mut theirs).await {
                if cut.load(Ordering::SeqCst) {
                    return;
                }
                let activity = command["args"]["activity"].clone();
                received.lock().unwrap().push((client_id.clone(), activity));
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

fn candidate(source: &str, kind: Kind, details: &str, now: Instant) -> Candidate {
    Candidate {
        source: source.into(),
        label: source.into(),
        client_id: format!("{:018}", source.len()),
        activity: Activity {
            details: Some(details.into()),
            ..Activity::default()
        },
        kind,
        foreground: false,
        paused: false,
        hide_when_paused: true,
        domains: Vec::new(),
        application: String::new(),
        expires_at: Some(now + Duration::from_secs(60)),
    }
}

fn offer(candidate: Candidate) -> Message {
    Message::Candidate(Box::new(candidate))
}

fn publisher(wire: &Wire) -> (Publisher<Wire>, watch::Receiver<Status>) {
    let (status, receiver) = watch::channel(Status::default());
    (
        Publisher::new(wire.clone(), Preferences::default(), status),
        receiver,
    )
}

fn at(base: Instant, seconds: u64) -> Instant {
    base + Duration::from_secs(seconds)
}

#[tokio::test]
async fn the_higher_ranked_source_is_published_with_its_application_id() {
    let wire = Wire::new(true);
    let (mut publisher, status) = publisher(&wire);
    let base = Instant::now();
    publisher.apply(offer(candidate("Song", Kind::Music, "Track 1", base)));
    publisher.apply(offer(candidate("Game", Kind::Game, "Level 1", base)));
    publisher.refresh(base).await;

    assert_eq!(wire.details(), [Some("Level 1".into())]);
    assert_eq!(wire.sent()[0].0, format!("{:018}", "Game".len()));
    let status = status.borrow();
    assert_eq!(status.winner.as_deref(), Some("Game"));
    assert_eq!(status.rows.len(), 2);
}

#[tokio::test]
async fn refreshing_an_unchanged_source_sends_nothing() {
    let wire = Wire::new(true);
    let (mut publisher, _) = publisher(&wire);
    let base = Instant::now();
    publisher.apply(offer(candidate("Game", Kind::Game, "Level 1", base)));
    publisher.refresh(base).await;
    for step in 1..5 {
        let now = at(base, step * 3);
        publisher.apply(offer(candidate("Game", Kind::Game, "Level 1", now)));
        publisher.refresh(now).await;
    }
    assert_eq!(wire.sent().len(), 1);
}

#[tokio::test]
async fn updates_closer_than_the_minimum_gap_wait_their_turn() {
    let wire = Wire::new(true);
    let (mut publisher, _) = publisher(&wire);
    let base = Instant::now();
    publisher.apply(offer(candidate("Game", Kind::Game, "Level 1", base)));
    publisher.refresh(base).await;
    publisher.apply(offer(candidate("Game", Kind::Game, "Level 2", base)));
    publisher.refresh(at(base, 1)).await;
    assert_eq!(wire.details(), [Some("Level 1".into())]);

    publisher.refresh(at(base, 5)).await;
    assert_eq!(
        wire.details(),
        [Some("Level 1".into()), Some("Level 2".into())]
    );
}

#[tokio::test]
async fn a_silent_source_is_dropped_and_the_activity_cleared() {
    let wire = Wire::new(true);
    let (mut publisher, status) = publisher(&wire);
    let base = Instant::now();
    publisher.apply(offer(candidate("Game", Kind::Game, "Level 1", base)));
    publisher.refresh(base).await;
    publisher.refresh(at(base, 61)).await;

    assert_eq!(wire.details(), [Some("Level 1".into()), None]);
    assert_eq!(status.borrow().winner, None);
    assert_eq!(status.borrow().source_errors["Game"], SILENT);
}

#[tokio::test]
async fn a_failed_source_withdraws_its_activity() {
    let wire = Wire::new(true);
    let (mut publisher, status) = publisher(&wire);
    let base = Instant::now();
    publisher.apply(offer(candidate("Game", Kind::Game, "Level 1", base)));
    publisher.refresh(base).await;
    publisher.apply(Message::Failed {
        source: "Game".into(),
        error: "script error: boom".into(),
    });
    publisher.refresh(at(base, 1)).await;
    assert_eq!(wire.details(), [Some("Level 1".into()), None]);
    assert_eq!(status.borrow().source_errors["Game"], "script error: boom");
}

#[tokio::test]
async fn an_error_of_any_size_is_cut_before_it_is_kept() {
    let wire = Wire::new(true);
    let (mut publisher, status) = publisher(&wire);
    publisher.apply(Message::Failed {
        source: "Game".into(),
        error: "é".repeat(6_000_000),
    });
    publisher.refresh(Instant::now()).await;
    assert_eq!(
        status.borrow().source_errors["Game"].chars().count(),
        MAX_ERROR
    );
}

#[tokio::test]
async fn an_error_leaves_the_status_when_the_source_recovers_or_goes_quiet() {
    let wire = Wire::new(true);
    let (mut publisher, status) = publisher(&wire);
    let base = Instant::now();
    let failed = || Message::Failed {
        source: "Game".into(),
        error: "script error: boom".into(),
    };
    publisher.apply(failed());
    publisher.refresh(base).await;
    assert!(status.borrow().source_errors.contains_key("Game"));

    // The same error again changes nothing; going quiet clears it.
    publisher.apply(failed());
    assert!(!publisher.dirty);
    publisher.apply(Message::Clear {
        source: "Game".into(),
    });
    publisher.refresh(at(base, 1)).await;
    assert!(status.borrow().source_errors.is_empty());

    // An error followed by a report clears it as well.
    publisher.apply(failed());
    publisher.refresh(at(base, 2)).await;
    publisher.apply(offer(candidate("Game", Kind::Game, "Level 1", base)));
    publisher.refresh(at(base, 3)).await;
    assert!(status.borrow().source_errors.is_empty());
    assert_eq!(status.borrow().winner.as_deref(), Some("Game"));
}

#[tokio::test]
async fn pausing_clears_and_resuming_publishes_again() {
    let wire = Wire::new(true);
    let (mut publisher, _) = publisher(&wire);
    let base = Instant::now();
    publisher.apply(offer(candidate("Game", Kind::Game, "Level 1", base)));
    publisher.refresh(base).await;

    let mut paused = Preferences::default();
    paused.global.paused = true;
    publisher.apply(Message::Preferences(Box::new(paused)));
    publisher.refresh(at(base, 1)).await;
    publisher.apply(Message::Preferences(Box::default()));
    publisher.refresh(at(base, 2)).await;
    assert_eq!(wire.details(), [Some("Level 1".into()), None]);
    publisher.refresh(at(base, 6)).await;
    assert_eq!(
        wire.details(),
        [Some("Level 1".into()), None, Some("Level 1".into())]
    );
}

#[tokio::test]
async fn going_idle_clears_and_coming_back_publishes_again() {
    let wire = Wire::new(true);
    let (mut publisher, status) = publisher(&wire);
    let base = Instant::now();
    publisher.apply(offer(candidate("Game", Kind::Game, "Level 1", base)));
    publisher.refresh(base).await;

    publisher.apply(Message::Idle(true));
    publisher.refresh(at(base, 5)).await;
    assert_eq!(wire.details(), [Some("Level 1".into()), None]);
    assert_eq!(
        status.borrow().rows[0].reason,
        rpp_core::policy::IDLE_REASON
    );
    // Saying it again changes nothing and sends nothing.
    publisher.apply(Message::Idle(true));
    publisher.refresh(at(base, 10)).await;
    assert_eq!(wire.details().len(), 2);

    publisher.apply(Message::Idle(false));
    publisher.refresh(at(base, 15)).await;
    assert_eq!(
        wire.details(),
        [Some("Level 1".into()), None, Some("Level 1".into())]
    );
}

#[tokio::test]
async fn reconnecting_clears_the_old_client_and_publishes_again() {
    let wire = Wire::new(true);
    let (mut publisher, _) = publisher(&wire);
    let base = Instant::now();
    publisher.apply(offer(candidate("Game", Kind::Game, "Level 1", base)));
    publisher.refresh(base).await;
    publisher.apply(Message::Reconnect);
    publisher.refresh(at(base, 1)).await;
    // Cleared on the old connection, then shown again on the new one.
    assert_eq!(
        wire.details(),
        [Some("Level 1".into()), None, Some("Level 1".into())]
    );
}

#[tokio::test]
async fn hidden_fields_never_reach_discord() {
    let wire = Wire::new(true);
    let (mut publisher, status) = publisher(&wire);
    let mut preferences = Preferences::default();
    preferences.presences.insert(
        "Game".into(),
        PresencePreferences {
            show_title: false,
            ..Default::default()
        },
    );
    publisher.apply(Message::Preferences(Box::new(preferences)));
    let base = Instant::now();
    let mut item = candidate("Game", Kind::Game, "Secret level", base);
    item.activity.state = Some("Secret state".into());
    publisher.apply(offer(item));
    publisher.refresh(base).await;
    let sent = wire.sent();
    assert_eq!(sent[0].1["details"], "Game");
    assert!(sent[0].1.get("state").is_none());
    // The interface shows what was sent, never the hidden original.
    let shown = status.borrow().activity.clone().unwrap();
    assert_eq!(shown.details.as_deref(), Some("Game"));
    assert_eq!(shown.state, None);
}

#[tokio::test]
async fn an_unreachable_discord_is_retried_with_a_wait_and_then_recovers() {
    let wire = Wire::new(false);
    let (mut publisher, status) = publisher(&wire);
    let base = Instant::now();
    publisher.apply(offer(candidate("Game", Kind::Game, "Level 1", base)));
    publisher.refresh(base).await;
    assert!(status.borrow().delivery_error.is_some());
    assert!(wire.sent().is_empty());

    wire.up.store(true, Ordering::SeqCst);
    publisher.refresh(at(base, 1)).await;
    assert!(wire.sent().is_empty(), "still inside the retry wait");
    publisher.refresh(at(base, 5)).await;
    assert_eq!(wire.details(), [Some("Level 1".into())]);
    assert_eq!(status.borrow().delivery_error, None);
}

#[tokio::test]
async fn stopping_removes_the_activity() {
    let wire = Wire::new(true);
    let (mut publisher, _) = publisher(&wire);
    let base = Instant::now();
    publisher.apply(offer(candidate("Game", Kind::Game, "Level 1", base)));
    publisher.refresh(base).await;
    publisher.finish().await;
    assert_eq!(wire.details(), [Some("Level 1".into()), None]);
}

#[tokio::test]
async fn the_run_loop_processes_messages_and_ends_on_stop() {
    let wire = Wire::new(true);
    let (publisher, status) = publisher(&wire);
    let (messages, receiver) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        let mut receiver = receiver;
        publisher.run(&mut receiver).await;
    });
    let now = Instant::now();
    messages
        .send(offer(candidate("Game", Kind::Game, "Level 1", now)))
        .unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(status.borrow().winner.as_deref(), Some("Game"));
    messages.send(Message::Stop).unwrap();
    task.await.unwrap();
    assert_eq!(wire.details(), [Some("Level 1".into()), None]);
}

#[tokio::test]
async fn presence_health_reaches_the_status_and_clears() {
    let wire = Wire::new(true);
    let (mut publisher, status) = publisher(&wire);
    let base = Instant::now();
    publisher.apply(Message::Health {
        source: "Video".into(),
        blank: true,
    });
    publisher.refresh(base).await;
    assert!(status.borrow().blank_sources.contains("Video"));
    publisher.apply(Message::Health {
        source: "Video".into(),
        blank: false,
    });
    publisher.refresh(at(base, 1)).await;
    assert!(status.borrow().blank_sources.is_empty());
}

fn with_application(source: &str, client_id: &str, kind: Kind, now: Instant) -> Message {
    let mut offered = candidate(source, kind, source, now);
    offered.client_id = client_id.into();
    offer(offered)
}

#[tokio::test]
async fn an_unset_application_never_reaches_discord_and_blocks_nobody() {
    let wire = Wire::new(true);
    let (mut publisher, status) = publisher(&wire);
    let base = Instant::now();
    publisher.apply(with_application(
        "Program",
        "00000000000000000",
        Kind::Game,
        base,
    ));
    publisher.apply(offer(candidate("Video", Kind::Video, "A video", base)));
    publisher.refresh(base).await;

    assert_eq!(wire.details(), [Some("A video".into())]);
    assert_eq!(status.borrow().winner.as_deref(), Some("Video"));
    assert_eq!(status.borrow().delivery_error, None);
    assert_eq!(status.borrow().source_errors["Program"], NO_APPLICATION);
}

#[tokio::test]
async fn a_refused_application_gives_way_to_the_next_presence_and_stays_out() {
    let wire = Wire::new(true);
    let (mut publisher, status) = publisher(&wire);
    let base = Instant::now();
    publisher.apply(with_application(
        "Program",
        UNKNOWN_APPLICATION,
        Kind::Game,
        base,
    ));
    publisher.apply(offer(candidate("Video", Kind::Video, "A video", base)));
    publisher.refresh(base).await;
    assert!(wire.details().is_empty());
    assert_eq!(status.borrow().source_errors["Program"], NO_APPLICATION);

    // The next pass needs no wait: Discord itself was fine.
    publisher.refresh(base).await;
    assert_eq!(wire.details(), [Some("A video".into())]);
    assert_eq!(status.borrow().winner.as_deref(), Some("Video"));
    assert_eq!(status.borrow().delivery_error, None);

    // Later reports of the same presence do not try again.
    publisher.apply(with_application(
        "Program",
        UNKNOWN_APPLICATION,
        Kind::Game,
        base,
    ));
    publisher.refresh(at(base, 5)).await;
    assert_eq!(status.borrow().winner.as_deref(), Some("Video"));
    assert_eq!(wire.details().len(), 1);
}

/// Nothing changes on our side while Discord is away, so only a check notices.
#[tokio::test]
async fn an_unchanged_activity_comes_back_when_discord_restarts() {
    let wire = Wire::new(true);
    let (mut publisher, status) = publisher(&wire);
    let base = Instant::now();
    let keep = |publisher: &mut Publisher<Wire>, seconds| {
        publisher.apply(offer(candidate(
            "Game",
            Kind::Game,
            "Level 1",
            at(base, seconds),
        )));
    };
    keep(&mut publisher, 0);
    publisher.refresh(base).await;
    assert_eq!(wire.details().len(), 1);

    wire.close();
    keep(&mut publisher, CHECK_EVERY.as_secs());
    publisher.refresh(at(base, CHECK_EVERY.as_secs())).await;
    assert!(
        status.borrow().delivery_error.is_some(),
        "the loss went unnoticed"
    );

    wire.open();
    let mut seconds = CHECK_EVERY.as_secs();
    while wire.details().len() < 2 && seconds < 120 {
        seconds += 5;
        keep(&mut publisher, seconds);
        publisher.refresh(at(base, seconds)).await;
    }
    assert_eq!(
        wire.details(),
        [Some("Level 1".into()), Some("Level 1".into())]
    );
    assert_eq!(status.borrow().delivery_error, None);
}

#[tokio::test]
async fn a_healthy_connection_is_checked_at_a_calm_pace() {
    let wire = Wire::new(true);
    let (mut publisher, _) = publisher(&wire);
    let base = Instant::now();
    publisher.apply(offer(candidate("Game", Kind::Game, "Level 1", base)));
    publisher.refresh(base).await;

    let early = CHECK_EVERY.as_secs() - 1;
    publisher.apply(offer(candidate(
        "Game",
        Kind::Game,
        "Level 1",
        at(base, early),
    )));
    publisher.refresh(at(base, early)).await;
    assert_eq!(wire.details().len(), 1, "checked too soon");

    let due = CHECK_EVERY.as_secs();
    publisher.apply(offer(candidate(
        "Game",
        Kind::Game,
        "Level 1",
        at(base, due),
    )));
    publisher.refresh(at(base, due)).await;
    assert_eq!(wire.details().len(), 2);
}

#[tokio::test]
async fn nothing_is_sent_while_nothing_is_shown() {
    let wire = Wire::new(true);
    let (mut publisher, _) = publisher(&wire);
    publisher.refresh(at(Instant::now(), 60)).await;
    assert!(wire.sent().is_empty());
}
