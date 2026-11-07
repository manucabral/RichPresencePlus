//! Entry point; all behavior lives in the library.

// Hides the console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // The browser starts this same program to ask for the pairing; it answers
    // and exits before the window or the single-instance check exist.
    if let Some(code) = rpp_desktop::native_host() {
        std::process::exit(code);
    }
    rpp_desktop::run();
}
