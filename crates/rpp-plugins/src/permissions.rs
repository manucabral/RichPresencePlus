//! What a script may ask the host to do, and the check that enforces it.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

/// Most lines a script can read from a file in one call.
pub const MAX_TAIL_LINES: usize = 500;
/// Most programs whose command line one presence may read.
pub const MAX_ARG_PROGRAMS: usize = 16;
/// Most options read from one program's command line.
pub const MAX_ARG_FLAGS: usize = 16;
/// Server query protocols the host speaks.
pub const QUERY_PROTOCOLS: [&str; 1] = ["samp"];

/// One thing a presence can reach. Two versions compare item by item, so an
/// update that asks for more shows exactly what it adds.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Grant {
    /// A hostname reachable over HTTPS.
    Http(String),
    /// The names of the running processes.
    Process,
    /// One option of a program's command line.
    Arg {
        /// Executable file name.
        program: String,
        /// The option, such as `-h`.
        option: String,
    },
    /// The foreground window.
    Window,
    /// The system media session.
    Media,
    /// A folder whose files can be read.
    File(String),
    /// A game server protocol.
    Query(String),
    /// A page pattern read through the browser extension.
    Site(String),
}

/// Capabilities a manifest can request. Everything is off by default.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Permissions {
    /// Hostnames reachable over HTTPS, matched exactly.
    pub http: Vec<String>,
    /// List the names of running processes.
    pub process: bool,
    /// Options read from the command line of a program, by executable name:
    /// `{"gta_sa.exe": ["-h", "-p"]}`. Only the values of the listed options
    /// reach the script, so a password or a session ticket passed on the same
    /// command line never does.
    pub args: BTreeMap<String, Vec<String>>,
    /// Read the process and title of the foreground window.
    pub window: bool,
    /// Read the current system media session.
    pub media: bool,
    /// Directories whose files may be read from the end. `%NAME%` stands for
    /// an environment variable and is replaced by [`Permissions::resolve`].
    pub files: Vec<String>,
    /// Game server protocols the script may use to ask a server for its
    /// public information, such as `samp`.
    pub query: Vec<String>,
}

/// A request from a script to the host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// GET an HTTPS address.
    HttpGet {
        /// Address to fetch.
        url: String,
    },
    /// Names of the running processes.
    ProcessList,
    /// Values of declared options on the command line of a running program.
    ProcessArgs {
        /// Executable file name, such as `gta_sa.exe`.
        name: String,
        /// Options to read; filled from the manifest, never by the script.
        flags: Vec<String>,
    },
    /// Foreground window.
    ForegroundWindow,
    /// Current media session.
    MediaSession,
    /// Newest file in a folder whose name ends with a suffix.
    LatestFile {
        /// Folder to search.
        dir: String,
        /// Ending of the file name, such as `_last.log`.
        suffix: String,
    },
    /// Last lines of a file.
    TailFile {
        /// File to read.
        path: String,
        /// Number of lines wanted.
        lines: usize,
    },
    /// Public information of a game server, asked with its own protocol.
    ServerQuery {
        /// Protocol name, such as `samp`.
        protocol: String,
        /// Server address or host name.
        host: String,
        /// Server port.
        port: u16,
    },
}

/// The host side of the capabilities; implemented by `rpp-host`.
pub trait Host {
    /// Performs an already authorized request and answers with JSON.
    ///
    /// Raises:
    ///     String: A message for the script if the request fails.
    fn call(&self, request: Request) -> Result<Value, String>;
}

/// A request the manifest did not allow.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("permission denied: {0}")]
pub struct Denied(pub String);

fn denied(what: impl Into<String>) -> Denied {
    Denied(what.into())
}

/// Lowercase with forward slashes, so Windows paths compare predictably.
fn normalize(path: &str) -> Option<String> {
    let text = path.replace('\\', "/").to_lowercase();
    let safe = !text.contains('\0') && text.split('/').all(|part| part != "..");
    safe.then(|| text.trim_end_matches('/').to_owned())
}

impl Permissions {
    /// One [`Grant`] per host, option, folder and capability.
    pub fn grants(&self) -> BTreeSet<Grant> {
        // Named one by one: a new permission must not stay out of this list unnoticed.
        let Self {
            http,
            process,
            args,
            window,
            media,
            files,
            query,
        } = self;
        let mut grants = BTreeSet::new();
        grants.extend(http.iter().cloned().map(Grant::Http));
        grants.extend(files.iter().cloned().map(Grant::File));
        grants.extend(query.iter().cloned().map(Grant::Query));
        for (program, options) in args {
            grants.extend(options.iter().map(|option| Grant::Arg {
                program: program.clone(),
                option: option.clone(),
            }));
        }
        grants.extend(process.then_some(Grant::Process));
        grants.extend(window.then_some(Grant::Window));
        grants.extend(media.then_some(Grant::Media));
        grants
    }

