# Performance

AltFade spends almost all its life hidden in the tray, often next to a game.
Its idle cost must stay close to zero. This page records why it once wasn't,
what fixed it, and the rules that keep it that way.

## The 1.1.2 CPU problem

Users saw AltFade using noticeable CPU "for no reason", worst while gaming.
Measured on an i5 (14th gen, 16 threads) with a 165 Hz display. Numbers are
CPU time as a percentage of **one core**, summed over `alt-fade.exe` and all
its `msedgewebview2.exe` children:

| Scenario | 1.1.2 | After fix |
|---|---|---|
| Hidden in tray, service on, media playing (ducking) | 32–39 % | 0.2–0.4 % |
| Hidden in tray, service off | 17–25 % | 0.4–0.8 % |
| Autostart (`--minimized`, never opened) | 17–24 % | 0.3–0.4 % |
| Window open, audio playing | 13–17 % | 3–4 % |

### Root causes

1. **WebView2 never knew the window was hidden.** Close-to-tray called
   `window.hide()`, which hides the native window only. The WebView2
   controller's `IsVisible` stayed `true`, so the page kept rendering
   (compositor and GPU process) and `document.hidden` stayed `false`. The
   polling guard (`if (document.hidden) return`) therefore never fired, and
   the UI polled the backend three times a second forever. The autostart
   path was hit too: the window is created hidden but the webview starts
   "visible". Cost was highest with the service on and media playing
   (32–39 %), the state where the UI also runs its infinite "ducking" pulse
   animation, which a hidden-but-"visible" page keeps rendering.
   **Fix:** `window.rs` hides and shows the webview together with the
   window (`Webview::hide/show` → `ICoreWebView2Controller::SetIsVisible`),
   and also syncs it on minimize/restore. `usePoll` stops and starts on
   `visibilitychange`.
2. **Level meters animated layout at the refresh rate.** Each meter used
   `transition: width 0.25s`, retriggered every 500 ms while anything made
   sound. While a game runs, sound never stops. Even as a compositor-only
   `transform` transition it still cost ~10 % of a core at 165 Hz.
   **Fix:** meters use `transform: scaleX()` with no transition.
3. **Backend recreated COM/WinRT objects on every call.**
   - A new SMTC manager (`RequestAsync().get()`) on every
     `is_media_playing` (duck loop, 2 Hz) and `get_now_playing` (UI, 1 Hz):
     ~0.8 ms CPU and ~3.5 ms of cross-process waiting each. Cached: ~0.
   - A new device enumerator, `Activate` per render device, and
     `OpenProcess` per session on every peak poll (2 Hz): ~1.6 ms each.
     Cached: ~0.5 ms.

   **Fix:** shared SMTC manager with retry-on-failure (`media::with_manager`),
   thread-local device/session-name cache with a 5 s TTL
   (`sessions::DeviceCache`), and an icon cache.

Process names are now resolved with `PROCESS_QUERY_LIMITED_INFORMATION`
instead of `PROCESS_QUERY_INFORMATION | PROCESS_VM_READ`. Opening a game with
VM-read rights twice a second is the kind of access anti-cheat drivers
filter and log. The new right also works on elevated processes, so admin
games now show up in the session list. `audiodg.exe` is filtered out
explicitly, since it used to be hidden only by accident.

## Rules

- **Never poll from the UI unless the page is visible.** Use `usePoll`. Don't
  hand-roll `setInterval`.
- **Always hide/show the main window through `window::show_main` /
  `window::hide_main`.** Calling `win.hide()` directly brings back problem 1.
- **Prefer push over poll.** If the backend already knows when something
  changes (ducked, active), emit an event.
- **No continuous CSS animation or transition on data that updates
  periodically.** Anything animated at the display refresh rate costs about
  10 % of a core while visible. One-shot hover/toggle transitions are fine.
  The ducking pulse on the status dot is the one deliberate exception: it
  only runs while the window is open and ducking.
- **Cache COM/WinRT objects. Don't recreate them per call.** The SMTC manager
  is agile and can be shared. WASAPI interfaces aren't `Send`, so cache them
  per thread.
- **Keep blocking audio work off the main thread.** Commands that touch
  COM/WinRT must be `async fn`.

## How to measure

Run a release build with a separate identifier, so it neither collides with
an installed AltFade (single-instance lock, config, autostart) nor touches
real audio:

```powershell
npx tauri build --no-bundle --config '{\"identifier\":\"com.sercannaya.altfade.perftest\",\"bundle\":{\"createUpdaterArtifacts\":false}}'
```

Then sample `TotalProcessorTime` of `alt-fade.exe` plus all descendant
`msedgewebview2.exe` processes over 20–30 s. Split the WebView2 processes by
their `--type=` (renderer, gpu-process, …), measure the states above, and let
the window settle for ~20 s after launch, since WebView2 start-up skews the
first seconds. Test with audio playing and with silence: many costs only
appear while something makes sound. Delete
`%APPDATA%\com.sercannaya.altfade.perftest` and
`%LOCALAPPDATA%\com.sercannaya.altfade.perftest` afterwards.
