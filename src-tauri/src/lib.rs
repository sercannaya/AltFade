#[cfg(target_os = "windows")]
mod audio;

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, WindowEvent,
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub target: Option<String>,
    pub duck_percent: u32,
    pub active: bool,
    pub lang: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            target: None,
            duck_percent: 20,
            active: false,
            lang: "tr".into(),
        }
    }
}

impl Settings {
    fn duck_volume(&self) -> f32 {
        (self.duck_percent as f32 / 100.0).clamp(0.05, 0.9)
    }
}

/// Snapshot taken when ducking starts, so the exact process and its
/// pre-duck volume can be restored even if the target selection changes.
#[derive(Clone)]
pub struct DuckedState {
    pub target: String,
    pub original_volume: f32,
    pub applied_volume: f32,
}

#[derive(Default)]
pub struct AppState {
    pub settings: Mutex<Settings>,
    pub ducked: Mutex<Option<DuckedState>>,
}

fn config_path(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("config.json"))
}

fn load_settings(app: &AppHandle) -> Settings {
    config_path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_settings(app: &AppHandle) {
    let settings = app.state::<AppState>().settings.lock().unwrap().clone();
    let Some(path) = config_path(app) else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string_pretty(&settings) {
        let _ = std::fs::write(path, json);
    }
}

#[cfg(target_os = "windows")]
fn restore_ducked(state: &AppState) {
    let taken = state.ducked.lock().unwrap().take();
    if let Some(ducked) = taken {
        audio::set_process_volume(&ducked.target, ducked.original_volume);
    }
}

#[tauri::command]
fn get_settings(state: tauri::State<AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn get_audio_sessions() -> Vec<audio::AudioSession> {
    #[cfg(target_os = "windows")]
    return audio::list_sessions();
    #[cfg(not(target_os = "windows"))]
    vec![]
}

#[tauri::command]
fn set_duck_volume(app: AppHandle, state: tauri::State<AppState>, percent: u32) {
    state.settings.lock().unwrap().duck_percent = percent.clamp(5, 90);
    save_settings(&app);
}

#[tauri::command]
fn set_target_process(app: AppHandle, state: tauri::State<AppState>, name: String) {
    state.settings.lock().unwrap().target = Some(name);
    save_settings(&app);
}

#[tauri::command]
fn toggle_ducking(app: AppHandle, state: tauri::State<AppState>) -> bool {
    let active = {
        let mut settings = state.settings.lock().unwrap();
        settings.active = !settings.active;
        settings.active
    };
    save_settings(&app);
    active
}

#[tauri::command]
fn set_language(app: AppHandle, state: tauri::State<AppState>, lang: String) {
    state.settings.lock().unwrap().lang = lang.clone();
    if let Some(tray) = app.tray_by_id("main") {
        if let Ok(menu) = build_tray_menu(&app, &lang) {
            let _ = tray.set_menu(Some(menu));
        }
    }
    save_settings(&app);
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
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(500));

        let state = handle.state::<AppState>();
        let (active, target, duck_vol) = {
            let settings = state.settings.lock().unwrap();
            (settings.active, settings.target.clone(), settings.duck_volume())
        };
        let ducked = state.ducked.lock().unwrap().clone();

        // Target changed or cleared while ducked: restore the old process first.
        if let Some(d) = &ducked {
            if target.as_deref() != Some(d.target.as_str()) {
                audio::fade_process_volume(&d.target, d.applied_volume, d.original_volume, 300, 10);
                *state.ducked.lock().unwrap() = None;
                continue;
            }
        }

        let Some(target_name) = target else {
            continue;
        };

        if active {
            let playing = audio::is_media_playing();
            match (&ducked, playing) {
                (None, true) => {
                    let original = audio::get_process_volume(&target_name).unwrap_or(1.0);
                    let duck_to = duck_vol.min(original);
                    audio::fade_process_volume(&target_name, original, duck_to, 400, 16);
                    *state.ducked.lock().unwrap() = Some(DuckedState {
                        target: target_name,
                        original_volume: original,
                        applied_volume: duck_to,
                    });
                }
                (Some(d), false) => {
                    audio::fade_process_volume(
                        &target_name,
                        d.applied_volume,
                        d.original_volume,
                        1500,
                        30,
                    );
                    *state.ducked.lock().unwrap() = None;
                }
                (Some(d), true) => {
                    // Duck level changed from the slider while ducked: apply it live.
                    let duck_to = duck_vol.min(d.original_volume);
                    if (duck_to - d.applied_volume).abs() > 0.001 {
                        audio::set_process_volume(&target_name, duck_to);
                        if let Some(d) = state.ducked.lock().unwrap().as_mut() {
                            d.applied_volume = duck_to;
                        }
                    }
                }
                (None, false) => {}
            }
        } else if let Some(d) = &ducked {
            audio::fade_process_volume(&target_name, d.applied_volume, d.original_volume, 1000, 20);
            *state.ducked.lock().unwrap() = None;
        }
    });
}

fn show_main_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }
}

fn tray_labels(lang: &str) -> (&'static str, &'static str) {
    match lang {
        "tr" => ("Göster", "Çıkış"),
        _ => ("Show", "Quit"),
    }
}

fn build_tray_menu(app: &AppHandle, lang: &str) -> tauri::Result<Menu<tauri::Wry>> {
    let (show_label, quit_label) = tray_labels(lang);
    let show = MenuItem::with_id(app, "show", show_label, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", quit_label, true, None::<&str>)?;
    Menu::with_items(app, &[&show, &quit])
}

fn build_tray(app: &tauri::App, lang: &str) -> tauri::Result<()> {
    let menu = build_tray_menu(app.handle(), lang)?;

    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))
        .expect("tray icon not found");

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("AltFade")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            get_audio_sessions,
            set_target_process,
            toggle_ducking,
            set_duck_volume,
            set_language,
            get_autostart_enabled,
            set_autostart_enabled,
        ])
        .setup(|app| {
            let settings = load_settings(app.handle());
            let lang = settings.lang.clone();
            *app.state::<AppState>().settings.lock().unwrap() = settings;

            build_tray(app, &lang)?;

            // The window is created hidden (visible: false in tauri.conf.json);
            // only show it when not launched by autostart.
            if !std::env::args().any(|a| a == "--minimized") {
                show_main_window(app.handle());
            }

            #[cfg(target_os = "windows")]
            start_duck_thread(app);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, _event| {
            #[cfg(target_os = "windows")]
            if let tauri::RunEvent::Exit = _event {
                restore_ducked(&_app_handle.state::<AppState>());
            }
        });
}
