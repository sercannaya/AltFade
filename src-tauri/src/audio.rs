use std::collections::HashMap;
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
            eRender, Endpoints::IAudioMeterInformation, IAudioSessionControl2,
            IAudioSessionManager2, IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator,
            DEVICE_STATE_ACTIVE,
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

#[derive(serde::Serialize, Clone)]
pub struct SessionPeak {
    pub name: String,
    pub peak: f32,
}

#[derive(serde::Serialize, Clone)]
pub struct MediaSource {
    pub id: String,
    pub playing: bool,
}

#[derive(serde::Serialize, Clone)]
pub struct NowPlaying {
    pub source: String,
    pub title: String,
    pub artist: String,
    pub playing: bool,
}

pub struct FadeTarget {
    pub name: String,
    pub from: f32,
    pub to: f32,
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

/// Session managers for every active render device, not just the default one,
/// so targets playing through a non-default output are still found.
fn session_managers() -> Vec<IAudioSessionManager2> {
    init_com();
    let mut managers = Vec::new();
    unsafe {
        let Ok(enumerator) =
            CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL)
        else {
            return managers;
        };
        let Ok(devices) = enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) else {
            return managers;
        };
        let count = devices.GetCount().unwrap_or(0);
        for i in 0..count {
            let Ok(device) = devices.Item(i) else {
                continue;
            };
            if let Ok(manager) = device.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None) {
                managers.push(manager);
            }
        }
    }
    managers
}

/// Calls `f` with the process name and session control of every audio session
/// on every active render device.
fn for_each_session(mut f: impl FnMut(&str, &windows::Win32::Media::Audio::IAudioSessionControl)) {
    for manager in session_managers() {
        unsafe {
            let Ok(enumerator) = manager.GetSessionEnumerator() else {
                continue;
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
                f(&name, &ctrl);
            }
        }
    }
}

pub fn list_sessions() -> Vec<AudioSession> {
    init_com();
    let mut pids: HashMap<String, u32> = HashMap::new();
    for manager in session_managers() {
        unsafe {
            let Ok(enumerator) = manager.GetSessionEnumerator() else {
                continue;
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
                pids.entry(name).or_insert(pid);
            }
        }
    }
    let mut sessions: Vec<AudioSession> = pids
        .into_iter()
        .map(|(name, pid)| {
            let icon = get_exe_path(pid).and_then(|p| get_icon_base64(&p));
            AudioSession { name, icon }
        })
        .collect();
    sessions.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    sessions
}

/// Instantaneous peak level per process, aggregated as max across sessions.
pub fn get_session_peaks() -> Vec<SessionPeak> {
    init_com();
    let mut peaks: HashMap<String, f32> = HashMap::new();
    for_each_session(|name, ctrl| unsafe {
        let Ok(meter) = ctrl.cast::<IAudioMeterInformation>() else {
            return;
        };
        let peak = meter.GetPeakValue().unwrap_or(0.0);
        let entry = peaks.entry(name.to_string()).or_insert(0.0);
        if peak > *entry {
            *entry = peak;
        }
    });
    peaks
        .into_iter()
        .map(|(name, peak)| SessionPeak { name, peak })
        .collect()
}

