//! Updates of the application itself, through the Tauri updater.

use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

/// Public key of the release signing key: the `.pub` file that
/// `cargo tauri signer generate` writes, as it is.
const PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEM1NDExQkVFRjFFMUFDNTEKUldSUnJPSHg3aHRCeFRwdHcreStGL0RwWTh2MVpSWjFmMjhpQ1lLVExhcCs5WmNYSmVtM3kvQjIK";

/// Where the latest release describes itself.
const ENDPOINT: &str =
    "https://github.com/manucabral/RichPresencePlus/releases/latest/download/latest.json";

/// What the interface shows about updates.
#[derive(Debug, Serialize)]
pub struct UpdateView {
    /// False while the build has no release key, so updates are not checked.
    pub configured: bool,
    /// Version that can be installed, when one is newer than the running one.
    pub version: Option<String>,
    /// Release notes of that version.
    pub notes: Option<String>,
}

fn updater(app: &AppHandle) -> Result<Option<tauri_plugin_updater::Updater>, String> {
    if PUBLIC_KEY.is_empty() {
        return Ok(None);
    }
    let endpoint = ENDPOINT.parse::<tauri::Url>().map_err(|e| e.to_string())?;
    app.updater_builder()
        .pubkey(PUBLIC_KEY)
        .endpoints(vec![endpoint])
        .map_err(|e| e.to_string())?
        .build()
        .map(Some)
        .map_err(|e| e.to_string())
}

/// Asks whether a newer version is published.
#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<UpdateView, String> {
    let Some(updater) = updater(&app)? else {
        return Ok(UpdateView {
            configured: false,
            version: None,
            notes: None,
        });
    };
    let found = updater.check().await.map_err(|e| e.to_string())?;
    Ok(UpdateView {
        configured: true,
        version: found.as_ref().map(|update| update.version.clone()),
        notes: found.and_then(|update| update.body),
    })
}

/// Downloads the newer version, verifies its signature and installs it.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    let updater = updater(&app)?.ok_or("Updates are not configured in this build.")?;
    let update = updater
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or("This is the latest version.")?;
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| e.to_string())
}
