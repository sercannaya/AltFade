# Development

## Prerequisites

- Windows 10/11. The app is Windows-only; the crate refuses to compile
  elsewhere.
- Node.js 20+ and npm.
- Rust stable (MSVC toolchain) with the Visual Studio C++ build tools.
- WebView2 Runtime (preinstalled on Windows 11).

## Commands

| Task | Command |
|---|---|
| Install JS deps | `npm ci` |
| Run the app in dev mode (Vite + debug Rust build) | `npm run tauri dev` |
| Type-check and build the frontend | `npm run build` |
| Format the backend | `cargo fmt` (in `src-tauri/`) |
| Lint the backend | `cargo clippy --all-targets` (in `src-tauri/`) |
| Backend unit tests | `cargo test` (in `src-tauri/`) |
| Release build without installers | `npx tauri build --no-bundle` |
| Full release build (NSIS + MSI) | `npx tauri build` (needs the updater signing key, see below) |

Notes:

- A dev build and an installed AltFade share the same identifier, so the
  single-instance plugin makes the second one just focus the first. Quit the
  installed app from the tray first, or build with a different identifier
  (see [performance.md](performance.md#how-to-measure)).
- They also share `%APPDATA%\com.sercannaya.altfade\config.json`. A dev
  build with the service on **will duck your real apps**.
- Debug builds never register the autostart entry (antivirus heuristics
  flag unsigned exes autostarting from build folders).
- Games running as administrator: names now resolve, so they appear in the
  session list. Whether their volume can be changed without running
  AltFade elevated hasn't been verified yet.

## Releasing

1. Bump the version in **all three** places: `package.json`,
   `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`. Cargo will update
   `Cargo.lock` on the next build.
2. Merge `dev` into `master` through a PR, then push a tag `vX.Y.Z`.
3. `.github/workflows/release.yml` builds on `windows-latest` with
   `tauri-apps/tauri-action` and creates a **draft** GitHub release with the
   installers and `latest.json`. It needs the `TAURI_SIGNING_PRIVATE_KEY`
   secret; the matching public key is `plugins.updater.pubkey` in
   `tauri.conf.json`.
4. Publish the draft. Installed apps check
   `releases/latest/download/latest.json` at startup and show an update
   banner.

## Conventions

- Commit messages follow Conventional Commits (`feat:`, `fix:`, `refactor:`,
  `chore:`, `docs:`, `style:`, `ci:`, `build:`).
- Branches: work happens on `feature/<topic>` or `bugfix/<topic>`, merged
  into `dev` (the integration branch, and the PR base). `master` only
  receives `dev` for releases.
- UI strings go in both `src/locales/en.json` and `src/locales/tr.json`.
  Tray strings are in `src-tauri/src/tray.rs`.
- Adding a command means touching four places: the function in
  `commands.rs`, `generate_handler!` in `lib.rs`, the wrapper in `src/api.ts`,
  and any payload type in `src/types.ts`.
- Adding a setting means a field on `Settings` (with a default), a command
  to change it, and the matching field in `src/types.ts`. Existing configs
  must keep loading (`#[serde(default)]`).
