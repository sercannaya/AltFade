use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use windows::{
    core::{Interface, GUID},
    Media::Control::{
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus,
    },
    Win32::{
        Foundation::{CloseHandle, HMODULE},
        Media::Audio::{
            eConsole, eRender, IAudioSessionControl2, IAudioSessionManager2,
            IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator,
        },
        System::{
            Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED},
            ProcessStatus::K32GetModuleBaseNameW,
            Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
        },
    },
};

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
        Some(
            OsString::from_wide(&buf[..len as usize])
                .to_string_lossy()
                .into_owned(),
        )
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

fn init_com() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

pub fn list_sessions() -> Vec<String> {
    init_com();
    let Ok(manager) = session_manager() else {
        return vec![];
    };
    let mut names: Vec<String> = Vec::new();
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
            if let Some(name) = get_process_name(pid) {
                if !names.contains(&name) {
                    names.push(name);
                }
            }
        }
    }
    names
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

pub fn fade_process_volume(target: &str, from: f32, to: f32, duration_ms: u64, steps: u32) {
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
        set_process_volume(target, vol);
        std::thread::sleep(std::time::Duration::from_millis(step_ms));
    }
}

pub fn set_process_volume(target: &str, volume: f32) {
    init_com();
    let Ok(manager) = session_manager() else {
        return;
    };
    unsafe {
        let Ok(enumerator) = manager.GetSessionEnumerator() else {
            return;
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
            if let Some(name) = get_process_name(pid) {
                if name.eq_ignore_ascii_case(target) {
                    if let Ok(vol) = ctrl.cast::<ISimpleAudioVolume>() {
                        let _ = vol.SetMasterVolume(volume, &GUID::zeroed());
                    }
                }
            }
        }
    }
}
