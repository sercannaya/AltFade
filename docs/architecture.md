# Architecture

AltFade is a Windows-only Tauri v2 tray app. It lowers ("ducks") the volume
of chosen apps, usually games, while another app plays media, then fades them
back when the media stops.

```
┌──────────────── WebView2 (Vue 3) ────────────────┐
│  App.vue ─ components/*  ── store.ts ── api.ts   │  UI: settings, live meters
└───────────────────────┬──────────────────────────┘
          invoke (IPC)  │  ▲  events: active-changed, ducked-changed,
                        ▼  │          update-available
┌──────────────── Rust (src-tauri/src) ────────────┐
│  commands.rs ── settings.rs / state.rs           │
│  ducker.rs   (duck loop thread, 500 ms tick)     │
│  tray.rs, hotkey.rs, window.rs, updater.rs       │
│  audio/  sessions.rs (WASAPI)  media.rs (SMTC)   │
└──────────────────────────────────────────────────┘
```

## How ducking works

1. **Detect media.** `audio::media` asks the System Media Transport Controls
   (SMTC, the API behind the Windows media flyout) which apps have a media
   session and whether it is `Playing`. Sources are identified by their
   AppUserModelId (AUMID), e.g. `SpotifyAB.SpotifyMusic_…!Spotify` or
   `Chrome`.
2. **Filter triggers.** `is_media_playing` counts a playing source only if:
   - the trigger list is empty, or the AUMID contains one of the trigger
     entries (case-insensitive substring);
   - it isn't in `ignored_sources` (exact AUMID match);
   - it doesn't look like one of the targets itself (`aumid_matches_process`).
     Otherwise a game that publishes its own media session would keep itself
     ducked forever.
3. **Duck / restore.** `ducker.rs` runs on its own thread and ticks every
   500 ms. When media starts, it snapshots each target's current volume
   (`DuckedProcess`), fades down to `duck_percent` (never raising an already
   quieter target), and holds the duck. When media stops, it waits
   `unduck_delay_ms` so track changes don't pump the volume, then fades back
   to the snapshot. Changing the target list while ducked restores the old
   set first. Changing the duck level while ducked applies live.
4. **Volume control.** `audio::sessions` walks the WASAPI audio sessions on
   **every active render device**, not just the default one, and sets
   `ISimpleAudioVolume` on all sessions whose process exe name matches a
   target.

`ducker::restore_now` puts volumes back instantly on exit and before an
update restarts the app.

## Backend modules (`src-tauri/src`)

| Module | Responsibility |
|---|---|
| `lib.rs` | Builder wiring: plugins, command list, setup, window events. |
| `commands.rs` | Every `#[tauri::command]`. Mirrored by `src/api.ts`. |
| `settings.rs` | `Settings` (persisted JSON), load/save, `settings::update`. |
| `state.rs` | `AppState` (settings, ducked snapshot, mica flag), `set_active`. |
| `ducker.rs` | The duck loop and its fade parameters. |
| `audio/media.rs` | SMTC: media sources, now playing, trigger filtering. |
| `audio/sessions.rs` | WASAPI: session list, peaks, get/set/fade volume. |
| `audio/process.rs` | pid → exe path/name (`PROCESS_QUERY_LIMITED_INFORMATION`). |
| `audio/icon.rs` | Exe icon → PNG data URL, cached per path. |
| `audio/com.rs` | Per-thread COM (MTA) initialization. |
| `tray.rs` | Tray icon (status dot), tooltip, menu. |
| `window.rs` | Show/hide the main window **and its webview**, Mica. |
| `hotkey.rs` | Global `Ctrl+Alt+D` toggle. |
| `updater.rs` | Startup update check and install. |

### Threads and COM

- **Main thread**: Tao event loop, WebView2, tray. It is an STA. Never block
  it with audio work.
- **Duck loop** (`duck-loop` thread): the only thing that changes volumes
  outside of exit/update.
- **Tokio workers**: async Tauri commands. Every command that touches
  COM/WinRT is `async fn` so it runs here instead of on the main thread.
- Every audio entry point calls `com::ensure_initialized()` (MTA, once per
  thread).
- The SMTC manager is agile and shared process-wide (`media::MANAGER`).
  WASAPI interfaces are not `Send`, so the device/session cache is
  **thread-local** (`sessions::CACHE`) and rebuilt every 5 s.

## Frontend (`src/`)

| File | Responsibility |
|---|---|
| `main.ts` | Creates the app, installs i18n, global CSS. |
| `App.vue` | Layout, start/stop button, autostart switch, refresh-on-show. |
| `components/*.vue` | One card each: header, update banner, status, sessions, triggers, settings. |
| `store.ts` | Shared reactive `state` and all actions (optimistic update, then persist through `api`). |
| `api.ts` | Typed wrappers for every command and backend event. |
| `types.ts` | TS mirrors of the Rust payloads. |
| `composables/usePoll.ts` | Visibility-aware, non-overlapping polling. |
| `composables/useAppearance.ts` | Theme, accent color, Mica flag (localStorage). |
| `lib/sources.ts` | AUMID → chip label, chip list building. |
| `locales/{en,tr}.json` | UI strings. The tray strings live in `tray.rs`. |

The UI polls only while it is visible:

- `get_session_peaks` every 500 ms (`SessionList`), for the live meters;
- `get_status` every 1 s (`StatusCard`), for "now playing".

The ducked state is also **pushed** (`ducked-changed`), and so is the
service on/off state (`active-changed`, since the tray and hotkey can change
it). See [performance.md](performance.md) for why visibility matters so much
here.

## Persistence

- `%APPDATA%\com.sercannaya.altfade\config.json`: `Settings` (targets,
  levels, triggers, ignored sources, hotkey, service state, language). The
  serde field names are the on-disk format; renaming one breaks existing
  configs. Pre-1.1 configs with a single `target` are migrated on load.
- `localStorage` (WebView2 profile): theme and accent color only.
- Autostart: the `tauri-plugin-autostart` Run-key entry, launched with
  `--minimized` (starts in the tray). Debug builds never register it.
