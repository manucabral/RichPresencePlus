//! Accepting extension connections and collecting the pages they report.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::io;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use rpp_core::Quiet;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Semaphore, watch};
use tokio::task::{AbortHandle, JoinHandle};
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::StatusCode;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::{WebSocketStream, accept_hdr_async_with_config};

use crate::protocol::{
    Browser, Envelope, FromExtension, Page, Rule, SiteRequest, ToExtension, VERSION,
};
use crate::secure::{self, Session};
use crate::sessions::{SessionView, Sessions};

/// Largest message accepted from the extension.
const MAX_MESSAGE: usize = 256 * 1024;
/// Longest address or title kept for a page.
const MAX_TEXT: usize = 2048;
/// Most connections open at once, paired or not.
const MAX_CONNECTIONS: usize = 32;
/// Most connections at once that have not proved the pairing code.
const MAX_WAITING: usize = 8;
/// Presences waiting to be made. Each one is a click on a page, so a few is plenty.
const MAX_SITE_REQUESTS: usize = 4;
/// Shortest time between two refused proofs in the log.
const LOG_REFUSALS_EVERY: Duration = Duration::from_secs(60);
/// Wait after the operating system refuses a connection.
const ACCEPT_RETRY: Duration = Duration::from_millis(200);

/// How the server behaves.
#[derive(Debug, Clone)]
pub struct Config {
    /// Loopback port to listen on; 0 picks a free one.
    pub port: u16,
    /// Secret the extension must present.
    pub token: String,
    /// Time allowed for the WebSocket upgrade, and then for the first message.
    pub hello_timeout: Duration,
    /// Longest wait for a client to take a frame.
    pub send_timeout: Duration,
    /// A connection that stays silent this long is closed.
    pub idle_timeout: Duration,
    /// Most pages one connection may keep.
    pub max_pages: usize,
}

impl Config {
    /// Creates a configuration with the default limits.
    ///
    /// Args:
    ///     port: Loopback port, or 0 for any free port.
    pub fn new(port: u16, token: String) -> Self {
        Self {
            port,
            token,
            hello_timeout: Duration::from_secs(5),
            send_timeout: Duration::from_secs(5),
            idle_timeout: Duration::from_secs(60),
            max_pages: 256,
        }
    }
}

/// An unguessable token: 64 hexadecimal characters from two random UUIDs.
pub fn generate_token() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

struct Shared {
    config: Config,
    pages: Mutex<HashMap<(u64, i64), Page>>,
    published: watch::Sender<Vec<Page>>,
    rules: watch::Receiver<Vec<Rule>>,
    sessions: watch::Sender<Sessions>,
    next_connection: AtomicU64,
    /// Connections that have not proved the pairing code, oldest first.
    waiting: Mutex<VecDeque<(u64, AbortHandle)>>,
    /// Any local program can fail the proof as often as it likes.
    refusals: Mutex<Quiet>,
    /// Presences asked for from a page; the application takes them when it looks.
    site_requests: Arc<Mutex<VecDeque<SiteRequest>>>,
}

/// Seconds since the Unix epoch; 0 if the clock is before it.
fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

/// Lists an authenticated extension for as long as its connection lives.
struct Registered<'a> {
    sessions: &'a watch::Sender<Sessions>,
    id: u64,
}

impl<'a> Registered<'a> {
    fn new(
        sessions: &'a watch::Sender<Sessions>,
        id: u64,
        browser: Browser,
        version: &str,
        extension: &str,
    ) -> Self {
        sessions.send_modify(|all| all.open(id, browser, version, extension, unix_now()));
        Self { sessions, id }
    }
}

impl Drop for Registered<'_> {
    fn drop(&mut self) {
        self.sessions
            .send_modify(|all| all.close(self.id, unix_now()));
    }
}

impl Shared {
    fn publish(&self, pages: &HashMap<(u64, i64), Page>) {
        let mut tabs: BTreeMap<u64, usize> = BTreeMap::new();
        for (connection, _) in pages.keys() {
            *tabs.entry(*connection).or_default() += 1;
        }
        self.sessions.send_modify(|all| all.set_tabs(&tabs));
        let mut all: Vec<Page> = pages.values().cloned().collect();
        all.sort_by(|a, b| (&a.browser, a.tab, &a.presence).cmp(&(&b.browser, b.tab, &b.presence)));
        self.published.send_replace(all);
    }

    fn with_pages(&self, change: impl FnOnce(&mut HashMap<(u64, i64), Page>)) {
        if let Ok(mut pages) = self.pages.lock() {
            change(&mut pages);
            self.publish(&pages);
        }
    }

