//! Tauri commands invoked from the frontend (`src/api.ts`).
//!
//! Commands that touch COM/WinRT are `async` so they run on a tokio worker:
//! sync commands run on the main thread and would freeze the window while the
//! call blocks.

use serde::Serialize;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager};

use crate::{
    audio, hotkey, settings, settings::Settings, state, state::AppState, tray, updater, window,
};

#[derive(Serialize)]
pub struct Status {
    ducked: bool,
    now_playing: Option<audio::NowPlaying>,
}

#[tauri::command]
pub fn get_settings(state: tauri::State<AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
pub async fn get_status(app: AppHandle) -> Status {
    Status {
        ducked: app.state::<AppState>().ducked.lock().unwrap().is_some(),
        now_playing: audio::now_playing(),
    }
}

#[tauri::command]
pub async fn get_audio_sessions() -> Vec<audio::AudioSession> {
    audio::list_sessions()
}

#[tauri::command]
pub async fn get_session_peaks() -> Vec<audio::SessionPeak> {
    audio::session_peaks()
}

#[tauri::command]
pub async fn get_media_sources() -> Vec<audio::MediaSource> {
    audio::list_media_sources()
}

#[tauri::command]
pub fn set_targets(app: AppHandle, names: Vec<String>) {
    settings::update(&app, |s| s.targets = names);
}

#[tauri::command]
pub fn set_trigger_apps(app: AppHandle, apps: Vec<String>) {
    settings::update(&app, |s| s.trigger_apps = apps);
}

#[tauri::command]
pub fn set_ignored_sources(app: AppHandle, sources: Vec<String>) {
    settings::update(&app, |s| s.ignored_sources = sources);
}

#[tauri::command]
pub fn set_duck_volume(app: AppHandle, percent: u32) {
    settings::update(&app, |s| s.duck_percent = percent.clamp(5, 90));
}

#[tauri::command]
pub fn set_unduck_delay(app: AppHandle, ms: u64) {
    settings::update(&app, |s| s.unduck_delay_ms = ms.min(10_000));
}

#[tauri::command]
pub fn toggle_ducking(app: AppHandle) -> bool {
    state::set_active(&app, None)
}

#[tauri::command]
pub fn set_language(app: AppHandle, lang: String) {
    settings::update(&app, |s| s.lang = lang);
    tray::update(&app);
}

#[tauri::command]
pub fn set_hotkey_enabled(app: AppHandle, enabled: bool) {
    settings::update(&app, |s| s.hotkey_enabled = enabled);
    hotkey::register(&app, enabled);
}

#[tauri::command]
pub fn is_mica_active(state: tauri::State<AppState>) -> bool {
    state.mica_active.load(Ordering::Relaxed)
}

#[tauri::command]
pub fn set_window_theme(app: AppHandle, dark: bool) {
    if let Some(win) = app.get_webview_window(window::MAIN) {
        let theme = if dark {
            tauri::Theme::Dark
        } else {
            tauri::Theme::Light
        };
        let _ = win.set_theme(Some(theme));
    }
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    updater::install(app).await
}

#[tauri::command]
pub fn get_autostart_enabled(app: AppHandle) -> bool {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
pub fn set_autostart_enabled(app: AppHandle, enabled: bool) {
    // Never register a dev/debug binary in the Run key: antivirus heuristics
    // flag unsigned exes autostarting from build directories as suspicious.
    #[cfg(debug_assertions)]
    {
        let _ = (app, enabled);
    }
    #[cfg(not(debug_assertions))]
    {
        use tauri_plugin_autostart::ManagerExt;
        let autolaunch = app.autolaunch();
        let _ = if enabled {
            autolaunch.enable()
        } else {
            autolaunch.disable()
        };
    }
}
