//! Self-update via GitHub releases (`plugins.updater` in tauri.conf.json).

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

use crate::{ducker, state::AppState};

/// Checks once at startup and emits `update-available` with the new version;
/// the UI then offers a one-click install.
pub fn spawn_check(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        if let Ok(updater) = app.updater() {
            if let Ok(Some(update)) = updater.check().await {
                let _ = app.emit("update-available", update.version.clone());
            }
        }
    });
}

/// Downloads and installs the pending update, then restarts.
pub async fn install(app: AppHandle) -> Result<(), String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no update available".to_string())?;
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| e.to_string())?;
    ducker::restore_now(&app.state::<AppState>());
    app.restart();
}
