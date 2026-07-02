#[cfg(target_os = "windows")]
mod audio;

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, WindowEvent,
};

#[cfg(desktop)]
const HOTKEY: &str = "ctrl+alt+d";

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub targets: Vec<String>,
    pub duck_percent: u32,
    pub unduck_delay_ms: u64,
    pub trigger_apps: Vec<String>,
    pub hotkey_enabled: bool,
    pub active: bool,
    pub lang: String,
    /// Pre-1.1 configs stored a single target; folded into `targets` on load.
    #[serde(skip_serializing)]
    target: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            targets: vec![],
            duck_percent: 20,
            unduck_delay_ms: 1500,
            trigger_apps: vec![],
            hotkey_enabled: true,
            active: false,
            lang: "tr".into(),
            target: None,
        }
    }
}

impl Settings {
    fn duck_volume(&self) -> f32 {
        (self.duck_percent as f32 / 100.0).clamp(0.05, 0.9)
    }
}

/// Snapshot taken when ducking starts, so the exact processes and their
/// pre-duck volumes can be restored even if the target selection changes.
#[derive(Clone)]
pub struct DuckedProcess {
    pub name: String,
    pub original_volume: f32,
    pub applied_volume: f32,
}

#[derive(Clone)]
pub struct DuckedState {
    pub processes: Vec<DuckedProcess>,
}

#[derive(Default)]
pub struct AppState {
    pub settings: Mutex<Settings>,
    pub ducked: Mutex<Option<DuckedState>>,
    pub mica_active: std::sync::atomic::AtomicBool,
}

fn config_path(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("config.json"))
}

fn load_settings(app: &AppHandle) -> Settings {
    let mut settings: Settings = config_path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    if settings.targets.is_empty() {
        if let Some(legacy) = settings.target.take() {
            settings.targets.push(legacy);
        }
    }
    settings
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
        for process in &ducked.processes {
            audio::set_process_volume(&process.name, process.original_volume);
        }
    }
}

/// Central on/off switch used by the UI command, the tray menu and the
/// global hotkey. `value: None` toggles.
fn set_active(app: &AppHandle, value: Option<bool>) -> bool {
    let state = app.state::<AppState>();
    let active = {
        let mut settings = state.settings.lock().unwrap();
        settings.active = value.unwrap_or(!settings.active);
        settings.active
    };
    save_settings(app);
    update_tray(app);
    let _ = app.emit("active-changed", active);
    active
}

#[derive(Serialize)]
struct Status {
    ducked: bool,
    now_playing: Option<audio::NowPlaying>,
}

