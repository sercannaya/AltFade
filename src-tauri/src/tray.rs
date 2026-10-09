use std::sync::OnceLock;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Wry,
};

use crate::{settings::Settings, state, state::AppState, window};

const TRAY_ID: &str = "main";

fn base_icon() -> &'static image::RgbaImage {
    static ICON: OnceLock<image::RgbaImage> = OnceLock::new();
    ICON.get_or_init(|| {
        image::load_from_memory(include_bytes!("../icons/32x32.png"))
            .expect("tray icon decode")
            .to_rgba8()
    })
}

/// App icon with a status dot in the bottom-right corner while the service is
/// on: green = armed, orange = ducking.
fn icon_image(active: bool, ducked: bool) -> Image<'static> {
    let mut img = base_icon().clone();
    if active {
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
    Image::new_owned(img.into_raw(), w, h)
}

fn tooltip(lang: &str, active: bool, ducked: bool) -> &'static str {
    match (lang, active, ducked) {
        ("tr", false, _) => "AltFade — kapalı",
        ("tr", true, false) => "AltFade — aktif, medya bekleniyor",
        ("tr", true, true) => "AltFade — ses kısıldı",
        (_, false, _) => "AltFade — off",
        (_, true, false) => "AltFade — armed, waiting for media",
        (_, true, true) => "AltFade — ducking",
    }
}

fn build_menu(app: &AppHandle, lang: &str, active: bool) -> tauri::Result<Menu<Wry>> {
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
/// the main thread so it is safe to call from the duck loop or hotkeys.
pub fn update(app: &AppHandle) {
    let app = app.clone();
    let _ = app.clone().run_on_main_thread(move || {
        let state = app.state::<AppState>();
        let (active, lang) = {
            let settings = state.settings.lock().unwrap();
            (settings.active, settings.lang.clone())
        };
        let ducked = state.ducked.lock().unwrap().is_some();
        let Some(tray) = app.tray_by_id(TRAY_ID) else {
            return;
        };
        if let Ok(menu) = build_menu(&app, &lang, active) {
            let _ = tray.set_menu(Some(menu));
        }
        let _ = tray.set_icon(Some(icon_image(active, ducked)));
        let _ = tray.set_tooltip(Some(tooltip(&lang, active, ducked)));
    });
}

pub fn build(app: &tauri::App, settings: &Settings) -> tauri::Result<()> {
    let menu = build_menu(app.handle(), &settings.lang, settings.active)?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon_image(settings.active, false))
        .tooltip(tooltip(&settings.lang, settings.active, false))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                window::show_main(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => window::show_main(app),
            "toggle" => {
                state::set_active(app, None);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}
