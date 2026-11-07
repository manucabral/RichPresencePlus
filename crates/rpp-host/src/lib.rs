//! The real implementation of the capabilities a presence script can request.

pub mod args;
pub mod files;
// Only the Windows sources use it; the tests run everywhere.
#[cfg_attr(not(windows), allow(dead_code))]
mod fresh;
pub mod http;

#[cfg(windows)]
mod media;
#[cfg(windows)]
pub mod native_host;
pub mod query;
mod samp;
#[cfg(windows)]
pub mod startup;
pub mod usage;
#[cfg(windows)]
mod win32;

#[cfg(windows)]
pub use win32::set_low_memory;

/// Hands the memory this process is not using back to Windows. Does nothing
/// elsewhere.
pub fn trim_memory() {
    #[cfg(windows)]
    win32::trim_memory();
}

use rpp_plugins::{Host, Request};
use serde_json::Value;

/// Executable name of the program serving a named pipe this process opened;
/// `None` if Windows does not say.
#[cfg(windows)]
pub fn pipe_server_executable(pipe: std::os::windows::io::RawHandle) -> Option<String> {
    win32::pipe_server_executable(pipe)
}

/// Time since the last keyboard or mouse input; `None` when it cannot be read
/// or off Windows.
pub fn idle_time() -> Option<std::time::Duration> {
    #[cfg(windows)]
    return win32::idle_time();
    #[cfg(not(windows))]
    None
}

/// Minutes after midnight on the computer's clock; `None` off Windows.
pub fn local_minute() -> Option<u16> {
    #[cfg(windows)]
    return Some(win32::local_minute());
    #[cfg(not(windows))]
    None
}

/// Sends a folder to the Recycle Bin; always fails off Windows.
pub fn to_recycle_bin(folder: &std::path::Path) -> Result<(), String> {
    #[cfg(windows)]
    return win32::to_recycle_bin(folder);
    #[cfg(not(windows))]
    {
        let _ = folder;
        Err("this is only available on Windows".to_owned())
    }
}

/// Today on the computer's clock as `YYYY-MM-DD`; `None` off Windows.
pub fn local_day() -> Option<String> {
    #[cfg(windows)]
    return Some(win32::local_day());
    #[cfg(not(windows))]
    None
}

/// Host that talks to the operating system and the network.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemHost;

/// Programs change slowly next to how often presences poll, so presences that
/// poll in the same second share one snapshot of the process list.
#[cfg(windows)]
static PROCESSES: fresh::Fresh<Vec<String>> = fresh::Fresh::new(std::time::Duration::from_secs(1));

impl Host for SystemHost {
    /// Performs a request that the manifest already allowed.
    fn call(&self, request: Request) -> Result<Value, String> {
        match request {
            Request::HttpGet { url } => http::get(&url),
            Request::TailFile { path, lines } => files::tail(&path, lines),
            Request::LatestFile { dir, suffix } => files::latest(&dir, &suffix),
            Request::ServerQuery {
                protocol,
                host,
                port,
            } => query::query(&protocol, &host, port),
            #[cfg(windows)]
            Request::ProcessList => PROCESSES
                .get(std::time::Instant::now(), win32::process_names)
                .map(|names| names.into()),
            #[cfg(windows)]
            Request::ProcessArgs { name, flags } => win32::process_args(&name, &flags),
            #[cfg(windows)]
            Request::ForegroundWindow => win32::foreground(),
            #[cfg(windows)]
            Request::MediaSession => media::current(),
            #[cfg(not(windows))]
            Request::ProcessList
            | Request::ProcessArgs { .. }
            | Request::ForegroundWindow
            | Request::MediaSession => Err("this source is only available on Windows".into()),
        }
    }
}
