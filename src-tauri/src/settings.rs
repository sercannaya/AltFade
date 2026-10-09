use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use crate::state::AppState;

/// Persisted user configuration (`%APPDATA%\<identifier>\config.json`).
/// Field names are the JSON keys the frontend reads, so renaming one is a
/// breaking change for existing configs.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Exe names ("game.exe") whose volume gets ducked.
    pub targets: Vec<String>,
    pub duck_percent: u32,
    pub unduck_delay_ms: u64,
    /// Substrings of media AppUserModelIds allowed to trigger ducking;
    /// empty means any media source triggers.
    pub trigger_apps: Vec<String>,
    /// Media sources that must never trigger ducking, even when the trigger
    /// list is empty. Lets a target that also registers a media session
    /// (a game with media-key support) avoid ducking itself.
    pub ignored_sources: Vec<String>,
    pub hotkey_enabled: bool,
    /// Whether the ducking service is running.
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
            ignored_sources: vec![],
            hotkey_enabled: true,
            active: false,
            lang: "tr".into(),
            target: None,
        }
    }
}

impl Settings {
    /// Duck level as a 0..1 volume, kept inside a sane range.
    pub fn duck_volume(&self) -> f32 {
        (self.duck_percent as f32 / 100.0).clamp(0.05, 0.9)
    }
}

fn config_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|d| d.join("config.json"))
}

/// Reads the config file, falling back to defaults when it is missing or
/// unreadable.
pub fn load(app: &AppHandle) -> Settings {
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

/// Writes the current in-memory settings to disk. Failures are ignored: the
/// app keeps working with the in-memory copy.
pub fn save(app: &AppHandle) {
    let settings = app.state::<AppState>().settings.lock().unwrap().clone();
    let Some(path) = config_path(app) else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string_pretty(&settings) {
        let _ = std::fs::write(path, json);
    }
}

/// Applies `change` to the in-memory settings and persists them.
pub fn update(app: &AppHandle, change: impl FnOnce(&mut Settings)) {
    change(&mut app.state::<AppState>().settings.lock().unwrap());
    save(app);
}
