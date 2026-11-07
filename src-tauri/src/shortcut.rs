//! The global shortcut that pauses or resumes publishing from any program.

use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::commands::Shared;

/// The plugin, with the handler every registered shortcut calls.
pub fn plugin() -> tauri::plugin::TauriPlugin<Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                toggle(app);
            }
        })
        .build()
}

/// Pauses or resumes, off the main thread, and tells the window and the tray.
fn toggle(app: &AppHandle) {
    let Some(controller) = app.try_state::<Shared>().map(|c| Shared::clone(&c)) else {
        return;
    };
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        match controller.toggle_pause() {
            Ok(paused) => tracing::info!(paused, "publishing toggled by the shortcut"),
            Err(error) => tracing::warn!("the pause shortcut failed: {error}"),
        }
        crate::tray::refresh(&app);
        let _ = app.emit("app-changed", ());
    });
}

/// Replaces the registered shortcut with one as the controller stores it.
///
/// Raises:
///     String: Windows refuses it, usually because another program holds it.
pub fn register(app: &AppHandle, shortcut: Option<&str>) -> Result<(), String> {
    let manager = app.global_shortcut();
    manager.unregister_all().map_err(|e| e.to_string())?;
    if let Some(shortcut) = shortcut {
        manager.register(shortcut).map_err(|_| {
            format!("Windows did not accept {shortcut}; another program may use it.")
        })?;
    }
    Ok(())
}

/// Registers the saved shortcut at start; a refusal is logged, not fatal.
pub fn register_saved(app: &AppHandle) {
    let Some(controller) = app.try_state::<Shared>() else {
        return;
    };
    if let Err(error) = register(app, controller.pause_shortcut().as_deref()) {
        tracing::warn!("{error}");
    }
}

#[cfg(test)]
mod tests {
    use tauri_plugin_global_shortcut::Shortcut;

    /// Every form the controller stores must be one the plugin reads.
    #[test]
    fn stored_shortcuts_are_understood_by_the_plugin() {
        for text in ["ctrl+alt+p", "shift+win+f12", "control+9", "alt+super+z"] {
            let stored = rpp_config::shortcut::normalize(text).unwrap();
            assert!(stored.parse::<Shortcut>().is_ok(), "{stored}");
        }
    }
}
