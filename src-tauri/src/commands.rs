//! Commands exposed to the interface. Errors are plain text for the user.

use std::path::Path;
use std::sync::Arc;

use rpp_app::views::{
    CaseResult, CatalogView, DevStatus, ExtensionSessions, FileChange, Grant, Inspection,
    InstallOutcome, LanguageView, LocalApiView, ManualView, Overview, Pairing, PreparedView,
    PresenceView, ReviewCheck, UsageView, VersionView,
};
use rpp_app::{Case, Controller, ManifestChanges, ProgramPresence, Template};
use rpp_config::{ManualPreset, Problem, Profile};
use rpp_core::Preferences;
use rpp_host::SystemHost;
use tauri::State;

use crate::logging::{LogView, Logging};

/// The controller, shared by every command.
pub type Shared = Arc<Controller<SystemHost>>;

/// Runs blocking work off the interface thread.
async fn blocking<T, F>(shared: &Shared, work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(Shared) -> T + Send + 'static,
{
    let shared = Arc::clone(shared);
    tauri::async_runtime::spawn_blocking(move || work(shared))
        .await
        .map_err(|e| e.to_string())
}

/// Like [`blocking`], then rebuilds the tray menu, which shows part of this state.
async fn changing<T, F>(app: &tauri::AppHandle, shared: &Shared, work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(Shared) -> T + Send + 'static,
{
    let result = blocking(shared, work).await;
    refresh_tray(app).await;
    result
}

/// Rebuilds the tray menu off the async workers: each item waits for the main
/// thread, and the state behind it takes locks.
pub async fn refresh_tray(app: &tauri::AppHandle) {
    let app = app.clone();
    if let Err(error) =
        tauri::async_runtime::spawn_blocking(move || crate::tray::refresh(&app)).await
    {
        tracing::debug!("the tray menu was not rebuilt: {error}");
    }
}

/// What the dashboard shows.
#[tauri::command]
pub async fn overview(state: State<'_, Shared>) -> Result<Overview, String> {
    blocking(&state, |app| app.overview()).await
}

/// Reads the diagnostics page: state, resources and the report to copy.
#[tauri::command]
pub async fn diagnostics(
    state: State<'_, Shared>,
) -> Result<rpp_app::diagnostics::Diagnostics, String> {
    blocking(&state, |app| {
        let measured = rpp_host::usage::app_usage().map(|usage| {
            let convert = |part: rpp_host::usage::Usage| rpp_app::diagnostics::Resources {
                memory_bytes: part.memory_bytes,
                cpu_seconds: part.cpu_seconds,
                processes: part.processes,
            };
            rpp_app::diagnostics::Measured {
                app: convert(usage.app),
                webview: convert(usage.webview),
            }
        });
        let home = std::env::var("USERPROFILE").ok();
        app.diagnostics(measured, home.as_deref())
    })
    .await
}

/// Shows saved custom statuses one after another.
#[tauri::command]
pub async fn start_rotation(
    state: State<'_, Shared>,
    names: Vec<String>,
    seconds: u32,
) -> Result<(), String> {
    blocking(&state, move |app| app.start_rotation(names, seconds)).await?
}

/// Ends the rotation of custom statuses.
#[tauri::command]
pub async fn stop_rotation(state: State<'_, Shared>) -> Result<(), String> {
    blocking(&state, |app| app.stop_rotation()).await?
}

/// A saved custom status as text to share.
#[tauri::command]
pub async fn export_manual_preset(
    state: State<'_, Shared>,
    name: String,
) -> Result<String, String> {
    blocking(&state, move |app| app.export_manual_preset(&name)).await?
}

/// Reads a shared custom status for review; nothing is saved.
#[tauri::command]
pub async fn import_manual_preset(
    state: State<'_, Shared>,
    text: String,
) -> Result<ManualPreset, String> {
    blocking(&state, move |app| app.import_manual_preset(&text)).await?
}

/// The shortcut that pauses publishing, if one is set.
#[tauri::command]
pub async fn pause_shortcut(state: State<'_, Shared>) -> Result<Option<String>, String> {
    blocking(&state, |app| app.pause_shortcut()).await
}

/// Sets the pause shortcut, or turns it off with `None`.
#[tauri::command]
pub async fn set_pause_shortcut(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    shortcut: Option<String>,
) -> Result<Option<String>, String> {
    blocking(&state, move |controller| {
        let clean = shortcut
            .as_deref()
            .map(rpp_config::shortcut::normalize)
            .transpose()?;
        let before = controller.pause_shortcut();
        crate::shortcut::register(&app, clean.as_deref())?;
        controller
            .set_pause_shortcut(clean.as_deref())
            .inspect_err(|_| {
                if let Err(error) = crate::shortcut::register(&app, before.as_deref()) {
                    tracing::warn!("the previous shortcut is not active: {error}");
                }
            })
    })
    .await?
}

/// Which Discord client receives the activity.
#[tauri::command]
pub async fn discord_client(
    state: State<'_, Shared>,
) -> Result<rpp_discord::DiscordClient, String> {
    blocking(&state, |app| app.discord_client()).await
}

/// Chooses the Discord client that receives the activity and moves it there.
#[tauri::command]
pub async fn set_discord_client(
    state: State<'_, Shared>,
    choice: rpp_discord::DiscordClient,
) -> Result<(), String> {
    blocking(&state, move |app| {
        // The connector follows first, so the reconnection reaches the new client.
        let before = app.discord_client();
        crate::shell::follow_discord_client(choice);
        app.set_discord_client(choice).inspect_err(|_| {
            crate::shell::follow_discord_client(before);
        })
    })
    .await?
}

/// Names of the presences the user marked as favorite.
#[tauri::command]
pub async fn favorites(state: State<'_, Shared>) -> Result<Vec<String>, String> {
    blocking(&state, |app| app.favorites()).await
}

/// Marks a presence as favorite, or not.
#[tauri::command]
pub async fn set_favorite(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    name: String,
    favorite: bool,
) -> Result<(), String> {
    // The tray lists the favorites, so it is rebuilt too.
    changing(&app, &state, move |app| app.set_favorite(&name, favorite)).await?
}

/// Browser extensions connected now and the last ones that were.
#[tauri::command]
pub async fn extension_sessions(state: State<'_, Shared>) -> Result<ExtensionSessions, String> {
    blocking(&state, |app| app.extension_sessions()).await
}

/// Every presence found, with its state and access.
#[tauri::command]
pub async fn presences(state: State<'_, Shared>) -> Result<Vec<PresenceView>, String> {
    blocking(&state, |app| app.presences()).await
}

/// Turns a presence on or off.
#[tauri::command]
pub async fn set_presence_enabled(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    name: String,
    enabled: bool,
) -> Result<(), String> {
    changing(&app, &state, move |app| app.set_enabled(&name, enabled)).await?
}

/// Saves the value of one option a presence declares.
#[tauri::command]
pub async fn set_presence_option(
    state: State<'_, Shared>,
    name: String,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    blocking(&state, move |app| app.set_option(&name, &key, value)).await?
}

/// Saves the sites the person added to a presence made for a server of their own.
#[tauri::command]
pub async fn set_presence_sites(
    state: State<'_, Shared>,
    name: String,
    sites: Vec<String>,
) -> Result<Vec<String>, String> {
    blocking(&state, move |app| app.set_sites(&name, &sites)).await?
}

/// The saved publishing preferences.
#[tauri::command]
pub async fn preferences(state: State<'_, Shared>) -> Result<Preferences, String> {
    blocking(&state, |app| app.preferences()).await
}

/// Saves new publishing preferences and returns them as stored.
#[tauri::command]
pub async fn save_preferences(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    preferences: Preferences,
) -> Result<Preferences, String> {
    changing(&app, &state, move |app| app.set_preferences(preferences)).await?
}

/// The code that pairs the browser extension.
#[tauri::command]
pub async fn pairing(state: State<'_, Shared>) -> Result<Pairing, String> {
    blocking(&state, |app| app.pairing()).await
}

/// The saved language and the language of the system.
#[tauri::command]
pub async fn language(state: State<'_, Shared>) -> Result<LanguageView, String> {
    blocking(&state, |app| app.language()).await
}

/// Saves the language; `auto` follows the system.
#[tauri::command]
pub async fn set_language(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    tag: String,
) -> Result<(), String> {
    changing(&app, &state, move |app| app.set_language(&tag)).await?
}

/// The approved presences and what is installed from them.
#[tauri::command]
pub async fn catalog(state: State<'_, Shared>, refresh: bool) -> Result<CatalogView, String> {
    blocking(&state, move |app| app.catalog(refresh)).await?
}

/// How many times each presence of the catalog was installed, by name.
#[tauri::command]
pub async fn catalog_installs(
    state: State<'_, Shared>,
) -> Result<std::collections::BTreeMap<String, u64>, String> {
    blocking(&state, |app| app.catalog_installs()).await
}

/// Installs or updates a presence from the catalog.
#[tauri::command]
pub async fn install_presence(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    name: String,
    accepted: Vec<Grant>,
) -> Result<InstallOutcome, String> {
    changing(&app, &state, move |app| {
        app.install_presence(&name, &accepted)
    })
    .await?
}

/// Returns whether the local address is on, and where.
#[tauri::command]
pub async fn local_api(state: State<'_, Shared>) -> Result<LocalApiView, String> {
    blocking(&state, |app| app.local_api()).await
}

/// Turns the local address on or off.
#[tauri::command]
pub async fn set_local_api(state: State<'_, Shared>, on: bool) -> Result<(), String> {
    blocking(&state, move |app| app.set_local_api(on)).await?
}

/// Returns what the "Your week" page shows.
#[tauri::command]
pub async fn usage(state: State<'_, Shared>) -> Result<UsageView, String> {
    blocking(&state, |app| app.usage()).await
}

/// Turns the count of time on display on or off.
#[tauri::command]
pub async fn set_track_time(state: State<'_, Shared>, on: bool) -> Result<(), String> {
    blocking(&state, move |app| app.set_track_time(on)).await?
}

/// Returns the time on display as a table.
#[tauri::command]
pub async fn export_usage(state: State<'_, Shared>) -> Result<String, String> {
    blocking(&state, |app| app.export_usage()).await
}

/// Deletes the time counted so far.
#[tauri::command]
pub async fn clear_usage(state: State<'_, Shared>) -> Result<(), String> {
    blocking(&state, |app| app.clear_usage()).await?
}

/// Sends a presence of the user's to the Recycle Bin.
#[tauri::command]
pub async fn delete_presence(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    name: String,
) -> Result<(), String> {
    changing(&app, &state, move |app| {
        app.delete_presence(&name, rpp_host::to_recycle_bin)
    })
    .await?
}

/// Removes a presence that was installed from the catalog.
#[tauri::command]
pub async fn remove_presence(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    name: String,
) -> Result<(), String> {
    changing(&app, &state, move |app| app.remove_presence(&name)).await?
}

/// The detail of the log file.
#[tauri::command]
pub async fn log_level(state: State<'_, Shared>) -> Result<String, String> {
    blocking(&state, |app| app.log_level()).await
}

/// The author written into new presences.
#[tauri::command]
pub async fn author(state: State<'_, Shared>) -> Result<rpp_app::views::AuthorView, String> {
    blocking(&state, |app| app.author()).await
}

/// Saves the author written into new presences.
#[tauri::command]
pub async fn set_author(state: State<'_, Shared>, author: String) -> Result<(), String> {
    blocking(&state, move |app| app.set_author(&author)).await?
}

/// Saves the detail of the log file and applies it at once.
#[tauri::command]
pub async fn set_log_level(
    state: State<'_, Shared>,
    logging: State<'_, Option<Logging>>,
    level: String,
) -> Result<(), String> {
    let saved = level.clone();
    blocking(&state, move |app| app.set_log_level(&saved)).await??;
    if let Some(logging) = logging.inner() {
        logging.set_level(&level);
    }
    Ok(())
}

/// The end of the newest log file and the folder that holds it.
#[tauri::command]
pub async fn logs(app: tauri::AppHandle) -> Result<LogView, String> {
    use tauri::Manager;
    tauri::async_runtime::spawn_blocking(move || {
        let logging = app.state::<Option<Logging>>();
        let logging = logging
            .inner()
            .as_ref()
            .ok_or("The log file could not be opened.")?;
        Ok(logging.view())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The saved manual presets and the one on display.
#[tauri::command]
pub async fn manual(state: State<'_, Shared>) -> Result<ManualView, String> {
    blocking(&state, |app| app.manual()).await
}

/// Saves a manual preset, replacing the one with the same name.
#[tauri::command]
pub async fn save_manual_preset(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    preset: ManualPreset,
) -> Result<(), String> {
    changing(&app, &state, move |app| app.save_manual_preset(preset)).await?
}

/// Deletes a manual preset.
#[tauri::command]
pub async fn delete_manual_preset(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    name: String,
) -> Result<(), String> {
    changing(&app, &state, move |app| app.delete_manual_preset(&name)).await?
}

/// Shows a manual preset, or stops showing the current one when `name` is empty.
#[tauri::command]
pub async fn show_manual(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    name: Option<String>,
) -> Result<(), String> {
    changing(&app, &state, move |app| app.show_manual(name.as_deref())).await?
}

/// The saved publishing profiles.
#[tauri::command]
pub async fn profiles(state: State<'_, Shared>) -> Result<Vec<Profile>, String> {
    blocking(&state, |app| app.profiles()).await
}

/// Saves the current rules and the presences that are off as a profile.
#[tauri::command]
pub async fn save_profile(state: State<'_, Shared>, name: String) -> Result<(), String> {
    blocking(&state, move |app| app.save_profile(&name)).await?
}

/// Deletes a profile.
#[tauri::command]
pub async fn delete_profile(state: State<'_, Shared>, name: String) -> Result<(), String> {
    blocking(&state, move |app| app.delete_profile(&name)).await?
}

/// Makes a profile the current rules.
#[tauri::command]
pub async fn apply_profile(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    name: String,
) -> Result<(), String> {
    changing(&app, &state, move |app| app.apply_profile(&name)).await?
}

/// Brings back an earlier version of a catalog presence.
#[tauri::command]
pub async fn restore_presence(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    name: String,
    version: String,
) -> Result<(), String> {
    changing(&app, &state, move |app| {
        app.restore_presence(&name, &version)
    })
    .await?
}

/// Creates a presence in the user's folder from a template; returns its folder.
#[tauri::command]
pub async fn create_presence(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    name: String,
    template: Template,
) -> Result<String, String> {
    changing(&app, &state, move |app| {
        app.create_presence(&name, template)
    })
    .await?
}

/// Creates a presence for a program, with no code; returns its folder.
#[tauri::command]
pub async fn create_program_presence(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    choice: ProgramPresence,
) -> Result<String, String> {
    changing(&app, &state, move |app| {
        app.create_program_presence(&choice)
    })
    .await?
}

/// The running programs a presence could be made for.
#[tauri::command]
pub async fn running_programs(state: State<'_, Shared>) -> Result<Vec<String>, String> {
    blocking(&state, |app| app.running_programs()).await?
}

/// Copies a presence into the user's folder under a new name; returns its folder.
#[tauri::command]
pub async fn copy_presence(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    name: String,
    new_name: String,
) -> Result<String, String> {
    changing(&app, &state, move |app| app.copy_presence(&name, &new_name)).await?
}

/// Reads the presence folders again; returns the folders that did not load.
#[tauri::command]
pub async fn reload_presences(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
) -> Result<Vec<Problem>, String> {
    changing(&app, &state, |app| app.reload_presences()).await
}

/// Runs a presence once and returns what it read and would show.
#[tauri::command]
pub async fn inspect_presence(
    state: State<'_, Shared>,
    name: String,
) -> Result<Inspection, String> {
    blocking(&state, move |app| app.inspect(&name)).await?
}

/// Starts or stops applying edits of a presence as they are saved.
#[tauri::command]
pub async fn watch_presence(state: State<'_, Shared>, name: Option<String>) -> Result<(), String> {
    blocking(&state, move |app| app.watch(name.as_deref())).await?
}

/// The saved versions of a presence, newest first.
#[tauri::command]
pub async fn presence_versions(
    state: State<'_, Shared>,
    name: String,
) -> Result<Vec<VersionView>, String> {
    blocking(&state, move |app| app.presence_versions(&name)).await?
}

/// Saves a presence as a new version.
#[tauri::command]
pub async fn save_version(
    state: State<'_, Shared>,
    name: String,
    notes: String,
) -> Result<VersionView, String> {
    blocking(&state, move |app| app.save_version(&name, &notes)).await?
}

/// Tells what changed in a presence since a saved version.
#[tauri::command]
pub async fn compare_version(
    state: State<'_, Shared>,
    name: String,
    id: u32,
) -> Result<Vec<FileChange>, String> {
    blocking(&state, move |app| app.compare_version(&name, id)).await?
}

/// Puts a saved version back.
#[tauri::command]
pub async fn restore_version(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    name: String,
    id: u32,
) -> Result<(), String> {
    changing(&app, &state, move |app| app.restore_version(&name, id)).await?
}

/// The test cases of a presence.
#[tauri::command]
pub async fn test_cases(state: State<'_, Shared>, name: String) -> Result<Vec<Case>, String> {
    blocking(&state, move |app| app.test_cases(&name)).await?
}

/// Keeps a test case, replacing the one with the same name.
#[tauri::command]
pub async fn save_test_case(
    state: State<'_, Shared>,
    name: String,
    case: Case,
) -> Result<(), String> {
    blocking(&state, move |app| app.save_test_case(&name, case)).await?
}

/// Removes a test case.
#[tauri::command]
pub async fn delete_test_case(
    state: State<'_, Shared>,
    name: String,
    case: String,
) -> Result<(), String> {
    blocking(&state, move |app| app.delete_test_case(&name, &case)).await?
}

/// Replays the test cases of a presence as it is now.
#[tauri::command]
pub async fn run_test_cases(
    state: State<'_, Shared>,
    name: String,
) -> Result<Vec<CaseResult>, String> {
    blocking(&state, move |app| app.run_test_cases(&name)).await?
}

/// Checks a presence against the rules of the catalog.
#[tauri::command]
pub async fn review_presence(
    state: State<'_, Shared>,
    name: String,
) -> Result<Vec<ReviewCheck>, String> {
    blocking(&state, move |app| app.review_presence(&name)).await?
}

/// Writes a clean copy of a presence and shows it in the file manager.
#[tauri::command]
pub async fn prepare_presence(
    state: State<'_, Shared>,
    name: String,
) -> Result<PreparedView, String> {
    let copy = blocking(&state, move |app| app.prepare_presence(&name)).await??;
    // The path comes from the controller, never from the interface. Failing to
    // show it takes nothing away: the copy is written.
    let _ = std::process::Command::new("explorer")
        .arg("/select,")
        .arg(&copy.path)
        .spawn();
    Ok(copy)
}

/// The address of a legal page; anything else is refused.
fn legal_address(page: &str, language: &str) -> Option<String> {
    const SITE: &str = "https://manucabral.github.io/RichPresencePlus";
    let page = ["privacy", "terms", "licenses"]
        .into_iter()
        .find(|known| *known == page)?;
    let prefix = if language.starts_with("es") {
        "/es"
    } else {
        ""
    };
    Some(format!("{SITE}{prefix}/{page}/"))
}

/// The address behind a link named in a text; anything else is refused.
fn link_address(target: &str) -> Option<&'static str> {
    match target {
        "developers" => Some("https://discord.com/developers/applications"),
        "docs" => Some("https://manucabral.github.io/RichPresencePlus/docs/"),
        "docs-es" => Some("https://manucabral.github.io/RichPresencePlus/es/docs/"),
        "publish" => Some("https://manucabral.github.io/RichPresencePlus/docs/publish/"),
        "publish-es" => Some("https://manucabral.github.io/RichPresencePlus/es/docs/publish/"),
        "issues" => Some("https://github.com/manucabral/RichPresencePlus/issues/new"),
        "upload" => Some("https://github.com/manucabral/RichPresencePlus/upload/main/presences"),
        _ => None,
    }
}

/// Opens an address in the browser. Callers build it from fixed parts: the
/// interface never hands one over.
fn open_in_browser(address: &str) -> Result<(), String> {
    std::process::Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", address])
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Opens a legal page in the browser.
#[tauri::command]
pub async fn open_legal(page: String, language: String) -> Result<(), String> {
    open_in_browser(&legal_address(&page, &language).ok_or("Unknown page")?)
}

/// Opens a link named in a text of the interface.
#[tauri::command]
pub async fn open_link(target: String) -> Result<(), String> {
    open_in_browser(link_address(&target).ok_or("Unknown link")?)
}

/// What is being watched and how the last reload went.
#[tauri::command]
pub async fn dev_status(state: State<'_, Shared>) -> Result<DevStatus, String> {
    blocking(&state, |app| app.dev_status()).await
}

/// Opens the folder of a presence in the file manager.
#[tauri::command]
pub async fn open_presence_folder(state: State<'_, Shared>, name: String) -> Result<(), String> {
    let folder = blocking(&state, move |app| app.presence_folder(&name)).await??;
    // The folder comes from the loaded presences, never from the interface.
    std::process::Command::new("explorer")
        .arg(folder)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// The installed Visual Studio Code, if there is one.
fn code_editor() -> Option<std::path::PathBuf> {
    let user = std::env::var_os("LOCALAPPDATA").map(|folder| Path::new(&folder).join("Programs"));
    let everyone = std::env::var_os("ProgramFiles").map(std::path::PathBuf::from);
    rpp_app::visual_studio_code(user.as_deref(), everyone.as_deref())
}

/// Whether a presence can be opened in Visual Studio Code.
#[tauri::command]
pub async fn has_code_editor() -> bool {
    code_editor().is_some()
}

/// Opens the folder of a presence in Visual Studio Code.
#[tauri::command]
pub async fn open_in_code_editor(state: State<'_, Shared>, name: String) -> Result<(), String> {
    let folder = blocking(&state, move |app| app.presence_folder(&name)).await??;
    let program = code_editor().ok_or("Visual Studio Code is not installed.")?;
    // The folder comes from the loaded presences and is an absolute path, so the
    // editor cannot read it as one of its options.
    std::process::Command::new(program)
        .arg(folder)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// The logos kept as files in the presence folders, as `data:` images by name.
#[tauri::command]
pub async fn presence_icons(
    state: State<'_, Shared>,
) -> Result<std::collections::HashMap<String, String>, String> {
    blocking(&state, |app| app.icons()).await
}

/// Changes the descriptive fields of a presence in the user's folder.
#[tauri::command]
pub async fn update_manifest(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    name: String,
    changes: ManifestChanges,
) -> Result<(), String> {
    changing(&app, &state, move |app| {
        app.update_manifest(&name, &changes)
    })
    .await?
}

#[cfg(test)]
mod tests {
    use super::{legal_address, link_address};

    #[test]
    fn only_known_legal_pages_have_an_address() {
        assert_eq!(
            legal_address("terms", "es-AR").as_deref(),
            Some("https://manucabral.github.io/RichPresencePlus/es/terms/")
        );
        assert_eq!(
            legal_address("privacy", "en").as_deref(),
            Some("https://manucabral.github.io/RichPresencePlus/privacy/")
        );
        assert_eq!(legal_address("https://evil.example", "en"), None);
        assert_eq!(legal_address("../x", "es"), None);
    }

    #[test]
    fn only_known_links_have_an_address() {
        for target in ["developers", "docs", "docs-es", "issues"] {
            assert!(link_address(target).unwrap().starts_with("https://"));
        }
        assert_eq!(link_address("https://evil.example"), None);
        assert_eq!(link_address(""), None);
    }
}