    /// Makes room for a new connection and notes it, as the oldest gives way.
    fn admit(&self, id: u64, start: impl FnOnce() -> AbortHandle) {
        // Held while the task starts: it must not pair before it is listed.
        let Ok(mut waiting) = self.waiting.lock() else {
            return;
        };
        if waiting.len() >= MAX_WAITING
            && let Some((_, oldest)) = waiting.pop_front()
        {
            oldest.abort();
        }
        waiting.push_back((id, start()));
    }

    /// A connection proved the pairing code and no longer gives way.
    fn paired(&self, id: u64) {
        if let Ok(mut waiting) = self.waiting.lock() {
            waiting.retain(|(listed, _)| *listed != id);
        }
    }

    fn may_log_refusal(&self) -> bool {
        self.refusals
            .lock()
            .is_ok_and(|mut quiet| quiet.ready(Instant::now()))
    }

    fn accepts_presence(&self, presence: &str) -> bool {
        self.rules
            .borrow()
            .iter()
            .any(|rule| rule.presence == presence)
    }
}

/// Handshake check: only browser extensions on the loopback host may connect.
// The error type is fixed by the `Callback` trait of tungstenite.
#[allow(clippy::result_large_err)]
fn check_handshake(request: &Request, response: Response) -> Result<Response, ErrorResponse> {
    let header = |name: &str| {
        request
            .headers()
            .get(name)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
    };
    let origin = header("origin");
    let host = header("host");
    let from_extension =
        origin.starts_with("chrome-extension://") || origin.starts_with("moz-extension://");
    let loopback = host.starts_with("127.0.0.1:") || host.starts_with("localhost:");
    if from_extension && loopback {
        return Ok(response);
    }
    let mut refusal = ErrorResponse::new(Some("forbidden".into()));
    *refusal.status_mut() = StatusCode::FORBIDDEN;
    Err(refusal)
}

/// Current time in Unix milliseconds.
fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        })
}

/// A message sent before the session exists.
fn plain(message: &ToExtension) -> Message {
    Message::text(serde_json::to_string(message).unwrap_or_default())
}

/// A message sealed with the session key, or `None` if sealing failed.
fn sealed(session: &mut Session, message: &ToExtension) -> Option<Message> {
    let text = serde_json::to_string(message).ok()?;
    session.seal(&text).ok().map(Message::text)
}

/// Opens a sealed envelope and reads the message inside.
fn opened(session: &mut Session, text: &str) -> Option<FromExtension> {
    let Envelope::Sealed { n, data } = serde_json::from_str(text).ok()?;
    let inner = session.open(n, &data).ok()?;
    serde_json::from_str(&inner).ok()
}

/// Keeps `value` when it fits a page field.
fn short(value: String) -> Option<String> {
    (value.len() <= MAX_TEXT).then_some(value)
}

struct Connection<'a> {
    shared: &'a Shared,
    id: u64,
    browser: String,
}

impl Connection<'_> {
    /// Applies one message and returns the reply, if it calls for one.
    fn handle(&self, message: FromExtension) -> Option<ToExtension> {
        match message {
            FromExtension::Ping => Some(ToExtension::Pong),
            FromExtension::Hello { .. } => None,
            FromExtension::MakeSite {
                name,
                host,
                first,
                second,
            } => {
                let mut waiting = self
                    .shared
                    .site_requests
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner);
                if waiting.len() < MAX_SITE_REQUESTS {
                    waiting.push_back(SiteRequest {
                        name,
                        host,
                        first,
                        second,
                    });
                }
                None
            }
            FromExtension::Gone { tab } => {
                self.shared.with_pages(|pages| {
                    pages.remove(&(self.id, tab));
                });
                None
            }
            FromExtension::Page {
                tab,
                presence,
                url,
                title,
                values,
                visible,
                focused,
            } => {
                let web = url.starts_with("http://") || url.starts_with("https://");
                let (Some(url), Some(title)) = (short(url), short(title)) else {
                    return None;
                };
                if !web || !self.shared.accepts_presence(&presence) {
                    return None;
                }
                let now = unix_millis();
                let mut page = Page {
                    browser: self.browser.clone(),
                    tab,
                    presence,
                    url,
                    title,
                    values,
                    received_at: now,
                    visible,
                    focused,
                    focused_at: if focused { now } else { 0 },
                };
                let limit = self.shared.config.max_pages;
                let id = self.id;
                self.shared.with_pages(|pages| {
                    let earlier = pages.get(&(id, tab)).map(|old| old.focused_at);
                    let mine = pages.keys().filter(|(owner, _)| *owner == id).count();
                    if earlier.is_some() || mine < limit {
                        // A tab that lost the focus keeps the moment it last had it.
                        page.focused_at = page.focused_at.max(earlier.unwrap_or(0));
                        pages.insert((id, tab), page);
                    }
                });
                None
            }
        }
    }
}

