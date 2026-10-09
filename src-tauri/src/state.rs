use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

use crate::{ducker::DuckedState, settings, settings::Settings, tray};

#[derive(Default)]
pub struct AppState {
    pub settings: Mutex<Settings>,
    /// `Some` while targets are ducked; owned by the duck loop.
    pub ducked: Mutex<Option<DuckedState>>,
    pub mica_active: AtomicBool,
}

/// Central on/off switch used by the UI command, the tray menu and the
/// global hotkey. `value: None` toggles. The duck loop picks the change up on
/// its next tick.
pub fn set_active(app: &AppHandle, value: Option<bool>) -> bool {
    let mut active = false;
    settings::update(app, |s| {
        s.active = value.unwrap_or(!s.active);
        active = s.active;
    });
    tray::update(app);
    let _ = app.emit("active-changed", active);
    active
}
