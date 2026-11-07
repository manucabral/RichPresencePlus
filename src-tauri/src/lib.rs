//! The desktop shell: window, tray icon and the commands the interface calls.

mod commands;
mod idle;
mod logging;
mod native;
mod shell;
mod shortcut;
mod startup;
mod tray;
mod updates;
mod window;

/// Starts the application and blocks until it exits.
pub fn run() {
    shell::run();
}

/// Answers the browser extension when the browser started this program, and
/// returns the exit code; `None` for a normal start.
pub fn native_host() -> Option<i32> {
    native::serve_if_called()
}
