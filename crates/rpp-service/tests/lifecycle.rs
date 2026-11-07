//! Starting, replacing and stopping presences while the service runs.

use rpp_core::{Activity, Candidate, Kind, Preferences};
use rpp_plugins::Manifest;
use rpp_service::{Presence, Service, no_pages};

use rpp_testkit::{Discord, Silent, eventually};

fn presence(details: &str) -> Presence {
    let manifest = Manifest::parse(
        r#"{"name":"Demo","version":"1","client_id":"12345678901234567","interval":1}"#,
    )
    .unwrap();
    let source = format!(
        "function read(){{return {{}}}} function activity(){{return {{details:'{details}'}}}}"
    );
    Presence {
        manifest,
        source,
        strings: Default::default(),
        settings: Default::default(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn presences_can_be_stopped_replaced_and_started_again() {
    let discord = Discord::default();
    let service = Service::start(
        discord.clone(),
        Silent,
        vec![presence("First one")],
        no_pages(),
        Preferences::default(),
    );
    assert_eq!(service.running(), ["Demo"]);
    eventually("the first activity", || {
        discord.details() == [Some("First one".into())]
    })
    .await;

    assert!(service.stop_presence("Demo"));
    assert!(service.running().is_empty());
    assert!(!service.stop_presence("Demo"));
    eventually("the activity to clear", || {
        discord.details().last() == Some(&None)
    })
    .await;

    service.start_presence(presence("Second one"));
    eventually("the second activity", || {
        discord.details().last() == Some(&Some("Second one".into()))
    })
    .await;

    // A presence started under the same name replaces the running one.
    service.start_presence(presence("Third one"));
    assert_eq!(service.running(), ["Demo"]);
    eventually("the third activity", || {
        discord.details().last() == Some(&Some("Third one".into()))
    })
    .await;

    tokio::task::spawn_blocking(move || service.shutdown())
        .await
        .unwrap();
    assert_eq!(discord.details().last(), Some(&None));
}

/// Opens a real loopback socket, which needs the I/O driver of the runtime
/// that runs the publisher (Discord's pipe needs it too).
#[derive(Clone)]
struct Loopback(std::net::SocketAddr);

impl rpp_discord::Connector for Loopback {
    type Io = tokio::net::TcpStream;

    async fn connect(&self) -> std::io::Result<Self::Io> {
        tokio::net::TcpStream::connect(self.0).await
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_publisher_runtime_can_open_sockets() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let service = Service::start(
        Loopback(listener.local_addr().unwrap()),
        Silent,
        vec![presence("Socket")],
        no_pages(),
        Preferences::default(),
    );
    let accepted =
        tokio::time::timeout(std::time::Duration::from_secs(10), listener.accept()).await;
    assert!(accepted.is_ok(), "the publisher never reached the socket");
    drop(service);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_offered_activity_outranks_scripts_until_it_is_withdrawn() {
    let discord = Discord::default();
    let service = Service::start(
        discord.clone(),
        Silent,
        vec![presence("From a script")],
        no_pages(),
        Preferences::default(),
    );
    eventually("the script activity", || {
        discord.details().last() == Some(&Some("From a script".into()))
    })
    .await;

    service.offer(Candidate {
        source: ".manual".into(),
        label: "Studying".into(),
        client_id: "12345678901234567".into(),
        activity: Activity {
            details: Some("Written by hand".into()),
            ..Activity::default()
        },
        kind: Kind::Manual,
        foreground: false,
        paused: false,
        hide_when_paused: true,
        domains: Vec::new(),
        application: String::new(),
        expires_at: None,
    });
    eventually("the manual activity", || {
        discord.details().last() == Some(&Some("Written by hand".into()))
    })
    .await;

    service.withdraw(".manual");
    eventually("the script again", || {
        discord.details().last() == Some(&Some("From a script".into()))
    })
    .await;
    tokio::task::spawn_blocking(move || service.shutdown())
        .await
        .unwrap();
}

/// Panics at the first connection, then behaves like the fake Discord.
#[derive(Clone)]
struct BreaksOnce {
    discord: Discord,
    broken: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl rpp_discord::Connector for BreaksOnce {
    type Io = <Discord as rpp_discord::Connector>::Io;

    async fn connect(&self) -> std::io::Result<Self::Io> {
        if !self.broken.swap(true, std::sync::atomic::Ordering::SeqCst) {
            panic!("a bug in the connection code");
        }
        self.discord.connect().await
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_publisher_that_panics_is_replaced_and_keeps_offered_activities_and_rules() {
    let discord = Discord::default();
    let connector = BreaksOnce {
        discord: discord.clone(),
        broken: Default::default(),
    };
    let service = Service::start(
        connector,
        Silent,
        vec![presence("From a script")],
        no_pages(),
        Preferences::default(),
    );
    service.offer(Candidate {
        source: ".manual".into(),
        label: "Studying".into(),
        client_id: "12345678901234567".into(),
        activity: Activity {
            details: Some("Written by hand".into()),
            ..Activity::default()
        },
        kind: Kind::Manual,
        foreground: false,
        paused: false,
        hide_when_paused: true,
        domains: Vec::new(),
        application: String::new(),
        expires_at: None,
    });
    // The first publisher dies on its first connection; the second one shows
    // the offered activity, which a script would never send again.
    eventually("the activity after the restart", || {
        discord.details().last() == Some(&Some("Written by hand".into()))
    })
    .await;
    tokio::task::spawn_blocking(move || service.shutdown())
        .await
        .unwrap();
}

/// A Discord that accepts nothing and never answers.
#[derive(Clone)]
struct Unanswered;

impl rpp_discord::Connector for Unanswered {
    type Io = tokio::io::DuplexStream;

    async fn connect(&self) -> std::io::Result<Self::Io> {
        std::future::pending().await
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn stopping_does_not_wait_forever_for_a_discord_that_never_answers() {
    let service = Service::start(
        Unanswered,
        Silent,
        vec![presence("Never shown")],
        no_pages(),
        Preferences::default(),
    );
    // Give the publisher time to get stuck connecting.
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    let started = std::time::Instant::now();
    tokio::task::spawn_blocking(move || service.shutdown())
        .await
        .unwrap();
    let waited = started.elapsed();
    let limit = rpp_service::PUBLISHER_STOP_WAIT + std::time::Duration::from_secs(3);
    assert!(waited < limit, "{waited:?}");
}
