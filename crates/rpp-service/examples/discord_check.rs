//! Holds a plain activity on the real Discord, to watch what happens around it.
//!
//! Usage: `discord_check <seconds>`.

use std::time::{Duration, Instant};

use rpp_core::activity::Activity;
use rpp_core::{Candidate, Kind, Preferences};
use rpp_discord::pipe::PipeConnector;
use rpp_service::{Message, Publisher, Status};
use tokio::sync::watch;

/// The application of the presences made for programs.
const APPLICATION: &str = "1567580382377803786";

fn offer() -> Message {
    Message::Candidate(Box::new(Candidate {
        source: "Check".to_owned(),
        label: "Check".to_owned(),
        client_id: APPLICATION.to_owned(),
        activity: Activity {
            details: Some("Rich Presence Plus".to_owned()),
            state: Some("Prueba de conexión".to_owned()),
            ..Activity::default()
        },
        kind: Kind::Game,
        foreground: false,
        paused: false,
        hide_when_paused: false,
        domains: Vec::new(),
        application: "check".to_owned(),
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
    let (sender, receiver) = watch::channel(Status::default());
    let mut publisher = Publisher::new(PipeConnector::new(), Preferences::default(), sender);

    let start = Instant::now();
    let mut last = String::new();
    while start.elapsed() < Duration::from_secs(seconds) {
        publisher.apply(offer());
        publisher.refresh(Instant::now()).await;
        let state = {
            let status = receiver.borrow();
            match &status.delivery_error {
                Some(error) => format!("failing: {error}"),
                None if status.winner.is_some() => "showing".to_owned(),
                None => "waiting".to_owned(),
            }
        };
        if state != last {
            println!("{:6.1}s {state}", start.elapsed().as_secs_f32());
            last = state;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    publisher.apply(Message::Clear {
        source: "Check".to_owned(),
    });
    publisher
        .refresh(Instant::now() + Duration::from_secs(5))
        .await;
}
