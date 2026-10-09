# AltFade

Windows-only Tauri v2 tray app that ducks game audio while other media plays
(WASAPI per-process volume + SMTC media detection). Rust backend in
`src-tauri/`, Vue 3 + TypeScript UI in `src/`.

- Architecture, modules, threads, data flow: [docs/architecture.md](docs/architecture.md)
- Performance history and rules: [docs/performance.md](docs/performance.md)
- Setup, commands, release process, conventions: [docs/development.md](docs/development.md)

## Commands

```powershell
npm ci                                   # JS deps
npm run tauri dev                        # run the app (dev)
npm run build                            # vue-tsc type-check + vite build
cd src-tauri; cargo fmt                  # format the backend
cd src-tauri; cargo clippy --all-targets # lint (keep it warning-free)
cd src-tauri; cargo test                 # unit tests
```

Before calling a change done, run `npm run build`, `cargo fmt`,
`cargo clippy --all-targets` and `cargo test`.

## Rules that are easy to break

- **The app lives in the tray next to games: idle CPU must stay ~0.** Read
  [docs/performance.md](docs/performance.md) before touching polling,
  animations, window visibility or COM code.
  - Show/hide the main window only via `window::show_main` /
    `window::hide_main`. A bare `win.hide()` leaves WebView2 rendering and
    polling in the background.
  - UI polling goes through `usePoll` (pauses while hidden, never overlaps).
    Prefer backend events over polling.
  - No CSS transitions or infinite animations on periodically updated data.
  - Cache COM/WinRT objects. The SMTC manager is shared (`media::MANAGER`);
    WASAPI objects are cached per thread (`sessions::CACHE`).
- **Never block the main thread with audio work.** Commands that touch
  COM/WinRT must be `async fn`. Call `com::ensure_initialized()` in every
  new audio entry point.
- **`Settings` field names are the on-disk config format** (and the
  frontend's `types.ts`). Don't rename them; add fields with defaults.
- **A new command touches four places:** `commands.rs`, `generate_handler!`
  in `lib.rs`, `src/api.ts`, and `src/types.ts` for any new payload.
- **UI strings go in both** `src/locales/en.json` and `tr.json`. Tray
  strings are in `src-tauri/src/tray.rs`.
- **Versions live in three files:** `package.json`, `src-tauri/Cargo.toml`,
  `src-tauri/tauri.conf.json`. Bump them together.
- A dev build shares the identifier and config with an installed AltFade:
  the installed one must be quit first, and a dev build with the service on
  ducks real apps.

## Domain notes

- Targets are exe names (`game.exe`), matched case-insensitively against
  audio session owners on **all** active render devices.
- Media sources are SMTC AppUserModelIds. Trigger entries match by substring;
  `ignored_sources` by exact id. Empty trigger list = any source triggers.
- A target that publishes its own media session must never trigger its own
  ducking (`aumid_matches_process`, plus `ignored_sources` for ids the
  heuristic can't match).
- Docs, code comments and commit messages are in English. Commits follow
  Conventional Commits.
