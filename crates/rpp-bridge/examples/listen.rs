//! A bridge without the application, to try the extension in a real browser.
//!
//! Usage: `listen <data folder> <seconds>`. Prints `listening <port>`, then
//! `connected <browser> <version>`, and exits; with no extension in time it exits with 1.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use rpp_bridge::native::{Pairing, save_pairing};
use rpp_bridge::{Bridge, Config, generate_token};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut args = std::env::args().skip(1);
    let dir = PathBuf::from(args.next().expect("a data folder"));
    let seconds: u64 = args.next().expect("seconds").parse().expect("a number");

    let bridge = Bridge::start(Config::new(0, generate_token()))
        .await
        .expect("a free port");
    let port = bridge.address().port();
    let token = bridge.token().to_owned();
    save_pairing(&dir, &Pairing { port, token }).expect("a writable folder");
    println!("listening {port}");

    let deadline = Instant::now() + Duration::from_secs(seconds);
    while Instant::now() < deadline {
        if let Some(session) = bridge.live_sessions().first() {
            println!(
                "connected {}",
                serde_json::to_string(session).expect("a session")
            );
            bridge.shutdown().await;
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    bridge.shutdown().await;
    std::process::exit(1);
}
