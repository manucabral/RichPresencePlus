//! The native messaging host: the browser starts this program to ask how to
//! reach the running application.

use std::io;
use std::path::{Path, PathBuf};

use rpp_bridge::native::{self, Family, HOST_NAME};
use rpp_host::native_host::{Browser, NativeHosts};

/// The application identifier in `tauri.conf.json`; it names the data folder.
pub const IDENTIFIER: &str = "com.manucabral.richpresenceplus";

/// Longest the host process lives; an answer takes a few milliseconds.
const HOST_DEADLINE: std::time::Duration = std::time::Duration::from_secs(10);

/// The data folder Tauri gives the application (`app_local_data_dir`).
fn data_dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|dir| PathBuf::from(dir).join(IDENTIFIER))
}

/// Answers the browser when it started this program.
///
/// Returns:
///     The exit code when a browser started the program; `None` for a normal start.
pub fn serve_if_called() -> Option<i32> {
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect();
    let caller = native::caller(&args)?;
    // Reading stdin has no timeout. A browser that opens the host and never
    // writes would leave this process waiting for good; this ends it.
    std::thread::spawn(|| {
        std::thread::sleep(HOST_DEADLINE);
        std::process::exit(2);
    });
    let Some(dir) = data_dir() else {
        return Some(1);
    };
    let served = native::serve(
        &caller,
        &mut io::stdin().lock(),
        &mut io::stdout().lock(),
        &dir,
    );
    Some(if served.is_ok() { 0 } else { 1 })
}

/// Writes the host manifests in `data_dir` and points Chrome, Edge and
/// Firefox to them.
pub fn register(data_dir: &Path) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
    for family in [Family::Chromium, Family::Firefox] {
        let text = serde_json::to_string_pretty(&native::host_manifest(family, &exe))
            .map_err(|e| e.to_string())?;
        std::fs::write(data_dir.join(family.manifest_file()), text).map_err(|e| e.to_string())?;
    }
    let hosts = NativeHosts::system(HOST_NAME);
    for browser in Browser::ALL {
        let family = if browser.is_firefox() {
            Family::Firefox
        } else {
            Family::Chromium
        };
        hosts.register(browser, &data_dir.join(family.manifest_file()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_identifier_is_the_one_tauri_uses() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(config["identifier"], IDENTIFIER);
    }
}
