//! AltFade: ducks game audio while other media plays. See `docs/architecture.md`.

#[cfg(not(windows))]
compile_error!("AltFade is Windows-only: it is built on WASAPI and SMTC.");

mod audio;
mod commands;
mod ducker;
mod hotkey;
mod settings;
mod state;
mod tray;
mod updater;
mod window;

use std::sync::atomic::Ordering;
use tauri::{Manager, RunEvent, WindowEvent};

use state::AppState;

/// Passed by the autostart entry; the app then starts in the tray.
const MINIMIZED_ARG: &str = "--minimized";

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            window::show_main(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![MINIMIZED_ARG]),
        ))
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::get_status,
            commands::get_audio_sessions,
            commands::get_session_peaks,
            commands::get_media_sources,
            commands::set_targets,
            commands::set_trigger_apps,
            commands::set_ignored_sources,
            commands::toggle_ducking,
            commands::set_duck_volume,
            commands::set_unduck_delay,
            commands::set_language,
            commands::set_hotkey_enabled,
            commands::is_mica_active,
            commands::set_window_theme,
            commands::install_update,
            commands::get_autostart_enabled,
            commands::set_autostart_enabled,
        ])
        .setup(|app| {
            let handle = app.handle();
            let settings = settings::load(handle);
            *app.state::<AppState>().settings.lock().unwrap() = settings.clone();

            tray::build(app, &settings)?;

            handle.plugin(tauri_plugin_global_shortcut::Builder::new().build())?;
            if settings.hotkey_enabled {
                hotkey::register(handle, true);
            }

            handle.plugin(tauri_plugin_updater::Builder::new().build())?;
            updater::spawn_check(handle.clone());

            if window::apply_mica(handle) {
                app.state::<AppState>()
                    .mica_active
                    .store(true, Ordering::Relaxed);
            }

            // The window is created hidden (visible: false in tauri.conf.json).
            // Under autostart it stays in the tray, and its webview must be
            // marked hidden too or the page would render and poll unseen.
            if std::env::args().any(|a| a == MINIMIZED_ARG) {
                window::hide_main(handle);
            } else {
                window::show_main(handle);
            }

            ducker::spawn(handle.clone());
            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                window::hide_main(window.app_handle());
            }
            WindowEvent::Resized(_) => window::sync_webview_visibility(window.app_handle()),
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                ducker::restore_now(&app.state::<AppState>());
            }
        });
}
