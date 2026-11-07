//! Start-up and exit: single instance, the controller, the window and the tray.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use rpp_app::{Controller, Options};
use rpp_catalog::{Catalog, Channel, Github, Key, Keys, keys};
use rpp_discord::DiscordClient;
use rpp_discord::pipe::{NamedPipeClient, PipeConnector};
use rpp_host::SystemHost;
use tauri::{AppHandle, Manager, RunEvent};

use crate::commands::{self, Shared};
use crate::logging::Logging;

/// The signed catalog of approved presences, installed into the user's folder.
fn open_catalog(data_dir: &std::path::Path) -> Option<Arc<Catalog>> {
    let source = Github::new(keys::REPOSITORY_OWNER, keys::REPOSITORY_NAME).ok()?;
    let trusted = Keys::parse(keys::TRUSTED).ok()?;
    Some(Arc::new(Catalog::new(
        Box::new(source),
        trusted,
        Channel::Stable,
        &data_dir.join("presences"),
        data_dir,
        Key::load_or_create(data_dir),
    )))
}

/// The Discord client the user chose, read by [`served_by_discord`].
static DISCORD_CLIENT: AtomicU8 = AtomicU8::new(0);

/// Makes the connector follow a choice of Discord client.
pub fn follow_discord_client(choice: DiscordClient) {
    DISCORD_CLIENT.store(choice.code(), Ordering::Relaxed);
}

/// Whether the chosen Discord client serves an opened pipe. A pipe whose server
/// Windows will not name is used only while the choice is automatic.
fn served_by_discord(pipe: &NamedPipeClient) -> Result<(), String> {
    use std::os::windows::io::AsRawHandle;
    let choice = DiscordClient::from_code(DISCORD_CLIENT.load(Ordering::Relaxed));
    match rpp_host::pipe_server_executable(pipe.as_raw_handle()) {
        Some(name) if choice.serves(&name) => Ok(()),
        Some(name) => Err(name),
        None if choice == DiscordClient::Auto => Ok(()),
        None => Err("Another program".to_owned()),
    }
}

/// Folders that hold presences: the user's own first, then the bundled ones.
fn presence_roots(app: &AppHandle, data_dir: &std::path::Path) -> Vec<PathBuf> {
    let mut roots = vec![data_dir.join("presences")];
    if let Ok(resources) = app.path().resource_dir() {
        roots.push(resources.join("presences"));
    }
    if cfg!(debug_assertions) {
        roots.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../presences"));
    }
    roots
}

/// Builds and runs the application.
pub fn run() {
    let application = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            crate::window::show(app);
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(crate::shortcut::plugin())
        .setup(|app| {
            let data_dir = app.path().app_local_data_dir()?;
            // The log starts before anything else so it records the start-up.
            let saved = rpp_config::SettingsStore::new(&data_dir).load();
            app.manage(Logging::start(&data_dir, &saved.settings.log_level));
            // Before the first connection, so it goes to the chosen client.
            follow_discord_client(saved.settings.discord_client);
            crate::logging::log_panics();
            // Each start writes the entries again, so a moved program is found.
            if let Err(error) = crate::native::register(&data_dir) {
                tracing::warn!("the browsers cannot start the pairing helper: {error}");
            }
            let options = Options {
                presence_roots: presence_roots(app.handle(), &data_dir),
                catalog: open_catalog(&data_dir),
                data_dir,
                version: app.package_info().version.to_string(),
                port_override: None,
                system_language: sys_locale::get_locale().unwrap_or_else(|| "en".to_owned()),
            };
            let controller = tauri::async_runtime::block_on(Controller::start(
                options,
                PipeConnector::verified(served_by_discord),
                SystemHost,
            ))?;
            app.manage(crate::window::WindowState::new(
                controller.minimize_to_tray(),
            ));
            let controller: Shared = Arc::new(controller);
            crate::idle::watch(&controller, app.handle().clone());
            app.manage::<Shared>(controller);
            crate::shortcut::register_saved(app.handle());
            crate::tray::build(app.handle())?;
            // Windows starts the program with `--hidden` to stay in the tray; the
            // window is created only when it is first shown.
            if !crate::startup::hidden_start() {
                crate::window::show(app.handle());
            }
            Ok(())
        })
        .on_window_event(crate::window::on_event)
        .invoke_handler(tauri::generate_handler![
            commands::overview,
            commands::presences,
            commands::set_presence_enabled,
            commands::set_presence_option,
            commands::log_level,
            commands::set_log_level,
            commands::author,
            commands::set_author,
            commands::logs,
            commands::preferences,
            commands::save_preferences,
            commands::pairing,
            commands::extension_sessions,
            commands::favorites,
            commands::pause_shortcut,
            commands::set_pause_shortcut,
            commands::discord_client,
            commands::set_discord_client,
            commands::export_manual_preset,
            commands::start_rotation,
            commands::stop_rotation,
            commands::import_manual_preset,
            commands::diagnostics,
            commands::set_favorite,
            commands::language,
            commands::set_language,
            commands::catalog,
            commands::install_presence,
            commands::delete_presence,
            commands::local_api,
            commands::set_local_api,
            commands::usage,
            commands::set_track_time,
            commands::export_usage,
            commands::clear_usage,
            commands::remove_presence,
            commands::restore_presence,
            commands::create_presence,
            commands::copy_presence,
            commands::create_program_presence,
            commands::running_programs,
            commands::reload_presences,
            commands::inspect_presence,
            commands::watch_presence,
            commands::dev_status,
            commands::open_legal,
            commands::open_link,
            commands::presence_versions,
            commands::save_version,
            commands::compare_version,
            commands::restore_version,
            commands::test_cases,
            commands::save_test_case,
            commands::delete_test_case,
            commands::run_test_cases,
            commands::review_presence,
            commands::prepare_presence,
            commands::has_code_editor,
            commands::open_in_code_editor,
            commands::presence_icons,
            commands::update_manifest,
            commands::open_presence_folder,
            commands::manual,
            commands::save_manual_preset,
            commands::delete_manual_preset,
            commands::show_manual,
            commands::profiles,
            commands::save_profile,
            commands::delete_profile,
            commands::apply_profile,
            crate::updates::check_update,
            crate::updates::install_update,
            crate::window::window_visible,
            crate::startup::startup,
            crate::startup::save_startup,
        ])
        .build(tauri::generate_context!());
    let Ok(application) = application else {
        eprintln!("Rich Presence Plus could not start");
        return;
    };
    application.run(|app, event| match event {
        // The window going away must not end the program; only the tray menu
        // does, and that exit carries a code.
        RunEvent::ExitRequested {
            code: None, api, ..
        } => api.prevent_exit(),
        RunEvent::Exit => {
            if let Some(controller) = app.try_state::<Shared>() {
                let controller = Arc::clone(&controller);
                // Removes the activity from Discord before the process ends.
                std::thread::scope(|scope| {
                    scope.spawn(|| tauri::async_runtime::block_on(controller.shutdown()));
                });
            }
        }
        _ => {}
    });
}
