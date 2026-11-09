//! The main window: showing it, hiding it in the tray, and telling the
//! interface whether it is on screen.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Position, Size, State,
    WebviewWindow, WebviewWindowBuilder, WindowEvent,
};

/// How long a hidden window keeps its web view, so that opening it again right
/// after closing it stays instant.
const KEEP_HIDDEN: Duration = Duration::from_secs(10);

/// How long a destroyed web view takes to go away. The memory it used is only
/// worth handing back after that.
const TRIM_AFTER: Duration = Duration::from_secs(3);

/// Where a destroyed window was, for the next one to open in the same place.
#[derive(Debug, Clone, Copy)]
struct Placement {
    position: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
    maximized: bool,
}

/// What the window is doing and what the user chose for it.
#[derive(Debug, Default)]
pub struct WindowState {
    visible: AtomicBool,
    minimize_to_tray: AtomicBool,
    /// Moves with every show and hide; a release checks that it did not move.
    turn: AtomicU64,
    /// A web view is being built, on its own thread.
    building: AtomicBool,
    /// The web view was told to use less memory while minimized.
    low_memory: AtomicBool,
    placement: Mutex<Option<Placement>>,
}

impl WindowState {
    /// A state for a window that starts hidden.
    pub fn new(minimize_to_tray: bool) -> Self {
        Self {
            minimize_to_tray: AtomicBool::new(minimize_to_tray),
            ..Self::default()
        }
    }

    /// Applies a new choice for the minimize button.
    pub fn set_minimize_to_tray(&self, enabled: bool) {
        self.minimize_to_tray.store(enabled, Ordering::Relaxed);
    }

