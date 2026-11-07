//! The server against real WebSocket clients on the loopback interface.

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use rpp_bridge::secure::{self, Session};
use rpp_bridge::{Bridge, Browser, Config, Page, Rule, VERSION, generate_token};
use rpp_plugins::Manifest;
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

const EXTENSION: &str = "chrome-extension://abcdefghijklmnop";

fn rules() -> Vec<Rule> {
    let video = Manifest::parse(
        r#"{"name":"Video","version":"1","client_id":"12345678901234567",
            "page":{"matches":["*://a.io/*"],"extract":{"title":{"from":"text","selector":"h1"}}}}"#,
    )
    .unwrap();
    Rule::from_manifests([&video])
}

async fn start_with(configure: impl FnOnce(&mut Config)) -> Bridge {
    let mut config = Config::new(0, generate_token());
    configure(&mut config);
    let bridge = Bridge::start(config).await.unwrap();
    bridge.set_rules(rules());
    bridge
}

async fn start() -> Bridge {
    start_with(|_| {}).await
}

async fn open(bridge: &Bridge, origin: Option<&str>) -> Result<Socket, String> {
    let mut request = format!("ws://{}/", bridge.address())
        .into_client_request()
        .unwrap();
    if let Some(origin) = origin {
        request
            .headers_mut()
            .insert("origin", origin.parse().unwrap());
    }
    connect_async(request)
        .await
        .map(|(socket, _)| socket)
        .map_err(|error| error.to_string())
}

/// Something that carries JSON messages: a raw socket before the handshake,
/// or a [`Client`] that seals and opens them after it.
trait Wire {
    async fn send_json(&mut self, value: Value);
    async fn next_json(&mut self) -> Option<Value>;
}

impl Wire for Socket {
    async fn send_json(&mut self, value: Value) {
        self.send(Message::text(value.to_string())).await.unwrap();
    }

    async fn next_json(&mut self) -> Option<Value> {
        loop {
            let frame = timeout(Duration::from_secs(3), self.next()).await.ok()??;
            match frame.ok()? {
                Message::Text(text) => return serde_json::from_str(text.as_str()).ok(),
                Message::Close(_) => return None,
                _ => {}
            }
        }
    }
}

/// An extension that finished the handshake.
struct Client {
    socket: Socket,
    session: Session,
}

impl Wire for Client {
    async fn send_json(&mut self, value: Value) {
        let sealed = self.session.seal(&value.to_string()).unwrap();
        self.socket.send(Message::text(sealed)).await.unwrap();
    }

    async fn next_json(&mut self) -> Option<Value> {
        let envelope = self.socket.next_json().await?;
        let n = envelope["n"].as_u64()?;
        let text = self.session.open(n, envelope["data"].as_str()?).ok()?;
        serde_json::from_str(&text).ok()
    }
}

async fn send(wire: &mut impl Wire, value: Value) {
    wire.send_json(value).await;
}

async fn receive(wire: &mut impl Wire) -> Option<Value> {
    wire.next_json().await
}

/// What the extension learns from a handshake.
struct Answered {
    welcome: Value,
    session: Session,
    server: String,
    client: String,
}

/// Answers the challenge the way the extension does; `None` if the server
/// closed instead.
async fn answer(socket: &mut Socket, token: &str, version: u32) -> Option<Answered> {
    answer_as(socket, token, version, "chrome").await
}

/// Like [`answer`], naming another browser in the hello.
async fn answer_as(
    socket: &mut Socket,
    token: &str,
    version: u32,
    browser: &str,
) -> Option<Answered> {
    let challenge = socket.next_json().await?;
    assert_eq!(challenge["type"], "challenge");
    let server = challenge["nonce"].as_str()?.to_owned();
    let client = secure::new_nonce().unwrap();
    let proof = secure::proof(token, "rpp-hello", &server, &client);
    socket
        .send_json(
            json!({"type": "hello", "browser": browser, "browser_version": "24.12.1",
                    "version": version, "nonce": client, "proof": proof}),
        )
        .await;
    let welcome = socket.next_json().await?;
    Some(Answered {
        welcome,
        session: Session::for_extension(token, &server, &client),
        server,
        client,
    })
}

/// Connects, authenticates and consumes the welcome and the rules.
async fn session(bridge: &Bridge) -> Client {
    session_as(bridge, "chrome").await
}

/// Like [`session`], naming another browser in the hello.
async fn session_as(bridge: &Bridge, browser: &str) -> Client {
    let mut socket = open(bridge, Some(EXTENSION)).await.unwrap();
    let answered = answer_as(&mut socket, bridge.token(), VERSION, browser)
        .await
        .unwrap();
    assert_eq!(answered.welcome["type"], "welcome");
    let mut client = Client {
        socket,
        session: answered.session,
    };
    assert_eq!(receive(&mut client).await.unwrap()["type"], "rules");
    client
}

