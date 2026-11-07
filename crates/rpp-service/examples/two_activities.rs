//! Holds two activities on the real Discord at once, each with its own
//! application.
//!
//! Usage: `two_activities <seconds>`.

use std::time::{Duration, Instant};

use rpp_core::activity::Activity;
use rpp_core::{Candidate, Kind, Preferences};
use rpp_discord::pipe::PipeConnector;
use rpp_service::{Message, Publisher, Status};
use tokio::sync::watch;

/// Two applications the presences of this repository already use.
const APPLICATIONS: [(&str, &str); 2] = [
    ("First", "1113646725408772176"),
    ("Second", "1567580382377803786"),
];

fn offer(source: &str, application: &str) -> Message {
    Message::Candidate(Box::new(Candidate {
        source: source.to_owned(),
        label: source.to_owned(),
        client_id: application.to_owned(),
        activity: Activity {
            details: Some(format!("{source} activity")),
            state: Some("Rich Presence Plus test".to_owned()),
            ..Activity::default()
        },
        kind: Kind::Game,
        foreground: false,
        paused: false,
        hide_when_paused: false,
        domains: Vec::new(),
        application: source.to_owned(),
        expires_at: Some(Instant::now() + Duration::from_secs(60)),
    }))
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let seconds: u64 = std::env::args()
        .nth(1)
        .expect("seconds")
        .parse()
        .expect("a number");
    let mut running = Vec::new();
    for (source, application) in APPLICATIONS {
        let (sender, receiver) = watch::channel(Status::default());
        let publisher = Publisher::new(PipeConnector::new(), Preferences::default(), sender);
        running.push((source, application, publisher, receiver, String::new()));
    }

    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(seconds) {
        for (source, application, publisher, receiver, last) in &mut running {
            publisher.apply(offer(source, application));
            publisher.refresh(Instant::now()).await;
            let state = {
                let status = receiver.borrow();
                match &status.delivery_error {
                    Some(error) => format!("failing: {error}"),
                    None if status.winner.is_some() => "showing".to_owned(),
                    None => "waiting".to_owned(),
                }
            };
            if state != *last {
                println!("{:6.1}s {source}: {state}", start.elapsed().as_secs_f32());
                *last = state;
            }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    for (source, _, publisher, _, _) in &mut running {
        publisher.apply(Message::Clear {
            source: (*source).to_owned(),
        });
        publisher
            .refresh(Instant::now() + Duration::from_secs(5))
            .await;
    }
}
