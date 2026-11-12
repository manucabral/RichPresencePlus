//! Native messaging: how the browser extension learns the port and the token.

use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

use serde_json::{Value, json};

/// Name the browsers know the host by. Lowercase letters, digits and dots only.
pub const HOST_NAME: &str = "com.manucabral.richpresenceplus";

/// Chrome and Edge extension IDs that may ask for the pairing.
///
/// The first is the copy loaded by hand, fixed by `key` in the manifest; the
/// second is the one the Chrome Web Store assigned.
pub const CHROMIUM_EXTENSIONS: [&str; 2] = [
    "iijcphjgahgdkdeaibcgagjfoaedajjk",
    "cjdkaikgiokfekdbckccmlbffinaknib",
];

/// Firefox add-on IDs that may ask for the pairing (`browser_specific_settings.gecko.id`).
pub const FIREFOX_EXTENSIONS: [&str; 1] = ["richpresenceplus@manucabral.dev"];

/// File in the data folder that holds the bridge's current port and token.
pub const PAIRING_FILE: &str = "pairing.json";

/// Largest request read. The only request is `{"type":"pairing"}`.
pub const MAX_REQUEST: u32 = 1024;

/// Largest pairing file read; the real one is under 120 bytes.
const MAX_PAIRING_FILE: u64 = 4096;

/// Browsers that read the same kind of host manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    /// Chrome and Edge: `allowed_origins` with `chrome-extension://` origins.
    Chromium,
    /// Firefox: `allowed_extensions` with add-on IDs.
    Firefox,
}

impl Family {
    /// File name of this family's host manifest in the data folder.
    pub fn manifest_file(self) -> &'static str {
        match self {
            Self::Chromium => "native-host-chromium.json",
            Self::Firefox => "native-host-firefox.json",
        }
    }
}

/// The host manifest a browser reads to start this program.
///
/// Args:
///     exe: Absolute path of this program.
pub fn host_manifest(family: Family, exe: &Path) -> Value {
    let mut manifest = json!({
        "name": HOST_NAME,
        "description": "Tells the Rich Presence Plus extension how to reach the desktop app.",
        "path": exe.to_string_lossy(),
        "type": "stdio",
    });
    let (field, allowed): (&str, Vec<String>) = match family {
        Family::Chromium => (
            "allowed_origins",
            CHROMIUM_EXTENSIONS
                .iter()
                .map(|id| format!("chrome-extension://{id}/"))
                .collect(),
        ),
        Family::Firefox => (
            "allowed_extensions",
            FIREFOX_EXTENSIONS
                .iter()
                .map(|id| (*id).to_owned())
                .collect(),
        ),
    };
    manifest[field] = json!(allowed);
    manifest
}

/// The extension that started this program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caller {
    /// Which kind of browser started it.
    pub family: Family,
    /// The extension ID or add-on ID.
    pub id: String,
}

impl Caller {
    /// Tells whether this extension may receive the pairing.
    pub fn allowed(&self) -> bool {
        match self.family {
            Family::Chromium => CHROMIUM_EXTENSIONS.contains(&self.id.as_str()),
            Family::Firefox => FIREFOX_EXTENSIONS.contains(&self.id.as_str()),
        }
    }
}

/// A Chrome extension ID: 32 letters from `a` to `p`.
fn chromium_id(origin: &str) -> Option<&str> {
    let id = origin
        .strip_prefix("chrome-extension://")?
        .strip_suffix('/')?;
    (id.len() == 32 && id.bytes().all(|b| (b'a'..=b'p').contains(&b))).then_some(id)
}

/// A Firefox add-on ID: `name@domain` or a braced UUID.
fn firefox_id(text: &str) -> bool {
    let braced = text.len() == 38
        && text.starts_with('{')
        && text.ends_with('}')
        && text[1..37]
            .bytes()
            .all(|b| b.is_ascii_hexdigit() || b == b'-');
    let named = text.len() <= 80
        && text.split_once('@').is_some_and(|(name, domain)| {
            !name.is_empty()
                && !domain.is_empty()
                && text
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"@.-_".contains(&b))
        });
    braced || named
}

/// Tells whether a browser started this program to answer native messages.
///
/// Args:
///     args: The program's arguments, without the program itself.
///
/// Returns:
///     The extension that asked, or `None` for a normal start.
pub fn caller(args: &[String]) -> Option<Caller> {
    let first = args.first()?;
    if let Some(id) = chromium_id(first) {
        return Some(Caller {
            family: Family::Chromium,
            id: id.to_owned(),
        });
    }
    let second = args.get(1)?;
    (first.to_ascii_lowercase().ends_with(".json") && firefox_id(second)).then(|| Caller {
        family: Family::Firefox,
        id: second.clone(),
    })
}