#[tauri::command]
fn get_settings(state: tauri::State<AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

// The commands below do blocking COM/WinRT work (session enumeration, media
// info) and are polled by the UI. They MUST be async: sync Tauri commands run
// on the main thread and would freeze the window.
#[tauri::command]
async fn get_status(app: AppHandle) -> Status {
    Status {
        ducked: app.state::<AppState>().ducked.lock().unwrap().is_some(),
        #[cfg(target_os = "windows")]
        now_playing: audio::get_now_playing(),
        #[cfg(not(target_os = "windows"))]
        now_playing: None,
    }
}

#[tauri::command]
async fn get_audio_sessions() -> Vec<audio::AudioSession> {
    #[cfg(target_os = "windows")]
    return audio::list_sessions();
    #[cfg(not(target_os = "windows"))]
    vec![]
}

#[tauri::command]
async fn get_session_peaks() -> Vec<audio::SessionPeak> {
    #[cfg(target_os = "windows")]
    return audio::get_session_peaks();
    #[cfg(not(target_os = "windows"))]
    vec![]
}

#[tauri::command]
async fn get_media_sources() -> Vec<audio::MediaSource> {
    #[cfg(target_os = "windows")]
    return audio::list_media_sources();
    #[cfg(not(target_os = "windows"))]
    vec![]
}

#[tauri::command]
fn set_targets(app: AppHandle, state: tauri::State<AppState>, names: Vec<String>) {
    state.settings.lock().unwrap().targets = names;
    save_settings(&app);
}

#[tauri::command]
fn set_trigger_apps(app: AppHandle, state: tauri::State<AppState>, apps: Vec<String>) {
    state.settings.lock().unwrap().trigger_apps = apps;
    save_settings(&app);
}

#[tauri::command]
fn set_duck_volume(app: AppHandle, state: tauri::State<AppState>, percent: u32) {
    state.settings.lock().unwrap().duck_percent = percent.clamp(5, 90);
    save_settings(&app);
}

#[tauri::command]
fn set_unduck_delay(app: AppHandle, state: tauri::State<AppState>, ms: u64) {
    state.settings.lock().unwrap().unduck_delay_ms = ms.min(10_000);
    save_settings(&app);
}

#[tauri::command]
fn toggle_ducking(app: AppHandle) -> bool {
    set_active(&app, None)
}

#[tauri::command]
fn set_language(app: AppHandle, state: tauri::State<AppState>, lang: String) {
    state.settings.lock().unwrap().lang = lang;
    update_tray(&app);
    save_settings(&app);
}

#[tauri::command]
fn set_hotkey_enabled(app: AppHandle, state: tauri::State<AppState>, enabled: bool) {
    state.settings.lock().unwrap().hotkey_enabled = enabled;
    #[cfg(desktop)]
    register_hotkey(&app, enabled);
    save_settings(&app);
}

#[tauri::command]
fn is_mica_active(state: tauri::State<AppState>) -> bool {
    state.mica_active.load(std::sync::atomic::Ordering::Relaxed)
}

#[tauri::command]
fn set_window_theme(app: AppHandle, dark: bool) {
    if let Some(win) = app.get_webview_window("main") {
        let theme = if dark {
            tauri::Theme::Dark
        } else {
            tauri::Theme::Light
        };
        let _ = win.set_theme(Some(theme));
    }
}

#[cfg(desktop)]
#[tauri::command]
async fn install_update(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;
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
    #[cfg(target_os = "windows")]
    restore_ducked(&app.state::<AppState>());
    app.restart();
}

#[cfg(not(desktop))]
#[tauri::command]
async fn install_update(_app: AppHandle) -> Result<(), String> {
    Err("updates unsupported on this platform".into())
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

#[cfg(desktop)]
fn register_hotkey(app: &AppHandle, enabled: bool) {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
    let shortcuts = app.global_shortcut();
    let _ = shortcuts.unregister(HOTKEY);
    if enabled {
        let _ = shortcuts.on_shortcut(HOTKEY, |app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                set_active(app, None);
            }
        });
    }
}

#[cfg(target_os = "windows")]
fn restore_with_fade(ducked: &DuckedState, duration_ms: u64, steps: u32) {
    let fades: Vec<audio::FadeTarget> = ducked
        .processes
        .iter()
        .map(|p| audio::FadeTarget {
            name: p.name.clone(),
            from: p.applied_volume,
            to: p.original_volume,
        })
        .collect();
    audio::fade_volumes(&fades, duration_ms, steps);
}

