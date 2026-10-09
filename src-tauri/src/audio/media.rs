use std::sync::Mutex;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as SessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
};

use super::com;

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

/// One SMTC manager shared by the duck loop and every UI command. It is agile
/// (`Send + Sync`) and stays valid for the life of the process; requesting a
/// fresh one per call costs ~1 ms of CPU plus a cross-process round trip, and
/// used to happen several times a second (see `docs/performance.md`).
static MANAGER: Mutex<Option<SessionManager>> = Mutex::new(None);

/// Runs `f` against the shared manager. A failing call drops the cached
/// manager (the shell's media service may have restarted) and is retried once
/// with a fresh one, so `f` must report "nothing there" as `Ok`, not `Err`.
fn with_manager<T>(f: impl Fn(&SessionManager) -> windows::core::Result<T>) -> Option<T> {
    com::ensure_initialized();
    for _ in 0..2 {
        let manager = {
            let mut cached = MANAGER.lock().unwrap();
            if cached.is_none() {
                *cached = SessionManager::RequestAsync().ok()?.get().ok();
            }
            cached.clone()?
        };
        match f(&manager) {
            Ok(value) => return Some(value),
            Err(_) => *MANAGER.lock().unwrap() = None,
        }
    }
    None
}

/// windows-rs reports a null object as an `Err` carrying a success code; turn
/// that into `None` so it isn't mistaken for a broken manager.
fn optional<T>(result: windows::core::Result<T>) -> windows::core::Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(e) if e.code().is_ok() => Ok(None),
        Err(e) => Err(e),
    }
}

fn is_playing(session: &Session) -> bool {
    session
        .GetPlaybackInfo()
        .and_then(|info| info.PlaybackStatus())
        .map(|status| status == PlaybackStatus::Playing)
        .unwrap_or(false)
}

/// Every app currently registered as a media source (Spotify, browsers, …),
/// identified by its AppUserModelId.
pub fn list_media_sources() -> Vec<MediaSource> {
    with_manager(|manager| {
        let sessions = manager.GetSessions()?;
        let mut sources: Vec<MediaSource> = Vec::new();
        for i in 0..sessions.Size()? {
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
            let playing = is_playing(&session);
            if let Some(existing) = sources.iter_mut().find(|s| s.id == id) {
                existing.playing |= playing;
            } else {
                sources.push(MediaSource { id, playing });
            }
        }
        Ok(sources)
    })
    .unwrap_or_default()
}

/// Heuristic: does a media session's AppUserModelId belong to the given
/// process? Used to keep a target game from triggering its own ducking.
/// AUMIDs come in two shapes: a packaged reverse-DNS id ("Mojang.Minecraft")
/// or the launching executable's path ("C:\\Games\\game.exe", "...!game.exe");
/// a substring test on the exe stem catches both when the id embeds the exe
/// name. Store ids that don't (e.g. "Microsoft.MinecraftUWP_…!App") can't be
/// matched; that is what `ignored_sources` is for. Very short stems are
/// ignored so a generic name can't match an unrelated media source.
fn aumid_matches_process(aumid: &str, process: &str) -> bool {
    let process = process.to_lowercase();
    let stem = process.strip_suffix(".exe").unwrap_or(&process);
    stem.len() >= 3 && aumid.to_lowercase().contains(stem)
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
                || trigger_apps
                    .iter()
                    .any(|t| !t.is_empty() && source.id.to_lowercase().contains(&t.to_lowercase())))
            && !ignored_sources
                .iter()
                .any(|i| source.id.eq_ignore_ascii_case(i))
            && !exclude_processes
                .iter()
                .any(|p| aumid_matches_process(&source.id, p))
    })
}

/// The session Windows considers current (what the media flyout shows).
pub fn now_playing() -> Option<NowPlaying> {
    let session = with_manager(|manager| optional(manager.GetCurrentSession()))??;
    let source = session
        .SourceAppUserModelId()
        .map(|s| s.to_string())
        .unwrap_or_default();
    // Served by the media app itself, not the shell; failures here say nothing
    // about the shared manager, so they stay outside `with_manager`.
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
        playing: is_playing(&session),
    })
}

#[cfg(test)]
mod tests {
    use super::aumid_matches_process;

    #[test]
    fn matches_exe_path_aumid() {
        assert!(aumid_matches_process(
            r"C:\Games\Witcher3\witcher3.exe",
            "witcher3.exe"
        ));
        assert!(aumid_matches_process("SomeLauncher!NMS.exe", "NMS.exe"));
    }

    #[test]
    fn packaged_aumid_must_embed_the_stem() {
        assert!(aumid_matches_process("Mojang.Minecraft", "minecraft.exe"));
        assert!(!aumid_matches_process(
            "Microsoft.MinecraftUWP_8wekyb3d8bbwe!App",
            "Minecraft.Windows.exe"
        ));
    }

    #[test]
    fn ignores_short_stems_and_unrelated_sources() {
        assert!(!aumid_matches_process("Spotify.exe", "ab.exe"));
        assert!(!aumid_matches_process("Spotify.exe", "chrome.exe"));
    }
}