fn page(tab: i64, presence: &str, url: &str, title: &str) -> Value {
    json!({"type": "page", "tab": tab, "presence": presence, "url": url,
           "title": title, "values": {"title": title}})
}

/// Waits until the reported pages satisfy a condition.
async fn eventually(bridge: &Bridge, condition: impl Fn(&[Page]) -> bool) {
    for _ in 0..100 {
        if condition(&bridge.pages()) {
            return;
        }
        sleep(Duration::from_millis(20)).await;
    }
    panic!("condition not met, pages: {:?}", bridge.pages());
}

#[tokio::test]
async fn a_valid_hello_gets_a_proven_welcome_and_sealed_rules() {
    let bridge = start().await;
    let mut socket = open(&bridge, Some(EXTENSION)).await.unwrap();
    let answered = answer(&mut socket, bridge.token(), VERSION).await.unwrap();
    assert_eq!(answered.welcome["type"], "welcome");
    // The application proves it knows the token too.
    let proof = answered.welcome["proof"].as_str().unwrap();
    let token = bridge.token();
    assert!(
        secure::check_proof(
            token,
            "rpp-welcome",
            &answered.server,
            &answered.client,
            proof
        )
        .is_ok()
    );
    // The rules travel sealed: the raw frame does not show them.
    let raw = socket.next_json().await.unwrap();
    assert_eq!(raw["type"], "sealed");
    assert!(!raw.to_string().contains("Video"));
    let mut session = answered.session;
    let rules = session
        .open(raw["n"].as_u64().unwrap(), raw["data"].as_str().unwrap())
        .unwrap();
    assert!(rules.contains("\"presence\":\"Video\""));
    let mut client = Client { socket, session };
    send(&mut client, json!({"type": "ping"})).await;
    assert_eq!(receive(&mut client).await.unwrap(), json!({"type": "pong"}));
}

#[tokio::test]
async fn web_pages_and_unlabelled_clients_cannot_connect() {
    let bridge = start().await;
    for origin in [
        Some("https://evil.example.com"),
        Some("http://127.0.0.1"),
        Some("null"),
        None,
    ] {
        let result = open(&bridge, origin).await;
        assert!(result.is_err(), "{origin:?} was accepted");
    }
}

#[tokio::test]
async fn a_wrong_token_or_version_is_dropped_without_a_welcome() {
    let bridge = start().await;
    let mut socket = open(&bridge, Some(EXTENSION)).await.unwrap();
    assert!(
        answer(&mut socket, "not-the-token", VERSION)
            .await
            .is_none()
    );

    let mut socket = open(&bridge, Some(EXTENSION)).await.unwrap();
    assert!(answer(&mut socket, bridge.token(), 99).await.is_none());

    // The old first message, which carried the token itself, is refused.
    let mut socket = open(&bridge, Some(EXTENSION)).await.unwrap();
    socket.next_json().await.unwrap();
    socket
        .send_json(
            json!({"type": "hello", "token": bridge.token(), "browser": "chrome", "version": 1}),
        )
        .await;
    assert_eq!(socket.next_json().await, None);
}

#[tokio::test]
async fn speaking_out_of_turn_or_never_speaking_is_dropped() {
    let bridge = start_with(|config| config.hello_timeout = Duration::from_millis(150)).await;
    let mut early = open(&bridge, Some(EXTENSION)).await.unwrap();
    early.next_json().await.unwrap();
    early
        .send_json(page(1, "Video", "https://a.io/", "x"))
        .await;
    assert_eq!(early.next_json().await, None);

    let mut silent = open(&bridge, Some(EXTENSION)).await.unwrap();
    assert_eq!(silent.next_json().await.unwrap()["type"], "challenge");
    assert_eq!(silent.next_json().await, None);
    assert!(bridge.pages().is_empty());
}

#[tokio::test]
async fn after_the_handshake_only_sealed_messages_in_order_are_accepted() {
    let bridge = start().await;
    // A message in the clear ends the connection.
    let mut client = session(&bridge).await;
    client
        .socket
        .send_json(page(1, "Video", "https://a.io/", "plain"))
        .await;
    assert_eq!(receive(&mut client).await, None);

    // A replayed sealed message ends it too.
    let mut client = session(&bridge).await;
    let sealed = client
        .session
        .seal(&json!({"type": "ping"}).to_string())
        .unwrap();
    client
        .socket
        .send(Message::text(sealed.clone()))
        .await
        .unwrap();
    assert_eq!(receive(&mut client).await.unwrap()["type"], "pong");
    client.socket.send(Message::text(sealed)).await.unwrap();
    assert_eq!(client.socket.next_json().await, None);

    // A second hello inside the session is refused.
    let mut client = session(&bridge).await;
    send(
        &mut client,
        json!({"type": "hello", "browser": "x", "version": 2, "nonce": "a", "proof": "b"}),
    )
    .await;
    assert_eq!(receive(&mut client).await, None);
    assert!(bridge.pages().is_empty());
}