/// Why a native message could not be read or written.
#[derive(Debug, thiserror::Error)]
pub enum NativeError {
    /// The browser closed the stream before sending a message.
    #[error("the browser sent nothing")]
    Closed,
    /// The announced length is above the limit.
    #[error("the message announces {0} bytes, above the limit")]
    TooLarge(u32),
    /// Reading or writing the stream failed.
    #[error("native messaging stream: {0}")]
    Io(#[from] io::Error),
}

/// Reads the body of one framed message of at most `limit` bytes.
///
/// Raises:
///     NativeError: The stream ends, the length is above `limit`, or reading fails.
pub fn read_message<R: Read>(reader: &mut R, limit: u32) -> Result<Vec<u8>, NativeError> {
    let mut length = [0_u8; 4];
    match reader.read_exact(&mut length) {
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => {
            return Err(NativeError::Closed);
        }
        other => other?,
    }
    let length = u32::from_ne_bytes(length);
    if length > limit {
        return Err(NativeError::TooLarge(length));
    }
    let mut body = vec![0_u8; length as usize];
    reader.read_exact(&mut body)?;
    Ok(body)
}

/// Writes one framed message and flushes it.
pub fn write_message<W: Write>(writer: &mut W, message: &Value) -> Result<(), NativeError> {
    let body = message.to_string();
    // Answers are a few hundred bytes; the browsers accept up to 1 MiB.
    let length = u32::try_from(body.len()).map_err(|_| NativeError::TooLarge(u32::MAX))?;
    writer.write_all(&length.to_ne_bytes())?;
    writer.write_all(body.as_bytes())?;
    writer.flush()?;
    Ok(())
}

/// The port and token the running bridge uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pairing {
    /// Loopback port of the bridge.
    pub port: u16,
    /// Pairing token, 64 lowercase hexadecimal characters.
    pub token: String,
}

impl Pairing {
    fn valid(&self) -> bool {
        self.port > 0
            && self.token.len() == 64
            && self
                .token
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }
}

/// Records the bridge's port and token for the native host.
pub fn save_pairing(dir: &Path, pairing: &Pairing) -> io::Result<()> {
    let text = json!({ "port": pairing.port, "token": pairing.token }).to_string();
    let path = dir.join(PAIRING_FILE);
    let temporary = dir.join(format!("{PAIRING_FILE}.tmp"));
    fs::create_dir_all(dir)?;
    fs::write(&temporary, text)?;
    fs::rename(&temporary, &path)
}

/// Reads what [`save_pairing`] wrote; `None` when the file is missing, too
/// large or malformed.
pub fn load_pairing(dir: &Path) -> Option<Pairing> {
    let path = dir.join(PAIRING_FILE);
    if fs::metadata(&path).ok()?.len() > MAX_PAIRING_FILE {
        return None;
    }
    let value: Value = serde_json::from_str(&fs::read_to_string(path).ok()?).ok()?;
    let pairing = Pairing {
        port: u16::try_from(value.get("port")?.as_u64()?).ok()?,
        token: value.get("token")?.as_str()?.to_owned(),
    };
    pairing.valid().then_some(pairing)
}

/// The answer to one request from the extension: `pairing` with the port and
/// token, `absent` when the application has not recorded one, or `error` for
/// a request it does not know.
pub fn answer(request: &[u8], pairing: Option<&Pairing>) -> Value {
    let kind = serde_json::from_slice::<Value>(request)
        .ok()
        .and_then(|value| value.get("type")?.as_str().map(str::to_owned));
    match (kind.as_deref(), pairing) {
        (Some("pairing"), Some(pairing)) => {
            json!({ "type": "pairing", "port": pairing.port, "token": pairing.token })
        }
        (Some("pairing"), None) => json!({ "type": "absent" }),
        _ => json!({ "type": "error", "reason": "unknown request" }),
    }
}