    /// Replaces `%NAME%` in the file directories.
    pub fn resolve(mut self, lookup: impl Fn(&str) -> Option<String>) -> Self {
        self.files = self
            .files
            .iter()
            .filter_map(|entry| {
                let mut out = String::new();
                let mut rest = entry.as_str();
                while let Some(start) = rest.find('%') {
                    let (before, after) = rest.split_at(start);
                    let end = after[1..].find('%')?;
                    out.push_str(before);
                    out.push_str(&lookup(&after[1..=end])?);
                    rest = &after[end + 2..];
                }
                out.push_str(rest);
                Some(out)
            })
            .collect();
        self
    }

    /// Allows or refuses one request of the script.
    ///
    /// Raises:
    ///     Denied: The manifest did not grant the capability, or the address,
    ///         folder or line count is outside what it allows.
    pub fn check(&self, request: &Request) -> Result<(), Denied> {
        match request {
            Request::HttpGet { url } => self.check_http(url),
            Request::ProcessList => self.process.then_some(()).ok_or_else(|| denied("process")),
            Request::ProcessArgs { name, flags } => self.check_args(name, flags),
            Request::ForegroundWindow => self.window.then_some(()).ok_or_else(|| denied("window")),
            Request::MediaSession => self.media.then_some(()).ok_or_else(|| denied("media")),
            Request::TailFile { path, lines } => self.check_file(path, *lines),
            Request::LatestFile { dir, suffix } => self.check_folder(dir, suffix),
            Request::ServerQuery {
                protocol,
                host,
                port,
            } => self.check_query(protocol, host, *port),
        }
    }

    /// The options declared for a program, matched without regard to case.
    pub fn args_of(&self, name: &str) -> Option<&[String]> {
        self.args
            .iter()
            .find(|(program, _)| program.eq_ignore_ascii_case(name))
            .map(|(_, flags)| flags.as_slice())
    }

    /// Completes a request with what only the manifest may say: the options a
    /// command-line read returns come from here, not from the script.
    pub fn complete(&self, request: Request) -> Request {
        match request {
            Request::ProcessArgs { name, .. } => {
                let flags = self
                    .args_of(&name)
                    .map(<[String]>::to_vec)
                    .unwrap_or_default();
                Request::ProcessArgs { name, flags }
            }
            other => other,
        }
    }

    fn check_args(&self, name: &str, flags: &[String]) -> Result<(), Denied> {
        let plain = !name.is_empty()
            && name.len() <= 64
            && !name.contains(['/', '\\', ':', '\0'])
            && name != "."
            && name != "..";
        let declared = self.args_of(name).filter(|_| plain);
        // Every option asked for must be one the manifest lists for that program.
        match declared {
            Some(allowed) if !flags.is_empty() && flags.iter().all(|f| allowed.contains(f)) => {
                Ok(())
            }
            _ => Err(denied(format!("args: {name}"))),
        }
    }

    fn check_query(&self, protocol: &str, host: &str, port: u16) -> Result<(), Denied> {
        if !self.query.iter().any(|allowed| allowed == protocol) {
            return Err(denied(format!("query: {protocol}")));
        }
        let host_ok = !host.is_empty()
            && host.len() <= 253
            && host
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-');
        if !host_ok || port == 0 {
            return Err(denied("query: address"));
        }
        Ok(())
    }

    fn check_http(&self, address: &str) -> Result<(), Denied> {
        let url = Url::parse(address).map_err(|_| denied("http: invalid address"))?;
        let plain = url.scheme() == "https"
            && url.username().is_empty()
            && url.password().is_none()
            && url.port().is_none();
        let host = url.host_str().unwrap_or_default().to_lowercase();
        if plain
            && self
                .http
                .iter()
                .any(|allowed| allowed.to_lowercase() == host)
        {
            Ok(())
        } else {
            Err(denied(format!("http: {host}")))
        }
    }

