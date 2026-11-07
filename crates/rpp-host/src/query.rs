//! Asking game servers for their public information.

use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, ToSocketAddrs};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// How long an answer, or a silence, is reused for the same server. Name, mode
/// and players change slowly, and a popular server gets this question from
/// every player who uses the application.
const REUSE: Duration = Duration::from_secs(30);
/// Shortest time between two questions to servers not asked recently.
const SPACING: Duration = Duration::from_millis(500);
/// Longest wait for a server's answer.
const ANSWER_TIMEOUT: Duration = Duration::from_millis(750);
/// Longest wait for a host name to resolve; a script has 2 seconds per read.
const RESOLVE_TIMEOUT: Duration = Duration::from_millis(500);
/// Most names being resolved at once. Each takes a thread that cannot be stopped,
/// and a resolver that hangs must not let a script pile them up.
const MAX_RESOLVING: usize = 4;
/// Most servers remembered.
const MAX_REMEMBERED: usize = 32;

type Key = (String, SocketAddrV4);

/// Names being resolved right now.
static RESOLVING: AtomicUsize = AtomicUsize::new(0);

/// Remembered answers and when the last new question left.
pub struct Querier {
    answers: Mutex<HashMap<Key, (Instant, Value)>>,
    last_sent: Mutex<Option<Instant>>,
    spacing: Duration,
}

/// The one used by presences.
static SHARED: std::sync::LazyLock<Querier> = std::sync::LazyLock::new(|| Querier::new(SPACING));

/// Asks a server with the shared [`Querier`].
///
/// Raises:
///     String: See [`Querier::ask`].
pub fn query(protocol: &str, host: &str, port: u16) -> Result<Value, String> {
    SHARED.ask(protocol, host, port)
}

/// Addresses that are not one server: nothing, everyone, or a group.
fn reachable(ip: Ipv4Addr) -> bool {
    !(ip.is_unspecified() || ip.is_broadcast() || ip.is_multicast() || ip.octets()[0] == 0)
}

/// Resolves a host name, giving up after [`RESOLVE_TIMEOUT`].
fn resolve(host: &str, port: u16) -> Option<Ipv4Addr> {
    if let Ok(ip) = host.parse::<Ipv4Addr>() {
        return Some(ip);
    }
    if RESOLVING.fetch_add(1, Ordering::SeqCst) >= MAX_RESOLVING {
        RESOLVING.fetch_sub(1, Ordering::SeqCst);
        return None;
    }
    let (send, receive) = mpsc::channel();
    let name = host.to_owned();
    let started = std::thread::Builder::new()
        .name("query-resolve".into())
        .spawn(move || {
            let first = (name.as_str(), port)
                .to_socket_addrs()
                .ok()
                .and_then(|mut found| {
                    found.find_map(|addr| match addr {
                        SocketAddr::V4(v4) => Some(*v4.ip()),
                        SocketAddr::V6(_) => None,
                    })
                });
            RESOLVING.fetch_sub(1, Ordering::SeqCst);
            let _ = send.send(first);
        });
    if started.is_err() {
        RESOLVING.fetch_sub(1, Ordering::SeqCst);
        return None;
    }
    receive.recv_timeout(RESOLVE_TIMEOUT).ok().flatten()
}

impl Querier {
    /// A querier with its own memory, spacing new questions by `spacing`.
    pub fn new(spacing: Duration) -> Self {
        Self {
            answers: Mutex::new(HashMap::new()),
            last_sent: Mutex::new(None),
            spacing,
        }
    }

