//! Answers of the local read-only address for overlays and stream decks.
//!
//! A foreign `Host` or `Origin` is refused and no answer carries CORS headers,
//! so a web page cannot read it.

use serde::Serialize;

use crate::Activity;

/// Largest request read. A real one fits in a fraction of it.
pub const MAX_REQUEST: usize = 8 * 1024;

const OVERLAY_HTML: &str = include_str!("local_api/overlay.html");
const OVERLAY_JS: &str = include_str!("local_api/overlay.js");
const OVERLAY_CSS: &str = include_str!("local_api/overlay.css");
/// What the overlay may load: its own files, and the pictures Discord shows.
const OVERLAY_POLICY: &str = "default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src https:; base-uri 'none'; form-action 'none'";

/// What `/v1/now` answers.
#[derive(Debug, Serialize)]
struct Now<'a> {
    showing: bool,
    presence: Option<&'a str>,
    details: Option<&'a str>,
    state: Option<&'a str>,
    large_image: Option<&'a str>,
    small_image: Option<&'a str>,
    start: Option<u64>,
    end: Option<u64>,
}

/// The JSON of `/v1/now`: what Discord shows, every field empty when nothing is.
///
/// Args:
///     activity: As Discord received it, after the privacy rules.
pub fn now_json(presence: Option<&str>, activity: Option<&Activity>) -> String {
    let shown = activity.filter(|_| presence.is_some());
    let now = Now {
        showing: shown.is_some(),
        presence: presence.filter(|_| shown.is_some()),
        details: shown.and_then(|a| a.details.as_deref()),
        state: shown.and_then(|a| a.state.as_deref()),
        large_image: shown.and_then(|a| a.large_image.as_deref()),
        small_image: shown.and_then(|a| a.small_image.as_deref()),
        start: shown.and_then(|a| a.start),
        end: shown.and_then(|a| a.end),
    };
    serde_json::to_string(&now).unwrap_or_else(|_| "{\"showing\":false}".to_owned())
}

/// The value of a header, by its lowercase name.
fn header<'a>(request: &'a str, name: &str) -> Option<&'a str> {
    request.split("\r\n").skip(1).find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.eq_ignore_ascii_case(name).then(|| value.trim())
    })
}