#[cfg(target_os = "windows")]
fn start_duck_thread(app: &tauri::App) {
    use std::time::{Duration, Instant};

    let handle = app.handle().clone();
    std::thread::spawn(move || {
        let mut last_playing_at: Option<Instant> = None;
        loop {
            std::thread::sleep(Duration::from_millis(500));

            let state = handle.state::<AppState>();
            let (active, targets, duck_vol, unduck_delay, triggers) = {
                let settings = state.settings.lock().unwrap();
                (
                    settings.active,
                    settings.targets.clone(),
                    settings.duck_volume(),
                    settings.unduck_delay_ms,
                    settings.trigger_apps.clone(),
                )
            };
            let ducked = state.ducked.lock().unwrap().clone();

            if !active {
                if let Some(d) = ducked {
                    restore_with_fade(&d, 1000, 20);
                    *state.ducked.lock().unwrap() = None;
                    update_tray(&handle);
                }
                last_playing_at = None;
                continue;
            }

            let playing = audio::is_media_playing(&triggers, &targets);
            if playing {
                last_playing_at = Some(Instant::now());
            }

            match ducked {
                None => {
                    if playing && !targets.is_empty() {
                        let processes: Vec<DuckedProcess> = targets
                            .iter()
                            .map(|name| {
                                let original =
                                    audio::get_process_volume(name).unwrap_or(1.0);
                                DuckedProcess {
                                    name: name.clone(),
                                    original_volume: original,
                                    // Never raise a target that is already quieter.
                                    applied_volume: duck_vol.min(original),
                                }
                            })
                            .collect();
                        let fades: Vec<audio::FadeTarget> = processes
                            .iter()
                            .map(|p| audio::FadeTarget {
                                name: p.name.clone(),
                                from: p.original_volume,
                                to: p.applied_volume,
                            })
                            .collect();
                        audio::fade_volumes(&fades, 400, 16);
                        *state.ducked.lock().unwrap() = Some(DuckedState { processes });
                        update_tray(&handle);
                    }
                }
                Some(d) => {
                    let ducked_names: Vec<&String> = d.processes.iter().map(|p| &p.name).collect();
                    let target_names: Vec<&String> = targets.iter().collect();
                    if ducked_names != target_names {
                        // Target list changed: restore the old set; the next
                        // tick re-ducks the new one if media is still playing.
                        restore_with_fade(&d, 300, 10);
                        *state.ducked.lock().unwrap() = None;
                        update_tray(&handle);
                        continue;
                    }

                    if playing {
                        // Duck level changed from the slider while ducked: apply it live.
                        let mut updated = d.clone();
                        let mut changed = false;
                        for process in updated.processes.iter_mut() {
                            let want = duck_vol.min(process.original_volume);
                            if (want - process.applied_volume).abs() > 0.001 {
                                audio::set_process_volume(&process.name, want);
                                process.applied_volume = want;
                                changed = true;
                            }
                        }
                        if changed {
                            *state.ducked.lock().unwrap() = Some(updated);
                        }
                    } else {
                        // Debounce: hold the duck through short gaps (track
                        // changes, seeking) and only restore after the delay.
                        let waited_long_enough = last_playing_at
                            .map(|at| at.elapsed() >= Duration::from_millis(unduck_delay))
                            .unwrap_or(true);
                        if waited_long_enough {
                            restore_with_fade(&d, 1500, 30);
                            *state.ducked.lock().unwrap() = None;
                            update_tray(&handle);
                        }
                    }
                }
            }
        }
    });
}

fn show_main_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }
}

fn tray_icon_image(active: bool, ducked: bool) -> tauri::image::Image<'static> {
    #[cfg(target_os = "windows")]
    {
        let mut img = image::load_from_memory(include_bytes!("../icons/32x32.png"))
            .expect("tray icon decode")
            .to_rgba8();
        if active {
            // Status dot bottom-right: green = armed, orange = ducking.
            let color = if ducked {
                image::Rgba([255u8, 165, 66, 255])
            } else {
                image::Rgba([74u8, 222, 128, 255])
            };
            let (w, h) = img.dimensions();
            let (cx, cy, radius) = (w as f32 - 6.5, h as f32 - 6.5, 5.5f32);
            for y in 0..h {
                for x in 0..w {
                    let dist = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
                    if dist <= radius {
                        img.put_pixel(x, y, color);
                    } else if dist <= radius + 1.2 {
                        img.put_pixel(x, y, image::Rgba([13, 13, 20, 255]));
                    }
                }
            }
        }
        let (w, h) = img.dimensions();
        tauri::image::Image::new_owned(img.into_raw(), w, h)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (active, ducked);
        tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))
            .expect("tray icon not found")
    }
}

fn tray_tooltip(lang: &str, active: bool, ducked: bool) -> &'static str {
    match (lang, active, ducked) {
        ("tr", false, _) => "AltFade — kapalı",
        ("tr", true, false) => "AltFade — aktif, medya bekleniyor",
        ("tr", true, true) => "AltFade — ses kısıldı",
        (_, false, _) => "AltFade — off",
        (_, true, false) => "AltFade — armed, waiting for media",
        (_, true, true) => "AltFade — ducking",
    }
}

