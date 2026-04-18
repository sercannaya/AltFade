#[cfg(target_os = "windows")]
mod audio;

use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager, WindowEvent,
};

pub struct AppState {
    pub target_process: Mutex<Option<String>>,
    pub is_active: Mutex<bool>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            target_process: Mutex::new(None),
            is_active: Mutex::new(false),
        }
    }
}

#[tauri::command]
fn get_audio_sessions() -> Vec<String> {
    #[cfg(target_os = "windows")]
    return audio::list_sessions();
    #[cfg(not(target_os = "windows"))]
    vec![]
}

#[tauri::command]
fn set_target_process(state: tauri::State<AppState>, name: String) {
    *state.target_process.lock().unwrap() = Some(name);
}

#[tauri::command]
fn toggle_ducking(state: tauri::State<AppState>) -> bool {
    let mut active = state.is_active.lock().unwrap();
    *active = !*active;
    *active
}

#[tauri::command]
fn get_autostart_enabled(app: AppHandle) -> bool {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
fn set_autostart_enabled(app: AppHandle, enabled: bool) {
    use tauri_plugin_autostart::ManagerExt;
    let autolaunch = app.autolaunch();
    if enabled {
        let _ = autolaunch.enable();
    } else {
        let _ = autolaunch.disable();
    }
}

#[cfg(target_os = "windows")]
fn start_duck_thread(app: &tauri::App) {
    let handle = app.handle().clone();
    std::thread::spawn(move || {
        let mut was_ducked = false;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));

            let state = handle.state::<AppState>();
            let is_active = *state.is_active.lock().unwrap();
            let target = state.target_process.lock().unwrap().clone();

            let Some(target_name) = target else {
                continue;
            };

            if is_active {
                let playing = audio::is_media_playing();
                if playing && !was_ducked {
                    audio::fade_process_volume(&target_name, 1.0, 0.2, 400, 16);
                    was_ducked = true;
                } else if !playing && was_ducked {
                    audio::fade_process_volume(&target_name, 0.2, 1.0, 1500, 30);
                    was_ducked = false;
                }
            } else if was_ducked {
                audio::fade_process_volume(&target_name, 0.2, 1.0, 1000, 20);
                was_ducked = false;
            }
        }
    });
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Göster", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Çıkış", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("AltFade")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_audio_sessions,
            set_target_process,
            toggle_ducking,
            get_autostart_enabled,
            set_autostart_enabled,
        ])
        .setup(|app| {
            build_tray(app)?;
            #[cfg(target_os = "windows")]
            start_duck_thread(app);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                window.hide().unwrap();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