fn response(status: &str, kind: &str, policy: Option<&str>, body: &str) -> Vec<u8> {
    let policy = policy.map_or(String::new(), |policy| {
        format!("Content-Security-Policy: {policy}\r\n")
    });
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\n{policy}Cache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

fn refused(status: &str) -> Vec<u8> {
    response(status, "text/plain; charset=utf-8", None, status)
}

/// The whole HTTP answer to one request, ready to send.
///
/// Args:
///     port: The port this address listens on; `Host` must name it.
///     now: The current text of `/v1/now`.
pub fn answer(request: &[u8], port: u16, now: &str) -> Vec<u8> {
    let Ok(request) = std::str::from_utf8(request) else {
        return refused("400 Bad Request");
    };
    let mut words = request.split("\r\n").next().unwrap_or("").split(' ');
    let (Some(method), Some(path)) = (words.next(), words.next()) else {
        return refused("400 Bad Request");
    };
    let ours =
        |host: &str| host == format!("127.0.0.1:{port}") || host == format!("localhost:{port}");
    if !header(request, "host").is_some_and(ours) {
        return refused("403 Forbidden");
    }
    let origin = header(request, "origin");
    if origin.is_some_and(|origin| !origin.strip_prefix("http://").is_some_and(ours)) {
        return refused("403 Forbidden");
    }
    if method != "GET" {
        return refused("405 Method Not Allowed");
    }
    match path {
        "/v1/now" => response("200 OK", "application/json; charset=utf-8", None, now),
        "/overlay" => response(
            "200 OK",
            "text/html; charset=utf-8",
            Some(OVERLAY_POLICY),
            OVERLAY_HTML,
        ),
        "/overlay.js" => response("200 OK", "text/javascript; charset=utf-8", None, OVERLAY_JS),
        "/overlay.css" => response("200 OK", "text/css; charset=utf-8", None, OVERLAY_CSS),
        _ => refused("404 Not Found"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PORT: u16 = 28752;

    fn get(path: &str, headers: &str) -> String {
        let request = format!("GET {path} HTTP/1.1\r\n{headers}\r\n\r\n");
        String::from_utf8(answer(request.as_bytes(), PORT, "{\"showing\":false}")).unwrap()
    }

    const HOST: &str = "Host: 127.0.0.1:28752";

    #[test]
    fn the_address_answers_what_discord_shows() {
        let text = get("/v1/now", HOST);
        assert!(text.starts_with("HTTP/1.1 200 OK\r\n"), "{text}");
        assert!(text.contains("application/json"));
        assert!(text.ends_with("{\"showing\":false}"));
    }

    #[test]
    fn localhost_is_the_same_address() {
        assert!(get("/v1/now", "Host: localhost:28752").starts_with("HTTP/1.1 200"));
        assert!(get("/v1/now", "hOsT:   localhost:28752").starts_with("HTTP/1.1 200"));
    }

    /// A site that points its own name at 127.0.0.1 reads as same-origin in a browser.
    #[test]
    fn a_site_that_points_its_name_here_is_refused() {
        assert!(get("/v1/now", "Host: evil.example:28752").starts_with("HTTP/1.1 403"));
        assert!(get("/v1/now", "Host: 127.0.0.1").starts_with("HTTP/1.1 403"));
        assert!(get("/v1/now", "Host: 127.0.0.1:1").starts_with("HTTP/1.1 403"));
        assert!(get("/v1/now", "").starts_with("HTTP/1.1 403"));
    }

    #[test]
    fn a_page_from_elsewhere_is_refused() {
        let foreign = format!("{HOST}\r\nOrigin: https://evil.example");
        assert!(get("/v1/now", &foreign).starts_with("HTTP/1.1 403"));
        let null = format!("{HOST}\r\nOrigin: null");
        assert!(get("/v1/now", &null).starts_with("HTTP/1.1 403"));
        let own = format!("{HOST}\r\nOrigin: http://127.0.0.1:28752");
        assert!(get("/v1/now", &own).starts_with("HTTP/1.1 200"));
    }

    #[test]
    fn no_answer_lets_another_site_read_it() {
        for path in [
            "/v1/now",
            "/overlay",
            "/overlay.js",
            "/overlay.css",
            "/nothing",
        ] {
            assert!(
                !get(path, HOST).to_lowercase().contains("access-control"),
                "{path}"
            );
        }
    }

    #[test]
    fn only_reading_exists() {
        let request = format!("POST /v1/now HTTP/1.1\r\n{HOST}\r\n\r\n");
        let text = String::from_utf8(answer(request.as_bytes(), PORT, "{}")).unwrap();
        assert!(text.starts_with("HTTP/1.1 405"), "{text}");
    }

    #[test]
    fn garbage_gets_an_answer_and_no_panic() {
        for bytes in [
            &b""[..],
            b"\xff\xfe\x00",
            b"GET",
            b"\r\n\r\n",
            b"GET /v1/now",
        ] {
            let text = String::from_utf8(answer(bytes, PORT, "{}")).unwrap();
            assert!(text.starts_with("HTTP/1.1 4"), "{text}");
        }
    }

    #[test]
    fn the_overlay_page_carries_its_rules() {
        let text = get("/overlay", HOST);
        assert!(text.contains("Content-Security-Policy: default-src 'none'"));
        assert!(text.contains("<script src=\"/overlay.js\""));
        // Only text from the activity reaches the page, and never as markup.
        assert!(!OVERLAY_JS.contains("innerHTML"));
    }

    #[test]
    fn nothing_on_display_leaves_every_field_empty() {
        let text = now_json(None, Some(&Activity::default()));
        assert!(text.contains("\"showing\":false"));
        assert!(text.contains("\"presence\":null"));
    }

    #[test]
    fn the_activity_comes_out_as_discord_got_it() {
        let activity = Activity {
            details: Some("Reading".into()),
            start: Some(5),
            ..Activity::default()
        };
        let value: serde_json::Value =
            serde_json::from_str(&now_json(Some("Book"), Some(&activity))).unwrap();
        assert_eq!(value["showing"], true);
        assert_eq!(value["presence"], "Book");
        assert_eq!(value["details"], "Reading");
        assert_eq!(value["start"], 5);
    }
}