    /// Starts a new turn, which cancels any release waiting.
    fn next_turn(&self) -> u64 {
        self.turn.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Tells whether no show or hide came after the given turn.
    fn is_turn(&self, turn: u64) -> bool {
        self.turn.load(Ordering::SeqCst) == turn
    }

    /// Records the minimized state, which Windows repeats on every resize, and
    /// returns it only when it changed.
    fn minimized_change(&self, minimized: bool) -> Option<bool> {
        (self.low_memory.swap(minimized, Ordering::Relaxed) != minimized).then_some(minimized)
    }

    fn remember(&self, placement: Placement) {
        if let Ok(mut kept) = self.placement.lock() {
            *kept = Some(placement);
        }
    }

    fn placement(&self) -> Option<Placement> {
        self.placement.lock().ok().and_then(|kept| *kept)
    }
}

/// The web view keeps reporting itself as visible after `hide()`, so the
/// interface is told when it is on screen and stops refreshing while it is not.
fn announce(window: &WebviewWindow, visible: bool) {
    if let Some(state) = window.try_state::<WindowState>() {
        state.visible.store(visible, Ordering::Relaxed);
    }
    let _ = window.emit("window-visible", visible);
}

fn reveal(window: &WebviewWindow) {
    announce(window, true);
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

/// Builds the window from `tauri.conf.json`, where the last one was.
fn build(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    let config = app
        .config()
        .app
        .windows
        .first()
        .ok_or(tauri::Error::WindowNotFound)?;
    let window = WebviewWindowBuilder::from_config(app, config)?.build()?;
    if let Some(place) = app
        .try_state::<WindowState>()
        .and_then(|state| state.placement())
    {
        let _ = window.set_size(Size::Physical(place.size));
        let _ = window.set_position(Position::Physical(place.position));
        if place.maximized {
            let _ = window.maximize();
        }
    }
    Ok(window)
}

/// Shows and focuses the main window, creating it if it does not exist.
pub fn show(app: &AppHandle) {
    let Some(state) = app.try_state::<WindowState>() else {
        return;
    };
    state.next_turn();
    if let Some(window) = app.get_webview_window("main") {
        reveal(&window);
        return;
    }
    if state.building.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    // Windows deadlocks when a web view is built on the thread of the event
    // loop, and this runs there for clicks on the tray.
    std::thread::spawn(move || {
        match build(&app) {
            Ok(window) => reveal(&window),
            Err(error) => tracing::warn!("cannot open the window: {error}"),
        }
        if let Some(state) = app.try_state::<WindowState>() {
            state.building.store(false, Ordering::SeqCst);
        }
    });
}

/// Destroys a hidden window after [`KEEP_HIDDEN`], unless it was shown again.
fn release_later(app: AppHandle, turn: u64) {
    std::thread::spawn(move || {
        std::thread::sleep(KEEP_HIDDEN);
        let handle = app.clone();
        // On the event loop, the same thread that handles a click on the tray:
        // a show cannot slip between the check and the destruction.
        let _ = app.run_on_main_thread(move || {
            let Some(state) = handle.try_state::<WindowState>() else {
                return;
            };
            if !state.is_turn(turn) {
                return;
            }
            if let Some(window) = handle.get_webview_window("main") {
                // A minimized window sits far off screen; the place it had
                // before is the one to keep.
                if !window.is_minimized().unwrap_or(false)
                    && let (Ok(position), Ok(size)) = (window.outer_position(), window.inner_size())
                {
                    state.remember(Placement {
                        position,
                        size,
                        maximized: window.is_maximized().unwrap_or(false),
                    });
                }
                let _ = window.destroy();
                trim_later(handle.clone(), turn);
            }
        });
    });
}

/// Hands back the memory the web view left behind, unless the window returned.
fn trim_later(app: AppHandle, turn: u64) {
    std::thread::spawn(move || {
        std::thread::sleep(TRIM_AFTER);
        if app
            .try_state::<WindowState>()
            .is_some_and(|state| state.is_turn(turn))
        {
            rpp_host::trim_memory();
        }
    });
}

/// Asks the web view to use less memory while the window stays minimized.
fn set_low_memory(webview: &WebviewWindow, low: bool) {
    let result = webview.with_webview(move |platform| {
        #[cfg(windows)]
        if let Err(error) = rpp_host::set_low_memory(&platform.controller(), low) {
            tracing::debug!("the memory target of the web view stays: {error}");
        }
        #[cfg(not(windows))]
        let _ = (platform, low);
    });
    if let Err(error) = result {
        tracing::debug!("cannot reach the web view: {error}");
    }
}

fn hide(window: &WebviewWindow) {
    let _ = window.hide();
    announce(window, false);
    if let Some(state) = window.try_state::<WindowState>() {
        release_later(window.app_handle().clone(), state.next_turn());
    }
}

/// Closing hides the window, and so does minimizing when the user chose it.
/// Publishing goes on either way; the tray menu quits.
pub fn on_event(window: &tauri::Window, event: &WindowEvent) {
    let Some(webview) = window.app_handle().get_webview_window(window.label()) else {
        return;
    };
    match event {
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            hide(&webview);
        }
        // Windows reports a minimize as a resize to nothing.
        WindowEvent::Resized(_) => {
            let Some(state) = window.try_state::<WindowState>() else {
                return;
            };
            let minimized = window.is_minimized().unwrap_or(false);
            if minimized && state.minimize_to_tray.load(Ordering::Relaxed) {
                hide(&webview);
            } else if let Some(low) = state.minimized_change(minimized) {
                // A minimized page still says it is visible, so the interface
                // would keep refreshing for nobody.
                announce(&webview, !low);
                set_low_memory(&webview, low);
            }
        }
        _ => {}
    }
}

/// Whether the window is on screen, for the interface to start right.
#[tauri::command]
pub fn window_visible(state: State<'_, WindowState>) -> bool {
    state.visible.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_show_or_a_second_hide_cancels_a_waiting_release() {
        let state = WindowState::new(false);
        let hidden = state.next_turn();
        assert!(state.is_turn(hidden));
        state.next_turn();
        assert!(!state.is_turn(hidden));
    }

    #[test]
    fn the_memory_target_changes_once_per_minimize_and_restore() {
        let state = WindowState::new(false);
        assert_eq!(state.minimized_change(false), None);
        assert_eq!(state.minimized_change(true), Some(true));
        assert_eq!(state.minimized_change(true), None);
        assert_eq!(state.minimized_change(false), Some(false));
        assert_eq!(state.minimized_change(false), None);
    }

    #[test]
    fn the_last_placement_is_kept() {
        let state = WindowState::new(false);
        assert!(state.placement().is_none());
        state.remember(Placement {
            position: PhysicalPosition::new(10, 20),
            size: PhysicalSize::new(800, 600),
            maximized: true,
        });
        let place = state.placement().unwrap();
        assert_eq!((place.position.x, place.size.height), (10, 600));
        assert!(place.maximized);
    }

    /// The window must not exist at the start: a start in the tray would pay
    /// for a web view nobody looks at.
    #[test]
    fn the_configuration_does_not_create_the_window_at_the_start() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let window = &config["app"]["windows"][0];
        assert_eq!(window["label"], "main");
        assert_eq!(window["create"], false);
        assert_eq!(window["visible"], false);
    }
}

#[cfg(test)]
mod browser_args {
    /// Setting these arguments replaces Tauri's defaults, so they are repeated.
    #[test]
    fn the_webview_stays_off_the_gpu_and_keeps_the_default_features() {
        let config = include_str!("../tauri.conf.json");
        assert!(config.contains("--disable-gpu"));
        assert!(config.contains("msWebOOUI,msPdfOOUI,msSmartScreenProtection"));
    }
}