#[tokio::test]
async fn pages_are_collected_updated_and_removed() {
    let bridge = start().await;
    let mut socket = session(&bridge).await;
    send(
        &mut socket,
        page(7, "Video", "https://a.io/watch?v=1", "First"),
    )
    .await;
    eventually(&bridge, |pages| {
        pages.len() == 1 && pages[0].title == "First"
    })
    .await;
    assert_eq!(bridge.pages()[0].browser, "chrome");
    assert_eq!(bridge.pages()[0].values["title"], "First");

    send(
        &mut socket,
        page(7, "Video", "https://a.io/watch?v=2", "Second"),
    )
    .await;
    eventually(&bridge, |pages| {
        pages.len() == 1 && pages[0].title == "Second"
    })
    .await;

    send(&mut socket, json!({"type": "gone", "tab": 7})).await;
    eventually(&bridge, <[Page]>::is_empty).await;
}

#[tokio::test]
async fn pages_disappear_when_the_extension_disconnects() {
    let bridge = start().await;
    let mut socket = session(&bridge).await;
    send(&mut socket, page(1, "Video", "https://a.io/", "x")).await;
    eventually(&bridge, |pages| pages.len() == 1).await;
    drop(socket);
    eventually(&bridge, <[Page]>::is_empty).await;
}

#[tokio::test]
async fn reports_for_unknown_presences_and_non_web_addresses_are_ignored() {
    let bridge = start().await;
    let mut socket = session(&bridge).await;
    send(&mut socket, page(1, "Unknown", "https://a.io/", "x")).await;
    send(&mut socket, page(2, "Video", "file:///etc/passwd", "x")).await;
    send(&mut socket, page(3, "Video", "javascript:alert(1)", "x")).await;
    send(
        &mut socket,
        page(
            4,
            "Video",
            &format!("https://a.io/{}", "x".repeat(3000)),
            "x",
        ),
    )
    .await;
    send(&mut socket, page(5, "Video", "https://a.io/ok", "kept")).await;
    eventually(&bridge, |pages| pages.len() == 1).await;
    assert_eq!(bridge.pages()[0].tab, 5);
}

#[tokio::test]
async fn one_connection_cannot_exceed_its_page_limit() {
    let bridge = start_with(|config| config.max_pages = 2).await;
    let mut socket = session(&bridge).await;
    for tab in 1..=4 {
        send(&mut socket, page(tab, "Video", "https://a.io/", "x")).await;
    }
    send(&mut socket, page(1, "Video", "https://a.io/", "updated")).await;
    eventually(&bridge, |pages| pages.iter().any(|p| p.title == "updated")).await;
    assert_eq!(bridge.pages().len(), 2);
}

#[tokio::test]
async fn new_rules_reach_connected_extensions() {
    let bridge = start().await;
    let mut socket = session(&bridge).await;
    bridge.set_rules(Vec::new());
    let update = receive(&mut socket).await.unwrap();
    assert_eq!(update, json!({"type": "rules", "rules": []}));
}

#[tokio::test]
async fn ping_is_answered_and_garbage_closes_the_connection() {
    let bridge = start().await;
    let mut socket = session(&bridge).await;
    send(&mut socket, json!({"type": "ping"})).await;
    assert_eq!(receive(&mut socket).await.unwrap(), json!({"type": "pong"}));
    send(&mut socket, json!({"type": "eval", "code": "1"})).await;
    assert_eq!(receive(&mut socket).await, None);
}

#[tokio::test]
async fn shutdown_closes_connections() {
    let bridge = start().await;
    let mut socket = session(&bridge).await;
    bridge.shutdown().await;
    assert_eq!(receive(&mut socket).await, None);
}

#[test]
fn tokens_are_long_and_different_each_time() {
    let (a, b) = (generate_token(), generate_token());
    assert_eq!(a.len(), 64);
    assert_ne!(a, b);
}

#[tokio::test]
async fn the_page_source_lists_only_the_pages_of_one_presence() {
    let bridge = start().await;
    let source = bridge.page_source();
    let mut socket = session(&bridge).await;
    send(&mut socket, page(1, "Video", "https://a.io/one", "One")).await;
    eventually(&bridge, |pages| pages.len() == 1).await;
    let listed = source("Video");
    assert_eq!(listed[0]["title"], "One");
    assert_eq!(listed[0]["values"]["title"], "One");
    assert!(listed[0]["received_at"].as_u64().unwrap() > 1_700_000_000_000);
    assert_eq!(source("Other"), json!([]));
}

