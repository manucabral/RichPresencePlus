//! Bounded HTTP GET for scripts.

use std::sync::OnceLock;
use std::time::Duration;

use serde_json::{Value, json};

/// Largest response body returned to a script.
const MAX_BODY: u64 = 1024 * 1024;
/// Time allowed for the whole request.
const TIMEOUT: Duration = Duration::from_secs(5);

/// One client for every script, so TLS setup and open connections are reused.
fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .max_redirects(0)
            .http_status_as_error(false)
            .build()
            .into()
    })
}

/// Sends a GET request.
///
/// Returns:
///     `{status, body}`; error statuses are data, not failures. Redirects are
///     not followed: the manifest approved one host.
///
/// Raises:
///     String: The request fails, times out or the body exceeds 1 MiB.
pub fn get(url: &str) -> Result<Value, String> {
    let mut response = agent().get(url).call().map_err(|e| e.to_string())?;
    let status = response.status().as_u16();
    let body = response
        .body_mut()
        .with_config()
        .limit(MAX_BODY)
        .read_to_string()
        .map_err(|e| e.to_string())?;
    Ok(json!({"status": status, "body": body}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    /// Serve one canned response on a local port.
    fn serve(response: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buffer = [0_u8; 1024];
                let _ = stream.read(&mut buffer);
                let _ = stream.write_all(response.as_bytes());
            }
        });
        format!("http://{address}/")
    }

    #[test]
    fn returns_status_and_body() {
        let url = serve("HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nhi");
        let answer = get(&url).unwrap();
        assert_eq!(answer, json!({"status": 200, "body": "hi"}));
    }

    #[test]
    fn error_statuses_are_data_not_failures() {
        let url = serve("HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        assert_eq!(get(&url).unwrap()["status"], 404);
    }

    #[test]
    fn redirects_are_not_followed() {
        let url = serve(
            "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        );
        assert_eq!(get(&url).unwrap()["status"], 302);
    }

    #[test]
    fn the_shared_client_still_refuses_redirects_after_other_requests() {
        let first = serve("HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok");
        assert_eq!(get(&first).unwrap()["status"], 200);
        let moved = serve(
            "HTTP/1.1 301 Moved\r\nLocation: http://127.0.0.1:1/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        );
        assert_eq!(get(&moved).unwrap()["status"], 301);
        assert!(std::ptr::eq(agent(), agent()));
    }

    #[test]
    fn unreachable_hosts_fail() {
        assert!(get("http://127.0.0.1:1/").is_err());
    }
}
