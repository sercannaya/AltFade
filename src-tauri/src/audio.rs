use std::ffi::OsString;
use std::mem;
use std::os::windows::ffi::OsStringExt;
use windows::{
    core::{Interface, GUID, PCWSTR},
    Media::Control::{
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus,
    },
    Win32::{
        Foundation::{CloseHandle, HANDLE, HMODULE},
        Graphics::Gdi::{
            CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject,
            BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, HBRUSH, HGDIOBJ,
        },
        Media::Audio::{
            eConsole, eRender, IAudioSessionControl2, IAudioSessionManager2,
            IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator,
        },
        System::{
            Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED},
            ProcessStatus::K32GetModuleBaseNameW,
            Threading::{
                OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
                PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
            },
        },
        UI::{
            Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON},
            WindowsAndMessaging::{DestroyIcon, DrawIconEx, DI_NORMAL},
        },
    },
};

#[derive(serde::Serialize, Clone)]
pub struct AudioSession {
    pub name: String,
    pub icon: Option<String>,
}

thread_local! {
    static COM_INITIALIZED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn init_com() {
    COM_INITIALIZED.with(|initialized| {
        if !initialized.get() {
            unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            }
            initialized.set(true);
        }
    });
}

fn get_process_name(pid: u32) -> Option<String> {
    if pid == 0 {
        return None;
    }
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid).ok()?;
        let mut buf = [0u16; 260];
        let len = K32GetModuleBaseNameW(handle, HMODULE(std::ptr::null_mut()), &mut buf);
        let _ = CloseHandle(handle);
        if len == 0 {
            return None;
        }
        Some(OsString::from_wide(&buf[..len as usize]).to_string_lossy().into_owned())
    }
}

fn get_exe_path(pid: u32) -> Option<String> {
    if pid == 0 {
        return None;
    }
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut size = buf.len() as u32;
        let result = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut size,
        );
        let _ = CloseHandle(handle);
        result.ok()?;
        if size == 0 {
            return None;
        }
        Some(OsString::from_wide(&buf[..size as usize]).to_string_lossy().into_owned())
    }
}

fn get_icon_base64(exe_path: &str) -> Option<String> {
    const SIZE: i32 = 32;
    unsafe {
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
        for i in 0..pixel_count {
            rgba[i * 4] = bgra[i * 4 + 2];
            rgba[i * 4 + 1] = bgra[i * 4 + 1];
            rgba[i * 4 + 2] = bgra[i * 4];
            rgba[i * 4 + 3] = bgra[i * 4 + 3];
        }

        SelectObject(dc, old_obj);
        let _ = DeleteObject(hbm);
        let _ = DeleteDC(dc);
        let _ = DestroyIcon(hicon);

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
}

fn session_manager() -> windows::core::Result<IAudioSessionManager2> {
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
        device.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None)
    }
}

pub fn list_sessions() -> Vec<AudioSession> {
    init_com();
    let Ok(manager) = session_manager() else {
        return vec![];
    };
    let mut seen: Vec<String> = Vec::new();
    let mut sessions: Vec<AudioSession> = Vec::new();
    unsafe {
        let Ok(enumerator) = manager.GetSessionEnumerator() else {
            return vec![];
        };
        let count = enumerator.GetCount().unwrap_or(0);
        for i in 0..count {
            let Ok(ctrl) = enumerator.GetSession(i) else {
                continue;
            };
            let Ok(ctrl2) = ctrl.cast::<IAudioSessionControl2>() else {
                continue;
            };
            let Ok(pid) = ctrl2.GetProcessId() else {
                continue;
            };
            let Some(name) = get_process_name(pid) else {
                continue;
            };
            if seen.contains(&name) {
                continue;
            }
            seen.push(name.clone());
            let icon = get_exe_path(pid).and_then(|p| get_icon_base64(&p));
            sessions.push(AudioSession { name, icon });
        }
    }
    sessions
}

pub fn is_media_playing() -> bool {
    init_com();
    let Ok(op) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync() else {
        return false;
    };
    let Ok(manager) = op.get() else {
        return false;
    };
    let Ok(session) = manager.GetCurrentSession() else {
        return false;
    };
    let Ok(info) = session.GetPlaybackInfo() else {
        return false;
    };
    let Ok(status) = info.PlaybackStatus() else {
        return false;
    };
    status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing
}

/// Resolves the volume interfaces of every audio session belonging to the
/// target process once, so callers don't re-enumerate on every fade step.
fn matching_session_volumes(target: &str) -> Vec<ISimpleAudioVolume> {
    init_com();
    let Ok(manager) = session_manager() else {
        return vec![];
    };
    let mut volumes = Vec::new();
    unsafe {
        let Ok(enumerator) = manager.GetSessionEnumerator() else {
            return volumes;
        };
        let count = enumerator.GetCount().unwrap_or(0);
        for i in 0..count {
            let Ok(ctrl) = enumerator.GetSession(i) else {
                continue;
            };
            let Ok(ctrl2) = ctrl.cast::<IAudioSessionControl2>() else {
                continue;
            };
            let Ok(pid) = ctrl2.GetProcessId() else {
                continue;
            };
            let Some(name) = get_process_name(pid) else {
                continue;
            };
            if name.eq_ignore_ascii_case(target) {
                if let Ok(vol) = ctrl.cast::<ISimpleAudioVolume>() {
                    volumes.push(vol);
                }
            }
        }
    }
    volumes
}

pub fn get_process_volume(target: &str) -> Option<f32> {
    let volumes = matching_session_volumes(target);
    let vol = volumes.first()?;
    unsafe { vol.GetMasterVolume().ok() }
}

pub fn set_process_volume(target: &str, volume: f32) {
    for vol in matching_session_volumes(target) {
        unsafe {
            let _ = vol.SetMasterVolume(volume, &GUID::zeroed());
        }
    }
}

pub fn fade_process_volume(target: &str, from: f32, to: f32, duration_ms: u64, steps: u32) {
    let sessions = matching_session_volumes(target);
    if sessions.is_empty() {
        return;
    }
    let step_ms = (duration_ms / steps as u64).max(1);
    let fading_up = to > from;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let eased = if fading_up {
            1.0 - (1.0 - t) * (1.0 - t)
        } else {
            t
        };
        let vol = (from + (to - from) * eased).clamp(0.0, 1.0);
        for session in &sessions {
            unsafe {
                let _ = session.SetMasterVolume(vol, &GUID::zeroed());
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(step_ms));
    }
}
