import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AudioSession, MediaSource, SessionPeak, Settings, Status } from "./types";

// Typed wrappers for the commands in src-tauri/src/commands.rs.
export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  getStatus: () => invoke<Status>("get_status"),
  getAudioSessions: () => invoke<AudioSession[]>("get_audio_sessions"),
  getSessionPeaks: () => invoke<SessionPeak[]>("get_session_peaks"),
  getMediaSources: () => invoke<MediaSource[]>("get_media_sources"),
  setTargets: (names: string[]) => invoke<void>("set_targets", { names }),
  setTriggerApps: (apps: string[]) => invoke<void>("set_trigger_apps", { apps }),
  setIgnoredSources: (sources: string[]) => invoke<void>("set_ignored_sources", { sources }),
  setDuckVolume: (percent: number) => invoke<void>("set_duck_volume", { percent }),
  setUnduckDelay: (ms: number) => invoke<void>("set_unduck_delay", { ms }),
  toggleDucking: () => invoke<boolean>("toggle_ducking"),
  setLanguage: (lang: string) => invoke<void>("set_language", { lang }),
  setHotkeyEnabled: (enabled: boolean) => invoke<void>("set_hotkey_enabled", { enabled }),
  isMicaActive: () => invoke<boolean>("is_mica_active"),
  setWindowTheme: (dark: boolean) => invoke<void>("set_window_theme", { dark }),
  installUpdate: () => invoke<void>("install_update"),
  getAutostartEnabled: () => invoke<boolean>("get_autostart_enabled"),
  setAutostartEnabled: (enabled: boolean) => invoke<void>("set_autostart_enabled", { enabled }),
};

// Events the backend pushes, so the UI doesn't have to poll for them.
export const events = {
  onActiveChanged: (cb: (active: boolean) => void) =>
    listen<boolean>("active-changed", (e) => cb(e.payload)),
  onDuckedChanged: (cb: (ducked: boolean) => void) =>
    listen<boolean>("ducked-changed", (e) => cb(e.payload)),
  onUpdateAvailable: (cb: (version: string) => void) =>
    listen<string>("update-available", (e) => cb(e.payload)),
};
