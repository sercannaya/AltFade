//! Windows audio plumbing.
//!
//! - [`sessions`]: per-process volume control and level metering through the
//!   WASAPI audio session APIs, across every active render device.
//! - [`media`]: "is something playing?" through the System Media Transport
//!   Controls (SMTC) that browsers, Spotify etc. publish to.
//!
//! Everything here is blocking COM/WinRT work. Callers must stay off the main
//! (UI) thread; see `docs/architecture.md`.

mod com;
mod icon;
mod media;
mod process;
mod sessions;

pub use media::{is_media_playing, list_media_sources, now_playing, MediaSource, NowPlaying};
pub use sessions::{
    fade_volumes, get_process_volume, list_sessions, session_peaks, set_process_volume,
    AudioSession, FadeTarget, SessionPeak,
};