#[tokio::test]
async fn each_connection_reports_its_browser() {
    let bridge = start().await;
    assert!(bridge.browsers().is_empty());
    let edge = session_as(&bridge, "edge").await;
    let chrome = session_as(&bridge, "chrome").await;
    let odd = session_as(&bridge, "<img src=x>").await;
    for _ in 0..100 {
        if bridge.extensions() == 3 {
            break;
        }
        sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(
        bridge.browsers(),
        vec![Browser::Chrome, Browser::Edge, Browser::Other]
    );
    drop(edge);
    for _ in 0..100 {
        if bridge.extensions() == 2 {
            break;
        }
        sleep(Duration::from_millis(20)).await;
    }
    // The one that left is the one removed, not the last one listed.
    assert_eq!(bridge.browsers(), vec![Browser::Chrome, Browser::Other]);
    let recent = bridge.recent_sessions();
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].browser, Browser::Edge);
    assert_eq!(recent[0].version, "24.12.1");
    assert!(recent[0].ended_at.is_some());
    assert_eq!(bridge.live_sessions().len(), 2);
    drop(chrome);
    drop(odd);
    for _ in 0..100 {
        if bridge.extensions() == 0 {
            break;
        }
        sleep(Duration::from_millis(20)).await;
    }
    assert!(bridge.browsers().is_empty());
}

#[tokio::test]
async fn connected_extensions_are_counted() {
    let bridge = start().await;
    assert_eq!(bridge.extensions(), 0);
    let first = session(&bridge).await;
    let second = session(&bridge).await;
    for _ in 0..100 {
        if bridge.extensions() == 2 {
            break;
        }
        sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(bridge.extensions(), 2);
    drop(first);
    drop(second);
    for _ in 0..100 {
        if bridge.extensions() == 0 {
            break;
        }
        sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(bridge.extensions(), 0);
}

/// Slots are taken before any proof: silent sockets must give way, or a local
/// program could keep the extension out for as long as it likes.
#[tokio::test]
async fn connections_that_never_pair_give_way_to_the_extension() {
    let bridge = start().await;
    let mut idle = Vec::new();
    for _ in 0..64 {
        idle.push(TcpStream::connect(bridge.address()).await.unwrap());
    }
    sleep(Duration::from_millis(100)).await;
    let client = timeout(Duration::from_secs(2), session(&bridge)).await;
    assert!(client.is_ok(), "the extension stayed out");
    assert_eq!(bridge.extensions(), 1);
    assert_eq!(idle.len(), 64);
    bridge.shutdown().await;
}

/// A paired extension no longer gives way to new connections.
#[tokio::test]
async fn a_paired_extension_keeps_its_connection_through_a_flood() {
    let bridge = start().await;
    let mut client = session(&bridge).await;
    let mut idle = Vec::new();
    for _ in 0..64 {
        idle.push(TcpStream::connect(bridge.address()).await.unwrap());
    }
    sleep(Duration::from_millis(100)).await;
    assert_eq!(bridge.extensions(), 1);
    send(&mut client, json!({"type": "ping"})).await;
    assert!(receive(&mut client).await.is_some());
    assert_eq!(idle.len(), 64);
    bridge.shutdown().await;
}

/// A program that connects and says nothing, not even the upgrade request,
/// loses its slot; otherwise a few of them keep the extension out for good.
#[tokio::test]
async fn sockets_that_never_start_the_upgrade_lose_their_slot() {
    let bridge = start_with(|config| config.hello_timeout = Duration::from_millis(100)).await;
    let mut idle = Vec::new();
    for _ in 0..32 {
        idle.push(TcpStream::connect(bridge.address()).await.unwrap());
    }
    sleep(Duration::from_millis(400)).await;
    // The silent sockets are still open on their side.
    assert_eq!(idle.len(), 32);
    let client = timeout(Duration::from_secs(2), session(&bridge)).await;
    assert!(client.is_ok(), "the extension stayed out");
    bridge.shutdown().await;
}

/// A client that stops reading fills the socket; the server must drop it
/// instead of waiting on the send for good.
#[tokio::test]
async fn a_client_that_stops_reading_is_dropped() {
    let bridge = start_with(|config| config.send_timeout = Duration::from_millis(100)).await;
    let client = session(&bridge).await;
    assert_eq!(bridge.extensions(), 1);

    let mut rule = rules().remove(0);
    rule.matches = vec!["x".repeat(4 * 1024 * 1024)];
    let mut dropped = false;
    for round in 0..200 {
        rule.presence = format!("Video {round}");
        bridge.set_rules(vec![rule.clone()]);
        sleep(Duration::from_millis(20)).await;
        if bridge.extensions() == 0 {
            dropped = true;
            break;
        }
    }
    assert!(dropped, "the stuck client kept its connection");
    drop(client);
    bridge.shutdown().await;
}
