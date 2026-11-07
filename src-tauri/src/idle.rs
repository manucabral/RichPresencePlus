//! Work done every few seconds: the time without input, for the idle setting,
//! the next step of a rotation of custom statuses, and a look at the presences folder.

use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

use rpp_app::Controller;
use rpp_host::SystemHost;
use tauri::{AppHandle, Emitter};

/// How often the work runs. The idle setting counts in minutes and a rotation
/// step lasts at least 15 seconds, so a few seconds late is not noticed.
const EVERY: Duration = Duration::from_secs(5);

/// Starts the thread that keeps the idle reading and the rotation current.
///
/// Args:
///     app: Told when the presences change from outside, so the window reads again.
pub fn watch(controller: &Arc<Controller<SystemHost>>, app: AppHandle) {
    let controller: Weak<Controller<SystemHost>> = Arc::downgrade(controller);
    let started = std::thread::Builder::new()
        .name("ticker".into())
        .spawn(move || {
            // Off the start of the application: nobody waits for an editor's files.
            if let Some(controller) = controller.upgrade() {
                controller.prepare_editors();
            }
            while let Some(controller) = controller.upgrade() {
                controller.update_idle(rpp_host::idle_time());
                controller.update_quiet(rpp_host::local_minute());
                controller.make_requested_sites();
                if controller.notice_folder_changes() {
                    controller.prepare_editors();
                    let _ = app.emit("app-changed", ());
                    crate::tray::refresh(&app);
                }
                controller.track_time(rpp_host::local_day().as_deref(), Instant::now());
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |elapsed| elapsed.as_secs());
                controller.tick_rotation(now);
                drop(controller);
                std::thread::sleep(EVERY);
            }
        });
    if let Err(error) = started {
        tracing::warn!("the idle setting and status rotation will not work: {error}");
    }
}