/// Answers one request from the browser.
///
/// Raises:
///     NativeError: The request cannot be read or the answer written.
pub fn serve<R: Read, W: Write>(
    caller: &Caller,
    input: &mut R,
    output: &mut W,
    dir: &Path,
) -> Result<(), NativeError> {
    let request = read_message(input, MAX_REQUEST)?;
    let reply = if caller.allowed() {
        answer(&request, load_pairing(dir).as_ref())
    } else {
        json!({ "type": "refused" })
    };
    write_message(output, &reply)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|a| (*a).to_owned()).collect()
    }

    fn framed(body: &[u8]) -> Vec<u8> {
        let mut bytes = (body.len() as u32).to_ne_bytes().to_vec();
        bytes.extend_from_slice(body);
        bytes
    }

    fn chrome() -> Caller {
        Caller {
            family: Family::Chromium,
            id: CHROMIUM_EXTENSIONS[0].into(),
        }
    }

    #[test]
    fn chrome_and_edge_pass_the_origin_first() {
        let origin = format!("chrome-extension://{}/", CHROMIUM_EXTENSIONS[0]);
        let found = caller(&args(&[&origin, "--parent-window=0"])).unwrap();
        assert_eq!(found, chrome());
        assert!(found.allowed());
        let other = caller(&args(&[
            "chrome-extension://abcdefghijklmnopabcdefghijklmnop/",
        ]))
        .unwrap();
        assert!(!other.allowed());
    }

    #[test]
    fn the_chrome_web_store_copy_may_pair() {
        let found = caller(&args(&[
            "chrome-extension://cjdkaikgiokfekdbckccmlbffinaknib/",
        ]))
        .unwrap();
        assert!(found.allowed());
    }

    #[test]
    fn firefox_passes_the_manifest_and_the_add_on_id() {
        let found = caller(&args(&[
            r"C:\Users\a\AppData\Local\rpp\native-host-firefox.json",
            FIREFOX_EXTENSIONS[0],
        ]))
        .unwrap();
        assert_eq!(found.family, Family::Firefox);
        assert!(found.allowed());
        let braced = caller(&args(&["x.JSON", "{12345678-1234-1234-1234-123456789abc}"])).unwrap();
        assert!(!braced.allowed());
    }

    #[test]
    fn a_normal_start_is_not_a_native_call() {
        for list in [
            vec![],
            vec!["--hidden"],
            vec!["chrome-extension://short/"],
            vec!["chrome-extension://ABCDEFGHIJKLMNOPABCDEFGHIJKLMNOP/"],
            vec!["chrome-extension://abcdefghijklmnopabcdefghijklmnop"],
            vec!["notes.json"],
            vec!["notes.txt", "a@b.c"],
            vec!["notes.json", "no-at-sign"],
            vec!["notes.json", "a b@c.d"],
        ] {
            assert_eq!(caller(&args(&list)), None, "{list:?}");
        }
    }

    #[test]
    fn messages_are_framed_with_a_native_endian_length() {
        let mut out = Vec::new();
        write_message(&mut out, &json!({"type": "absent"})).unwrap();
        let body = br#"{"type":"absent"}"#;
        assert_eq!(out, framed(body));
        assert_eq!(
            read_message(&mut Cursor::new(out), MAX_REQUEST).unwrap(),
            body
        );
    }

    #[test]
    fn reading_stops_at_the_limit_and_at_the_end() {
        let big = (MAX_REQUEST + 1).to_ne_bytes().to_vec();
        assert!(matches!(
            read_message(&mut Cursor::new(big), MAX_REQUEST),
            Err(NativeError::TooLarge(n)) if n == MAX_REQUEST + 1
        ));
        assert!(matches!(
            read_message(&mut Cursor::new(Vec::new()), MAX_REQUEST),
            Err(NativeError::Closed)
        ));
        // A body shorter than announced is an error, not a short message.
        let mut cut = framed(b"{\"type\":\"pairing\"}");
        cut.truncate(10);
        assert!(matches!(
            read_message(&mut Cursor::new(cut), MAX_REQUEST),
            Err(NativeError::Io(_))
        ));
    }

    #[test]
    fn the_pairing_file_round_trips_and_rejects_bad_content() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load_pairing(dir.path()), None);
        let pairing = Pairing {
            port: 38417,
            token: TOKEN.into(),
        };
        save_pairing(dir.path(), &pairing).unwrap();
        assert_eq!(load_pairing(dir.path()), Some(pairing));
        assert!(!dir.path().join("pairing.json.tmp").exists());

        for bad in [
            "{",
            r#"{"port": 0, "token": "TOKEN"}"#,
            r#"{"port": 70000, "token": "TOKEN"}"#,
            r#"{"port": 1, "token": "short"}"#,
            r#"{"port": 1, "token": "TOKEN-UPPER"}"#,
        ] {
            let text = bad
                .replace("TOKEN-UPPER", &TOKEN.to_uppercase())
                .replace("TOKEN", TOKEN);
            fs::write(dir.path().join(PAIRING_FILE), text).unwrap();
            assert_eq!(load_pairing(dir.path()), None, "{bad}");
        }
        fs::write(dir.path().join(PAIRING_FILE), " ".repeat(5000)).unwrap();
        assert_eq!(load_pairing(dir.path()), None);
    }

    #[test]
    fn answers_depend_on_the_request_and_the_file() {
        let pairing = Pairing {
            port: 5,
            token: TOKEN.into(),
        };
        assert_eq!(
            answer(br#"{"type":"pairing"}"#, Some(&pairing)),
            json!({"type": "pairing", "port": 5, "token": TOKEN})
        );
        assert_eq!(
            answer(br#"{"type":"pairing"}"#, None),
            json!({"type": "absent"})
        );
        for request in [&b"{}"[..], b"not json", br#"{"type":"run"}"#] {
            assert_eq!(answer(request, Some(&pairing))["type"], "error");
        }
    }

    #[test]
    fn serving_answers_allowed_callers_only() {
        let dir = tempfile::tempdir().unwrap();
        save_pairing(
            dir.path(),
            &Pairing {
                port: 9,
                token: TOKEN.into(),
            },
        )
        .unwrap();
        let request = framed(br#"{"type":"pairing"}"#);

        let mut out = Vec::new();
        serve(
            &chrome(),
            &mut Cursor::new(request.clone()),
            &mut out,
            dir.path(),
        )
        .unwrap();
        let reply: Value =
            serde_json::from_slice(&read_message(&mut Cursor::new(out), u32::MAX).unwrap())
                .unwrap();
        assert_eq!(reply["port"], 9);

        let stranger = Caller {
            family: Family::Chromium,
            id: "abcdefghijklmnopabcdefghijklmnop".into(),
        };
        let mut out = Vec::new();
        serve(&stranger, &mut Cursor::new(request), &mut out, dir.path()).unwrap();
        let text = String::from_utf8(out[4..].to_vec()).unwrap();
        assert_eq!(text, r#"{"type":"refused"}"#);
        assert!(!text.contains(TOKEN));
    }

    #[test]
    fn host_manifests_list_only_the_known_extensions() {
        let exe = Path::new(r"C:\Program Files\Rich Presence Plus\rpp-desktop.exe");
        let chromium = host_manifest(Family::Chromium, exe);
        assert_eq!(chromium["name"], HOST_NAME);
        assert_eq!(chromium["type"], "stdio");
        assert_eq!(chromium["path"], exe.to_string_lossy().as_ref());
        assert_eq!(
            chromium["allowed_origins"],
            json!([
                "chrome-extension://iijcphjgahgdkdeaibcgagjfoaedajjk/",
                "chrome-extension://cjdkaikgiokfekdbckccmlbffinaknib/",
            ])
        );
        assert!(chromium.get("allowed_extensions").is_none());
        let firefox = host_manifest(Family::Firefox, exe);
        assert_eq!(firefox["allowed_extensions"], json!(FIREFOX_EXTENSIONS));
        assert!(firefox.get("allowed_origins").is_none());
    }

    #[test]
    fn the_host_name_follows_the_browser_rules() {
        // Chrome accepts lowercase letters, digits, dots and underscores, and
        // no dot at either end or two in a row.
        assert!(
            HOST_NAME
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'_')
        );
        assert!(
            !HOST_NAME.starts_with('.') && !HOST_NAME.ends_with('.') && !HOST_NAME.contains("..")
        );
    }

    /// The development ID comes from the public key in the extension's
    /// manifest: the first 16 bytes of its SHA-256, each hex digit as `a`–`p`.
    #[test]
    fn the_extension_manifest_matches_the_allowed_ids() {
        use base64::Engine;
        use sha2::{Digest, Sha256};

        let text = include_str!("../../../extension/manifest.json");
        let manifest: Value = serde_json::from_str(text).unwrap();
        let key = base64::engine::general_purpose::STANDARD
            .decode(manifest["key"].as_str().unwrap())
            .unwrap();
        let id: String = Sha256::digest(&key)[..16]
            .iter()
            .flat_map(|byte| [byte >> 4, byte & 0xf])
            .map(|nibble| char::from(b'a' + nibble))
            .collect();
        assert_eq!(id, CHROMIUM_EXTENSIONS[0]);
        assert_eq!(
            manifest["browser_specific_settings"]["gecko"]["id"],
            FIREFOX_EXTENSIONS[0]
        );
        assert!(
            manifest["permissions"]
                .as_array()
                .unwrap()
                .contains(&json!("nativeMessaging"))
        );
    }
}
