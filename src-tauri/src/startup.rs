//! Starting this program with Windows, and the choices about the window.

use rpp_host::startup::{HIDDEN_ARGUMENT, Startup, StartupState, asked_by_installer};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::commands::Shared;
use crate::window::WindowState;

/// Whether Windows starts the running program, and how.
pub fn state() -> StartupState {
    std::env::current_exe()
        .map(|exe| Startup::system().state(&exe))
        .unwrap_or_default()
}

/// Makes the running program start with Windows, or stops it.
pub fn set(enabled: bool, hidden: bool) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    Startup::system().set(&exe, enabled.then_some(hidden))
}

/// Tells whether this run was started to stay in the tray.
pub fn hidden_start() -> bool {
    std::env::args()
        .skip(1)
        .any(|argument| argument == HIDDEN_ARGUMENT)
}

/// The choices shown in Settings and on the first run.
#[derive(Debug, Serialize)]
pub struct StartupView {
    /// Starts with Windows.
    pub enabled: bool,
    /// Starts hidden in the tray when it starts with Windows.
    pub hidden: bool,
    /// Minimizing hides the window in the tray.
    pub minimize_to_tray: bool,
    /// The user already answered, here or in the installer.
    pub asked: bool,
}

/// Reads how the application starts and where minimizing sends it.
#[tauri::command]
pub async fn startup(state: State<'_, Shared>) -> Result<StartupView, String> {
    let controller = Shared::clone(&state);
    tauri::async_runtime::spawn_blocking(move || {
        let current = self::state();
        StartupView {
            enabled: current.enabled,
            hidden: current.hidden,
            minimize_to_tray: controller.minimize_to_tray(),
            asked: controller.startup_asked() || asked_by_installer(),
        }
    })
    .await
    .map_err(|e| e.to_string())
}

/// Saves how the application starts and where minimizing sends it.
#[tauri::command]
pub async fn save_startup(
    app: AppHandle,
    state: State<'_, Shared>,
    window: State<'_, WindowState>,
    enabled: bool,
    hidden: bool,
    minimize_to_tray: bool,
) -> Result<(), String> {
    let controller = Shared::clone(&state);
    tauri::async_runtime::spawn_blocking(move || {
        set(enabled, hidden)?;
        controller.save_window_choices(minimize_to_tray)
    })
    .await
    .map_err(|e| e.to_string())??;
    window.set_minimize_to_tray(minimize_to_tray);
    crate::commands::refresh_tray(&app).await;
    // The first-run dialog and the Settings card read the new answer.
    let _ = app.emit("app-changed", ());
    Ok(())
}