fn build_tray_menu(app: &AppHandle, lang: &str, active: bool) -> tauri::Result<Menu<tauri::Wry>> {
    let (show_label, start_label, stop_label, quit_label) = match lang {
        "tr" => ("Göster", "Servisi Başlat", "Servisi Durdur", "Çıkış"),
        _ => ("Show", "Start Service", "Stop Service", "Quit"),
    };
    let show = MenuItem::with_id(app, "show", show_label, true, None::<&str>)?;
    let toggle = MenuItem::with_id(
        app,
        "toggle",
        if active { stop_label } else { start_label },
        true,
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", quit_label, true, None::<&str>)?;
    Menu::with_items(app, &[&show, &toggle, &separator, &quit])
}

/// Rebuilds the tray menu, icon and tooltip from current state. Dispatched to
/// the main thread so it is safe to call from the duck thread or hotkeys.
fn update_tray(app: &AppHandle) {
    let app = app.clone();
    let _ = app.clone().run_on_main_thread(move || {
        let state = app.state::<AppState>();
        let (active, lang) = {
            let settings = state.settings.lock().unwrap();
            (settings.active, settings.lang.clone())
        };
        let ducked = state.ducked.lock().unwrap().is_some();
        let Some(tray) = app.tray_by_id("main") else {
            return;
        };
        if let Ok(menu) = build_tray_menu(&app, &lang, active) {
            let _ = tray.set_menu(Some(menu));
        }
        let _ = tray.set_icon(Some(tray_icon_image(active, ducked)));
        let _ = tray.set_tooltip(Some(tray_tooltip(&lang, active, ducked)));
    });
}

fn build_tray(app: &tauri::App, settings: &Settings) -> tauri::Result<()> {
    let menu = build_tray_menu(app.handle(), &settings.lang, settings.active)?;

    TrayIconBuilder::with_id("main")
        .icon(tray_icon_image(settings.active, false))
        .tooltip(tray_tooltip(&settings.lang, settings.active, false))
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
            "toggle" => {
                set_active(app, None);
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
            get_status,
            get_audio_sessions,
            get_session_peaks,
            get_media_sources,
            set_targets,
            set_trigger_apps,
            toggle_ducking,
            set_duck_volume,
            set_unduck_delay,
            set_language,
            set_hotkey_enabled,
            is_mica_active,
            set_window_theme,
            install_update,
            get_autostart_enabled,
            set_autostart_enabled,
        ])
        .setup(|app| {
            let settings = load_settings(app.handle());
            let hotkey_enabled = settings.hotkey_enabled;
            *app.state::<AppState>().settings.lock().unwrap() = settings.clone();

            build_tray(app, &settings)?;

            #[cfg(desktop)]
            {
                app.handle()
                    .plugin(tauri_plugin_global_shortcut::Builder::new().build())?;
                if hotkey_enabled {
                    register_hotkey(app.handle(), true);
                }

                app.handle()
                    .plugin(tauri_plugin_updater::Builder::new().build())?;
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    use tauri_plugin_updater::UpdaterExt;
                    if let Ok(updater) = handle.updater() {
                        if let Ok(Some(update)) = updater.check().await {
                            let _ = handle.emit("update-available", update.version.clone());
                        }
                    }
                });
            }

            // Mica backdrop needs Windows 11 (build 22000+); older systems
            // keep the solid CSS background, so failure here is invisible.
            #[cfg(target_os = "windows")]
            if windows_version::OsVersion::current().build >= 22000 {
                if let Some(win) = app.get_webview_window("main") {
                    use tauri::window::{Effect, EffectsBuilder};
                    let effects = EffectsBuilder::new().effect(Effect::Mica).build();
                    if win.set_effects(effects).is_ok() {
                        app.state::<AppState>()
                            .mica_active
                            .store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            }

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