async fn serve(stream: TcpStream, shared: Arc<Shared>, id: u64, mut stop: watch::Receiver<bool>) {
    converse(stream, &shared, id, &mut stop).await;
    shared.with_pages(|pages| pages.retain(|(owner, _), _| *owner != id));
}

/// Sends one frame to a client that may have stopped reading.
///
/// Returns:
///     False if it could not be sent in time. The connection must then end,
///     or a stuck client would keep its slot and this task for good.
async fn deliver(socket: &mut WebSocketStream<TcpStream>, frame: Message, wait: Duration) -> bool {
    matches!(timeout(wait, socket.send(frame)).await, Ok(Ok(())))
}

async fn converse(stream: TcpStream, shared: &Shared, id: u64, stop: &mut watch::Receiver<bool>) {
    let limits = WebSocketConfig::default()
        .max_message_size(Some(MAX_MESSAGE))
        .max_frame_size(Some(MAX_MESSAGE));
    // Without a limit here a program that connects and stays silent would hold
    // a slot for good, and enough of them would keep the extension out.
    let upgrade = accept_hdr_async_with_config(stream, check_handshake, Some(limits));
    let Ok(Ok(mut socket)) = timeout(shared.config.hello_timeout, upgrade).await else {
        return;
    };
    let wait = shared.config.send_timeout;

    // Challenge, answer, welcome: see `secure` for why the token never travels.
    let Some(server_nonce) = secure::new_nonce() else {
        return;
    };
    let challenge = ToExtension::Challenge {
        version: VERSION,
        nonce: server_nonce.clone(),
    };
    if !deliver(&mut socket, plain(&challenge), wait).await {
        return;
    }
    let hello = timeout(shared.config.hello_timeout, socket.next()).await;
    let Ok(Some(Ok(Message::Text(text)))) = hello else {
        return;
    };
    let Ok(FromExtension::Hello {
        browser,
        browser_version,
        extension_version,
        version,
        nonce: client_nonce,
        proof,
    }) = serde_json::from_str(text.as_str())
    else {
        return;
    };
    let token = shared.config.token.as_str();
    let proven = secure::check_proof(token, "rpp-hello", &server_nonce, &client_nonce, &proof);
    if version != VERSION || browser.len() > 32 || proven.is_err() {
        if shared.may_log_refusal() {
            tracing::warn!(
                version,
                "a browser extension could not prove it knows the pairing code"
            );
        }
        return;
    }
    shared.paired(id);
    let welcome = ToExtension::Welcome {
        version: VERSION,
        proof: secure::proof(token, "rpp-welcome", &server_nonce, &client_nonce),
    };
    if !deliver(&mut socket, plain(&welcome), wait).await {
        return;
    }
    let mut session = Session::for_application(token, &server_nonce, &client_nonce);

    let kind = Browser::from_hello(&browser);
    let _registered = Registered::new(
        &shared.sessions,
        id,
        kind,
        &browser_version,
        &extension_version,
    );
    tracing::info!(browser = ?kind, "browser extension connected");
    let mut rules = shared.rules.clone();
    let connection = Connection {
        shared,
        id,
        browser,
    };
    let first = ToExtension::Rules {
        rules: rules.borrow_and_update().clone(),
    };
    let Some(first) = sealed(&mut session, &first) else {
        return;
    };
    if !deliver(&mut socket, first, wait).await {
        return;
    }

    loop {
        let reply = tokio::select! {
            incoming = timeout(shared.config.idle_timeout, socket.next()) => {
                let Ok(Some(Ok(frame))) = incoming else { break };
                match frame {
                    Message::Text(text) => {
                        // Anything that does not open, or a second hello, ends the connection.
                        match opened(&mut session, text.as_str()) {
                            Some(FromExtension::Hello { .. }) | None => break,
                            Some(message) => connection.handle(message),
                        }
                    }
                    Message::Close(_) => break,
                    _ => None,
                }
            }
            changed = rules.changed() => {
                if changed.is_err() {
                    break;
                }
                Some(ToExtension::Rules { rules: rules.borrow_and_update().clone() })
            }
            _ = stop.changed() => break,
        };
        if let Some(reply) = reply {
            let Some(frame) = sealed(&mut session, &reply) else {
                break;
            };
            if !deliver(&mut socket, frame, wait).await {
                break;
            }
        }
    }
    let _ = timeout(wait, socket.close(None)).await;
    tracing::info!("browser extension disconnected");
}

/// The running server.
#[derive(Debug)]
pub struct Bridge {
    /// Presences asked for from a page, shared with the connections.
    site_requests: Arc<Mutex<VecDeque<SiteRequest>>>,
    address: SocketAddr,
    token: String,
    rules: watch::Sender<Vec<Rule>>,
    pages: watch::Receiver<Vec<Page>>,
    sessions: watch::Receiver<Sessions>,
    stop: watch::Sender<bool>,
    accept: Mutex<Option<JoinHandle<()>>>,
}