/// Resolves the volume interfaces of every audio session belonging to the
/// target process once, so callers don't re-enumerate on every fade step.
fn matching_session_volumes(target: &str) -> Vec<ISimpleAudioVolume> {
    let mut volumes = Vec::new();
    for_each_session(|name, ctrl| {
        if name.eq_ignore_ascii_case(target) {
            if let Ok(vol) = ctrl.cast::<ISimpleAudioVolume>() {
                volumes.push(vol);
            }
        }
    });
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

/// Fades several processes simultaneously, each with its own from/to level.
pub fn fade_volumes(targets: &[FadeTarget], duration_ms: u64, steps: u32) {
    let resolved: Vec<(&FadeTarget, Vec<ISimpleAudioVolume>)> = targets
        .iter()
        .map(|t| (t, matching_session_volumes(&t.name)))
        .filter(|(_, sessions)| !sessions.is_empty())
        .collect();
    if resolved.is_empty() {
        return;
    }
    let step_ms = (duration_ms / steps as u64).max(1);
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        for (target, sessions) in &resolved {
            let eased = if target.to > target.from {
                1.0 - (1.0 - t) * (1.0 - t)
            } else {
                t
            };
            let vol = (target.from + (target.to - target.from) * eased).clamp(0.0, 1.0);
            for session in sessions {
                unsafe {
                    let _ = session.SetMasterVolume(vol, &GUID::zeroed());
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(step_ms));
    }
}

fn media_session_manager() -> Option<GlobalSystemMediaTransportControlsSessionManager> {
    init_com();
    GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .ok()?
        .get()
        .ok()
}

/// Every app currently registered as a media source (Spotify, browsers, …),
/// identified by its AppUserModelId.
pub fn list_media_sources() -> Vec<MediaSource> {
    let Some(manager) = media_session_manager() else {
        return vec![];
    };
    let Ok(sessions) = manager.GetSessions() else {
        return vec![];
    };
    let mut sources: Vec<MediaSource> = Vec::new();
    let count = sessions.Size().unwrap_or(0);
    for i in 0..count {
        let Ok(session) = sessions.GetAt(i) else {
            continue;
        };
        let Ok(id) = session.SourceAppUserModelId() else {
            continue;
        };
        let id = id.to_string();
        if id.is_empty() {
            continue;
        }
        let playing = session
            .GetPlaybackInfo()
            .and_then(|info| info.PlaybackStatus())
            .map(|s| s == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing)
            .unwrap_or(false);
        if let Some(existing) = sources.iter_mut().find(|s| s.id == id) {
            existing.playing |= playing;
        } else {
            sources.push(MediaSource { id, playing });
        }
    }
    sources
}

/// Heuristic: does a media session's AppUserModelId belong to the given
/// process? Used to keep a target game from triggering its own ducking.
/// AUMIDs come in two shapes: a packaged reverse-DNS id ("Mojang.Minecraft")
/// or the launching executable's path ("C:\\Games\\game.exe", "...!game.exe").
/// A plain substring test catches both when the id embeds the exe name; as a
/// fallback we compare the id's leaf segment against the exe stem so a game run
/// from an unusual path is still recognised. Very short stems are ignored to
/// avoid a generic name accidentally matching an unrelated media source.
fn aumid_matches_process(aumid: &str, process: &str) -> bool {
    let aumid = aumid.to_lowercase();
    let process = process.to_lowercase();
    let stem = process.strip_suffix(".exe").unwrap_or(&process);
    if stem.len() < 3 {
        return false;
    }
    if aumid.contains(stem) {
        return true;
    }
    let leaf = aumid
        .rsplit(|c: char| matches!(c, '\\' | '/' | '!' | '.'))
        .find(|s| !s.is_empty())
        .unwrap_or(&aumid);
    leaf == stem
}

/// `trigger_apps` empty means any media source triggers; target processes
/// never trigger their own ducking. Trigger entries match by substring so a
/// generic name like "Spotify" covers both the desktop exe and the Store
/// package AppUserModelId. `ignored_sources` holds full AppUserModelIds the
/// user has explicitly excluded (matched case-insensitively) so a target that
/// also publishes a media session can never keep itself ducked.
pub fn is_media_playing(
    trigger_apps: &[String],
    exclude_processes: &[String],
    ignored_sources: &[String],
) -> bool {
    list_media_sources().iter().any(|source| {
        source.playing
            && (trigger_apps.is_empty()
                || trigger_apps.iter().any(|t| {
                    !t.is_empty() && source.id.to_lowercase().contains(&t.to_lowercase())
                }))
            && !ignored_sources
                .iter()
                .any(|i| source.id.eq_ignore_ascii_case(i))
            && !exclude_processes
                .iter()
                .any(|p| aumid_matches_process(&source.id, p))
    })
}

pub fn get_now_playing() -> Option<NowPlaying> {
    let manager = media_session_manager()?;
    let session = manager.GetCurrentSession().ok()?;
    let source = session
        .SourceAppUserModelId()
        .map(|s| s.to_string())
        .unwrap_or_default();
    let playing = session
        .GetPlaybackInfo()
        .and_then(|info| info.PlaybackStatus())
        .map(|s| s == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing)
        .unwrap_or(false);
    let props = session.TryGetMediaPropertiesAsync().ok()?.get().ok()?;
    let title = props.Title().map(|s| s.to_string()).unwrap_or_default();
    let artist = props.Artist().map(|s| s.to_string()).unwrap_or_default();
    if source.is_empty() && title.is_empty() {
        return None;
    }
    Some(NowPlaying {
        source,
        title,
        artist,
        playing,
    })
}
