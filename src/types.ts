// Mirrors of the Rust payloads (serde field names) in src-tauri/src.

export interface AudioSession {
  name: string;
  icon: string | null;
}

export interface SessionPeak {
  name: string;
  peak: number;
}

export interface MediaSource {
  id: string;
  playing: boolean;
}

export interface NowPlaying {
  source: string;
  title: string;
  artist: string;
  playing: boolean;
}

export interface Status {
  ducked: boolean;
  now_playing: NowPlaying | null;
}

export interface Settings {
  targets: string[];
  duck_percent: number;
  unduck_delay_ms: number;
  trigger_apps: string[];
  ignored_sources: string[];
  hotkey_enabled: boolean;
  active: boolean;
  lang: string;
}
