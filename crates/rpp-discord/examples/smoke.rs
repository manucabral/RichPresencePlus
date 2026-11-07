//! Publishes a short test activity to the running Discord, then clears it.
//!
//! Usage: `cargo run -p rpp-discord --example smoke -- <client_id>`

use std::time::{Duration, Instant};

use rpp_core::{Activity, ActivityKind};
use rpp_discord::Connection;
use rpp_discord::pipe::PipeConnector;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let Some(client_id) = std::env::args().nth(1) else {
        eprintln!("usage: smoke <client_id>");
        std::process::exit(2);
    };
    let activity = Activity {
        details: Some("Rich Presence Plus R".into()),
        state: Some("Connection test".into()),
        kind: ActivityKind::Playing,
        ..Activity::default()
    }
    .validated()
    .expect("the test activity is valid");

    let mut connection = Connection::new(PipeConnector::new());
    match connection
        .publish(&client_id, Some(&activity), Instant::now())
        .await
    {
        Ok(()) => println!("published; visible for 6 seconds"),
        Err(error) => {
            eprintln!("publish failed: {error}");
            std::process::exit(1);
        }
    }
    tokio::time::sleep(Duration::from_secs(6)).await;
    match connection.publish(&client_id, None, Instant::now()).await {
        Ok(()) => println!("cleared"),
        Err(error) => eprintln!("clear failed: {error}"),
    }
}