impl Bridge {
    /// Starts listening on the loopback interface.
    pub async fn start(config: Config) -> io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", config.port)).await?;
        let address = listener.local_addr()?;
        let token = config.token.clone();
        let (rules, rules_receiver) = watch::channel(Vec::new());
        let (published, pages) = watch::channel(Vec::new());
        let (sessions_sender, sessions) = watch::channel(Sessions::default());
        let (stop, stop_receiver) = watch::channel(false);
        let site_requests = Arc::new(Mutex::new(VecDeque::new()));
        let shared = Arc::new(Shared {
            config,
            pages: Mutex::new(HashMap::new()),
            published,
            rules: rules_receiver,
            sessions: sessions_sender,
            next_connection: AtomicU64::new(1),
            waiting: Mutex::new(VecDeque::new()),
            refusals: Mutex::new(Quiet::new(LOG_REFUSALS_EVERY)),
            site_requests: Arc::clone(&site_requests),
        });
        let mut accepting = stop_receiver.clone();
        let open_slots = Arc::new(Semaphore::new(MAX_CONNECTIONS));
        let accept = tokio::spawn(async move {
            loop {
                tokio::select! {
                    accepted = listener.accept() => {
                        let Ok((stream, _)) = accepted else {
                            // Out of sockets or handles: wait instead of spinning.
                            tokio::time::sleep(ACCEPT_RETRY).await;
                            continue;
                        };
                        let Ok(permit) = Arc::clone(&open_slots).try_acquire_owned() else {
                            continue;
                        };
                        let id = shared.next_connection.fetch_add(1, Ordering::Relaxed);
                        let for_task = Arc::clone(&shared);
                        let stop = stop_receiver.clone();
                        shared.admit(id, || {
                            tokio::spawn(async move {
                                serve(stream, for_task, id, stop).await;
                                drop(permit);
                            })
                            .abort_handle()
                        });
                    }
                    _ = accepting.changed() => break,
                }
            }
        });
        Ok(Self {
            site_requests,
            address,
            token,
            rules,
            pages,
            sessions,
            stop,
            accept: Mutex::new(Some(accept)),
        })
    }

    /// Returns the address the server listens on.
    pub fn address(&self) -> SocketAddr {
        self.address
    }

    /// Returns the number of extensions that completed the hello.
    pub fn extensions(&self) -> usize {
        self.sessions.borrow().count()
    }

    /// The browser of each extension that completed the hello, sorted. Two
    /// profiles of one browser give two entries.
    pub fn browsers(&self) -> Vec<Browser> {
        self.sessions.borrow().browsers()
    }

    /// Returns the open sessions, oldest first.
    pub fn live_sessions(&self) -> Vec<SessionView> {
        self.sessions.borrow().live()
    }

    /// Returns the last sessions that ended, newest first; kept in memory only.
    pub fn recent_sessions(&self) -> Vec<SessionView> {
        self.sessions.borrow().recent()
    }

    /// Returns the secret the extension must present.
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Replaces the rules and pushes them to every connected extension.
    pub fn set_rules(&self, rules: Vec<Rule>) {
        self.rules.send_replace(rules);
    }

    /// Takes the presences asked for from a page since the last call, oldest
    /// first.
    pub fn take_site_requests(&self) -> Vec<SiteRequest> {
        self.site_requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .drain(..)
            .collect()
    }

    /// Returns the pages currently reported by all connected extensions.
    pub fn pages(&self) -> Vec<Page> {
        self.pages.borrow().clone()
    }

    /// A function from a presence name to the JSON array of its pages.
    pub fn page_source(&self) -> impl Fn(&str) -> serde_json::Value + Send + Sync + 'static {
        let pages = self.pages.clone();
        move |presence| {
            let listed: Vec<serde_json::Value> = pages
                .borrow()
                .iter()
                .filter(|page| page.presence == presence)
                .filter_map(|page| serde_json::to_value(page).ok())
                .collect();
            serde_json::Value::Array(listed)
        }
    }

    /// Returns a receiver that wakes when the pages change.
    pub fn watch_pages(&self) -> watch::Receiver<Vec<Page>> {
        self.pages.clone()
    }

    /// Calls `wake` with each presence whose pages change, until the bridge ends.
    ///
    /// Must be called inside a Tokio runtime.
    pub fn on_page_changes(&self, wake: impl Fn(&str) + Send + 'static) {
        tokio::spawn(crate::changes::follow(self.pages.clone(), wake));
    }

    /// Closes every connection and stops listening.
    pub async fn shutdown(&self) {
        self.stop.send_replace(true);
        let accept = self.accept.lock().map_or(None, |mut handle| handle.take());
        if let Some(accept) = accept {
            let _ = accept.await;
        }
    }
}
