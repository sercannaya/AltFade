use std::cell::RefCell;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use windows::{
    core::{Interface, GUID},
    Win32::Media::Audio::{
        eRender, Endpoints::IAudioMeterInformation, IAudioSessionControl, IAudioSessionControl2,
        IAudioSessionManager2, IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator,
        DEVICE_STATE_ACTIVE,
    },
    Win32::System::Com::{CoCreateInstance, CLSCTX_ALL},
};

use super::{com, icon, process};

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

pub struct FadeTarget {
    pub name: String,
    pub from: f32,
    pub to: f32,
}

/// How long a thread reuses its device list before enumerating again, so a
/// newly connected headset is picked up within a few seconds.
const DEVICE_CACHE_TTL: Duration = Duration::from_secs(5);

/// Windows' own audio hosts. They own sessions (e.g. for virtual devices) but
/// are never something to duck, so they stay out of the session list.
const SYSTEM_PROCESSES: &[&str] = &["audiodg.exe"];

/// The expensive half of a session walk: one session manager per active render
/// device (enumerator + `Activate` per device) and the pid → exe name lookups
/// (`OpenProcess` per session). Rebuilding these on every call made the 2 Hz
/// level-meter poll cost ~3× more CPU than needed.
///
/// WASAPI interfaces aren't `Send`, so each thread keeps its own cache, created
/// in that thread's apartment.
struct DeviceCache {
    built_at: Instant,
    managers: Vec<IAudioSessionManager2>,
    names: HashMap<u32, Option<String>>,
}

thread_local! {
    static CACHE: RefCell<Option<DeviceCache>> = const { RefCell::new(None) };
}

fn enumerate_managers() -> Vec<IAudioSessionManager2> {
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
        for i in 0..devices.GetCount().unwrap_or(0) {
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

/// Calls `f` with the pid, exe name and control of every audio session on
/// every active render device, so targets playing through a non-default output
/// are still found. `refresh` forces a fresh device enumeration.
fn for_each_session(refresh: bool, mut f: impl FnMut(u32, &str, &IAudioSessionControl)) {
    com::ensure_initialized();
    CACHE.with(|cell| {
        let mut slot = cell.borrow_mut();
        let stale = slot
            .as_ref()
            .is_none_or(|c| refresh || c.built_at.elapsed() >= DEVICE_CACHE_TTL);
        if stale {
            *slot = Some(DeviceCache {
                built_at: Instant::now(),
                managers: enumerate_managers(),
                names: HashMap::new(),
            });
        }
        let cache = slot.as_mut().expect("cache populated above");

        for manager in &cache.managers {
            let Ok(sessions) = (unsafe { manager.GetSessionEnumerator() }) else {
                continue;
            };
            for i in 0..unsafe { sessions.GetCount() }.unwrap_or(0) {
                let Ok(ctrl) = (unsafe { sessions.GetSession(i) }) else {
                    continue;
                };
                let Ok(pid) = ctrl
                    .cast::<IAudioSessionControl2>()
                    .and_then(|c| unsafe { c.GetProcessId() })
                else {
                    continue;
                };
                let name = cache.names.entry(pid).or_insert_with(|| {
                    process::exe_name(pid)
                        .filter(|n| !SYSTEM_PROCESSES.iter().any(|s| n.eq_ignore_ascii_case(s)))
                });
                if let Some(name) = name {
                    f(pid, name, &ctrl);
                }
            }
        }
    });
}

/// One entry per process that currently owns an audio session, with its icon.
pub fn list_sessions() -> Vec<AudioSession> {
    let mut pids: HashMap<String, u32> = HashMap::new();
    for_each_session(true, |pid, name, _| {
        pids.entry(name.to_owned()).or_insert(pid);
    });
    let mut sessions: Vec<AudioSession> = pids
        .into_iter()
        .map(|(name, pid)| AudioSession {
            icon: process::image_path(pid).and_then(|path| icon::data_url(&path)),
            name,
        })
        .collect();
    sessions.sort_by_key(|s| s.name.to_lowercase());
    sessions
}

/// Instantaneous peak level per process, aggregated as max across sessions.
pub fn session_peaks() -> Vec<SessionPeak> {
    let mut peaks: HashMap<String, f32> = HashMap::new();
    for_each_session(false, |_, name, ctrl| {
        let peak = ctrl
            .cast::<IAudioMeterInformation>()
            .and_then(|meter| unsafe { meter.GetPeakValue() })
            .unwrap_or(0.0);
        let entry = peaks.entry(name.to_owned()).or_insert(0.0);
        *entry = entry.max(peak);
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
    for_each_session(false, |_, name, ctrl| {
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
/// Blocks the calling thread for `duration_ms`.
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
            // Ease out when raising so the restore doesn't jump at the end.
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
        std::thread::sleep(Duration::from_millis(step_ms));
    }
}