    fn check_folder(&self, dir: &str, suffix: &str) -> Result<(), Denied> {
        if suffix.is_empty() || suffix.len() > 32 || suffix.contains(['/', '\\', '\0']) {
            return Err(denied("files: suffix"));
        }
        let candidate = normalize(dir).ok_or_else(|| denied("files: path"))?;
        let inside = self
            .files
            .iter()
            .filter_map(|allowed| normalize(allowed))
            .any(|allowed| {
                !allowed.is_empty()
                    && (candidate == allowed
                        || candidate
                            .strip_prefix(&allowed)
                            .is_some_and(|rest| rest.starts_with('/')))
            });
        if inside {
            Ok(())
        } else {
            Err(denied("files: outside the allowed folders"))
        }
    }

    fn check_file(&self, path: &str, lines: usize) -> Result<(), Denied> {
        if lines == 0 || lines > MAX_TAIL_LINES {
            return Err(denied("files: line count"));
        }
        let candidate = normalize(path).ok_or_else(|| denied("files: path"))?;
        let inside = self
            .files
            .iter()
            .filter_map(|dir| normalize(dir))
            .any(|dir| {
                !dir.is_empty()
                    && candidate
                        .strip_prefix(&dir)
                        .is_some_and(|rest| rest.starts_with('/'))
            });
        if inside {
            Ok(())
        } else {
            Err(denied("files: outside the allowed folders"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn http(url: &str) -> Request {
        Request::HttpGet { url: url.into() }
    }

    #[test]
    fn grants_name_each_host_option_and_capability() {
        let permissions = Permissions {
            http: vec!["a.example".into()],
            process: true,
            args: BTreeMap::from([("game.exe".into(), vec!["-h".into(), "-p".into()])]),
            window: true,
            media: true,
            files: vec!["%TEMP%".into()],
            query: vec!["samp".into()],
        };
        let grants: Vec<Grant> = permissions.grants().into_iter().collect();
        assert_eq!(grants.len(), 8, "{grants:?}");
        assert!(grants.contains(&Grant::Arg {
            program: "game.exe".into(),
            option: "-p".into()
        }));
        assert!(Permissions::default().grants().is_empty());
    }

    fn tail(path: &str, lines: usize) -> Request {
        Request::TailFile {
            path: path.into(),
            lines,
        }
    }

    #[test]
    fn nothing_is_allowed_by_default() {
        let none = Permissions::default();
        for request in [
            http("https://a.io/x"),
            Request::ProcessList,
            Request::ProcessArgs {
                name: "gta_sa.exe".into(),
                flags: vec!["-h".into()],
            },
            Request::ForegroundWindow,
            Request::MediaSession,
            tail("C:/logs/a.log", 10),
            Request::ServerQuery {
                protocol: "samp".into(),
                host: "127.0.0.1".into(),
                port: 7777,
            },
        ] {
            assert!(none.check(&request).is_err(), "{request:?}");
        }
    }

    fn args_permissions() -> Permissions {
        Permissions {
            args: BTreeMap::from([("gta_sa.exe".into(), vec!["-h".into(), "-p".into()])]),
            ..Default::default()
        }
    }

    #[test]
    fn only_declared_options_of_a_declared_program_can_be_read() {
        let allowed = args_permissions();
        let args = |name: &str, flags: &[&str]| Request::ProcessArgs {
            name: name.into(),
            flags: flags.iter().map(|f| (*f).to_owned()).collect(),
        };
        assert!(allowed.check(&args("GTA_SA.EXE", &["-h", "-p"])).is_ok());
        // A password option the manifest did not list is refused.
        assert!(allowed.check(&args("gta_sa.exe", &["-h", "-z"])).is_err());
        assert!(allowed.check(&args("gta_sa.exe", &[])).is_err());
        for bad in [
            "other.exe",
            "",
            "C:/gta_sa.exe",
            "sub\\gta_sa.exe",
            "..",
            "gta_sa.exe\0",
        ] {
            assert!(allowed.check(&args(bad, &["-h"])).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn the_options_come_from_the_manifest_not_the_script() {
        let allowed = args_permissions();
        let asked = Request::ProcessArgs {
            name: "Gta_Sa.exe".into(),
            flags: vec!["-z".into()],
        };
        assert_eq!(
            allowed.complete(asked),
            Request::ProcessArgs {
                name: "Gta_Sa.exe".into(),
                flags: vec!["-h".into(), "-p".into()],
            }
        );
        let unknown = allowed.complete(Request::ProcessArgs {
            name: "other.exe".into(),
            flags: vec!["-h".into()],
        });
        assert!(allowed.check(&unknown).is_err());
    }

    #[test]
    fn queries_need_a_declared_protocol_and_a_plain_address() {
        let allowed = Permissions {
            query: vec!["samp".into()],
            ..Default::default()
        };
        let query = |protocol: &str, host: &str, port: u16| Request::ServerQuery {
            protocol: protocol.into(),
            host: host.into(),
            port,
        };
        assert!(allowed.check(&query("samp", "144.217.0.1", 7777)).is_ok());
        assert!(
            allowed
                .check(&query("samp", "play.example.com", 7777))
                .is_ok()
        );
        for bad in [
            query("source", "1.2.3.4", 27015),
            query("samp", "", 7777),
            query("samp", "1.2.3.4:80", 7777),
            query("samp", "a b", 7777),
            query("samp", "1.2.3.4", 0),
        ] {
            assert!(allowed.check(&bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn http_needs_https_and_an_exact_host() {
        let allowed = Permissions {
            http: vec!["api.example.com".into()],
            ..Default::default()
        };
        assert!(allowed.check(&http("https://API.example.com/v1")).is_ok());
        for url in [
            "http://api.example.com/",
            "https://example.com/",
            "https://evil.api.example.com/",
            "https://api.example.com.evil.io/",
            "https://user:pw@api.example.com/",
            "https://api.example.com:8443/",
            "not a url",
        ] {
            assert!(allowed.check(&http(url)).is_err(), "{url}");
        }
    }

    #[test]
    fn files_stay_inside_the_declared_folders() {
        let allowed = Permissions {
            files: vec!["C:\\Users\\me\\Logs".into()],
            ..Default::default()
        };
        assert!(
            allowed
                .check(&tail("c:/users/me/logs/game.log", 50))
                .is_ok()
        );
        for path in [
            "C:/Users/me/Logs/../Secrets/key.txt",
            "C:/Users/me/LogsExtra/a.log",
            "C:/Users/me/Logs",
            "D:/Users/me/Logs/a.log",
        ] {
            assert!(allowed.check(&tail(path, 5)).is_err(), "{path}");
        }
        assert!(allowed.check(&tail("c:/users/me/logs/a.log", 0)).is_err());
        assert!(
            allowed
                .check(&tail("c:/users/me/logs/a.log", MAX_TAIL_LINES + 1))
                .is_err()
        );
    }

    #[test]
    fn latest_file_needs_an_allowed_folder_and_a_plain_suffix() {
        let allowed = Permissions {
            files: vec!["C:/Users/me/Logs".into()],
            ..Default::default()
        };
        let latest = |dir: &str, suffix: &str| Request::LatestFile {
            dir: dir.into(),
            suffix: suffix.into(),
        };
        assert!(
            allowed
                .check(&latest("c:/users/me/logs", "_last.log"))
                .is_ok()
        );
        assert!(
            allowed
                .check(&latest("c:/users/me/logs/sub", ".log"))
                .is_ok()
        );
        for (dir, suffix) in [
            ("c:/users/me", ".log"),
            ("c:/users/me/logs/../x", ".log"),
            ("c:/users/me/logs", "../x"),
            ("c:/users/me/logs", ""),
        ] {
            assert!(
                allowed.check(&latest(dir, suffix)).is_err(),
                "{dir} {suffix}"
            );
        }
    }

    #[test]
    fn variables_are_expanded_and_unknown_ones_drop_the_entry() {
        let permissions = Permissions {
            files: vec!["%LOCALAPPDATA%/Roblox/logs".into(), "%MISSING%/x".into()],
            ..Default::default()
        };
        let resolved = permissions
            .resolve(|name| (name == "LOCALAPPDATA").then(|| "C:/Users/me/AppData/Local".into()));
        assert_eq!(resolved.files, ["C:/Users/me/AppData/Local/Roblox/logs"]);
    }
}

#[cfg(test)]
mod random_input {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn a_path_that_climbs_is_never_normal(
            before in "([a-zA-Z0-9]{1,6}[/\\\\]){0,4}",
            after in "([/\\\\][a-zA-Z0-9]{1,6}){0,4}",
        ) {
            prop_assert_eq!(normalize(&format!("{before}..{after}")), None);
        }

        #[test]
        fn any_text_is_safe_as_a_path(path in "\\PC{0,200}") {
            if let Some(clean) = normalize(&path) {
                prop_assert!(!clean.split('/').any(|part| part == ".."));
                prop_assert!(!clean.contains('\\'));
            }
        }
    }
}