    /// Asks a server, or returns what it said less than 30 seconds ago;
    /// `null` if it did not answer.
    ///
    /// Args:
    ///     protocol: Such as `samp`.
    ///     host: An IPv4 address or a host name.
    ///
    /// Raises:
    ///     String: The protocol is unknown, the address cannot be resolved
    ///         or is not a single server, or a new question comes too soon.
    pub fn ask(&self, protocol: &str, host: &str, port: u16) -> Result<Value, String> {
        if protocol != "samp" {
            return Err(format!("unknown query protocol {protocol}"));
        }
        let ip = resolve(host, port).ok_or_else(|| format!("cannot resolve {host}"))?;
        if !reachable(ip) || port == 0 {
            return Err("that address is not a server".into());
        }
        let key = (protocol.to_owned(), SocketAddrV4::new(ip, port));
        let now = Instant::now();
        {
            let answers = self.answers.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some((at, value)) = answers.get(&key)
                && now.saturating_duration_since(*at) < REUSE
            {
                return Ok(value.clone());
            }
        }
        {
            let mut last = self
                .last_sent
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            if last.is_some_and(|at| now.saturating_duration_since(at) < self.spacing) {
                return Err("too many server queries; try again in a moment".into());
            }
            *last = Some(now);
        }
        let value =
            crate::samp::query_udp(key.1, ANSWER_TIMEOUT).map_or(Value::Null, |info| json!(info));
        let mut answers = self.answers.lock().unwrap_or_else(PoisonError::into_inner);
        if answers.len() >= MAX_REMEMBERED {
            answers.retain(|_, (at, _)| now.saturating_duration_since(*at) < REUSE);
            if answers.len() >= MAX_REMEMBERED {
                answers.clear();
            }
        }
        answers.insert(key, (Instant::now(), value.clone()));
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::UdpSocket;

    /// A server on this computer that answers `count` questions, then stops.
    fn fake_server(count: usize) -> (u16, std::thread::JoinHandle<usize>) {
        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        server
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let port = server.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            let mut answered = 0;
            let mut buf = [0_u8; 64];
            while answered < count {
                let Ok((size, client)) = server.recv_from(&mut buf) else {
                    break;
                };
                assert_eq!(size, 11);
                let reply = crate::samp::sample_answer(
                    Ipv4Addr::LOCALHOST,
                    port,
                    ["{FF0000}Las Venturas", "Freeroam", "English"],
                );
                server.send_to(&reply, client).unwrap();
                answered += 1;
            }
            answered
        });
        (port, handle)
    }

    #[test]
    fn a_server_is_asked_once_and_its_answer_reused() {
        let (port, server) = fake_server(1);
        let querier = Querier::new(Duration::ZERO);
        let first = querier.ask("samp", "127.0.0.1", port).unwrap();
        assert_eq!(first["hostname"], "Las Venturas");
        assert_eq!(first["max_players"], 200);
        let again = querier.ask("samp", "127.0.0.1", port).unwrap();
        assert_eq!(again, first);
        assert_eq!(server.join().unwrap(), 1);
    }

    #[test]
    fn a_silent_server_gives_null_and_is_not_asked_again_at_once() {
        // Bound but never read: nothing answers.
        let silent = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = silent.local_addr().unwrap().port();
        let querier = Querier::new(Duration::ZERO);
        assert_eq!(querier.ask("samp", "127.0.0.1", port).unwrap(), Value::Null);
        let started = Instant::now();
        assert_eq!(querier.ask("samp", "127.0.0.1", port).unwrap(), Value::Null);
        assert!(started.elapsed() < ANSWER_TIMEOUT, "the silence is reused");
    }

    #[test]
    fn new_questions_are_spaced_out() {
        let querier = Querier::new(Duration::from_secs(60));
        let silent = UdpSocket::bind("127.0.0.1:0").unwrap();
        let other = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = silent.local_addr().unwrap().port();
        querier.ask("samp", "127.0.0.1", port).unwrap();
        let second = other.local_addr().unwrap().port();
        assert!(querier.ask("samp", "127.0.0.1", second).is_err());
    }

    #[test]
    fn unknown_protocols_and_group_addresses_are_refused() {
        let querier = Querier::new(Duration::ZERO);
        assert!(querier.ask("source", "127.0.0.1", 27015).is_err());
        for host in ["0.0.0.0", "255.255.255.255", "224.0.0.1", "0.1.2.3"] {
            assert!(querier.ask("samp", host, 7777).is_err(), "{host}");
        }
        assert!(querier.ask("samp", "127.0.0.1", 0).is_err());
        assert!(querier.ask("samp", "name.invalid", 7777).is_err());
    }
}
