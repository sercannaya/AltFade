use std::collections::HashMap;
use std::mem;
use std::sync::{LazyLock, Mutex};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::HANDLE,
        Graphics::Gdi::{
            CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject, BITMAPINFO,
            BITMAPINFOHEADER, DIB_RGB_COLORS, HBRUSH, HGDIOBJ,
        },
        UI::{
            Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON},
            WindowsAndMessaging::{DestroyIcon, DrawIconEx, DI_NORMAL},
        },
    },
};

const SIZE: i32 = 32;

/// Rendered icons keyed by exe path. Extraction plus PNG encoding costs a few
/// milliseconds per exe and the result never changes while the app runs.
static CACHE: LazyLock<Mutex<HashMap<String, Option<String>>>> = LazyLock::new(Default::default);

/// The exe's shell icon as a `data:image/png;base64,…` URL.
pub(super) fn data_url(exe_path: &str) -> Option<String> {
    if let Some(cached) = CACHE.lock().unwrap().get(exe_path) {
        return cached.clone();
    }
    let icon = render(exe_path);
    CACHE
        .lock()
        .unwrap()
        .insert(exe_path.to_owned(), icon.clone());
    icon
}

fn render(exe_path: &str) -> Option<String> {
    let rgba = unsafe { draw_rgba(exe_path)? };
    let img = image::RgbaImage::from_raw(SIZE as u32, SIZE as u32, rgba)?;
    let mut png_bytes = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut png_bytes),
        image::ImageFormat::Png,
    )
    .ok()?;

    use base64::Engine;
    Some(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&png_bytes)
    ))
}

/// Draws the exe's icon into a 32×32 DIB and returns its pixels as RGBA.
unsafe fn draw_rgba(exe_path: &str) -> Option<Vec<u8>> {
    let path_wide: Vec<u16> = exe_path.encode_utf16().chain(std::iter::once(0)).collect();
    let mut shfi = SHFILEINFOW::default();
    let ret = SHGetFileInfoW(
        PCWSTR(path_wide.as_ptr()),
        Default::default(),
        Some(&mut shfi),
        mem::size_of::<SHFILEINFOW>() as u32,
        SHGFI_ICON,
    );
    if ret == 0 || shfi.hIcon.is_invalid() {
        return None;
    }
    let hicon = shfi.hIcon;

    let bi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: SIZE,
            biHeight: -SIZE,
            biPlanes: 1,
            biBitCount: 32,
            ..Default::default()
        },
        ..Default::default()
    };

    let dc = CreateCompatibleDC(None);
    let mut bits_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
    let hbm = match CreateDIBSection(
        dc,
        &bi,
        DIB_RGB_COLORS,
        &mut bits_ptr,
        HANDLE(std::ptr::null_mut()),
        0,
    ) {
        Ok(h) => h,
        Err(_) => {
            let _ = DeleteDC(dc);
            let _ = DestroyIcon(hicon);
            return None;
        }
    };

    let old_obj = SelectObject(dc, HGDIOBJ(hbm.0));
    let _ = DrawIconEx(
        dc,
        0,
        0,
        hicon,
        SIZE,
        SIZE,
        0,
        HBRUSH(std::ptr::null_mut()),
        DI_NORMAL,
    );

    let pixel_count = (SIZE * SIZE) as usize;
    let bgra = std::slice::from_raw_parts(bits_ptr as *const u8, pixel_count * 4);
    let mut rgba = vec![0u8; pixel_count * 4];
    for (dst, src) in rgba.chunks_exact_mut(4).zip(bgra.chunks_exact(4)) {
        dst.copy_from_slice(&[src[2], src[1], src[0], src[3]]);
    }

    SelectObject(dc, old_obj);
    let _ = DeleteObject(hbm);
    let _ = DeleteDC(dc);
    let _ = DestroyIcon(hicon);
    Some(rgba)
}
