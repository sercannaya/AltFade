//! Main window visibility.
//!
//! WebView2 only learns whether its page is visible through the controller,
//! not from the host window. Hiding the window alone (close-to-tray) leaves
//! the page "visible": it keeps rendering at the display refresh rate and its
//! timers keep polling the backend. Every hide/show therefore goes through
//! here so the webview's visibility always follows the window's. With the
//! webview hidden, `document.hidden` is true and the UI stops polling.

use tauri::{AppHandle, Manager, WebviewWindow};

pub const MAIN: &str = "main";

fn main_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(MAIN)
}

/// Shows, restores and focuses the main window.
pub fn show_main(app: &AppHandle) {
    if let Some(win) = main_window(app) {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.as_ref().show();
        let _ = win.set_focus();
    }
}

/// Hides the main window to the tray.
pub fn hide_main(app: &AppHandle) {
    if let Some(win) = main_window(app) {
        let _ = win.as_ref().hide();
        let _ = win.hide();
    }
}

/// Re-derives webview visibility from the window after a size change, which is
/// how minimize and restore surface as window events.
pub fn sync_webview_visibility(app: &AppHandle) {
    let Some(win) = main_window(app) else { return };
    let visible = win.is_visible().unwrap_or(true) && !win.is_minimized().unwrap_or(false);
    let _ = if visible {
        win.as_ref().show()
    } else {
        win.as_ref().hide()
    };
}

/// Mica backdrop needs Windows 11 (build 22000+); older systems keep the solid
/// CSS background, so failure here is invisible. Returns whether it applied.
pub fn apply_mica(app: &AppHandle) -> bool {
    if windows_version::OsVersion::current().build < 22000 {
        return false;
    }
    let Some(win) = main_window(app) else {
        return false;
    };
    use tauri::window::{Effect, EffectsBuilder};
    win.set_effects(EffectsBuilder::new().effect(Effect::Mica).build())
        .is_ok()
}
