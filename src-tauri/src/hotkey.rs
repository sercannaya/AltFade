use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::state;

/// Toggles the ducking service from anywhere, including inside a game.
/// The UI shows it as "Ctrl+Alt+D"; keep both in sync.
const HOTKEY: &str = "ctrl+alt+d";

/// (Re)registers or removes the global hotkey.
pub fn register(app: &AppHandle, enabled: bool) {
    let shortcuts = app.global_shortcut();
    let _ = shortcuts.unregister(HOTKEY);
    if enabled {
        let _ = shortcuts.on_shortcut(HOTKEY, |app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                state::set_active(app, None);
            }
        });
    }
}
