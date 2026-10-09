//! The background loop that ducks target apps while media plays and restores
//! them afterwards.

use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

use crate::{audio, settings::Settings, state::AppState, tray};

/// How often the loop re-checks media playback. Each check is a few
/// in-process calls on the cached SMTC manager, so this is cheap.
const TICK: Duration = Duration::from_millis(500);

/// Fade shapes as (duration ms, steps).
const DUCK_FADE: (u64, u32) = (400, 16);
const RESTORE_FADE: (u64, u32) = (1500, 30);
const STOP_FADE: (u64, u32) = (1000, 20);
const RETARGET_FADE: (u64, u32) = (300, 10);

/// Snapshot taken when ducking starts, so the exact processes and their
/// pre-duck volumes can be restored even if the target selection changes.
#[derive(Clone)]
pub struct DuckedProcess {
    pub name: String,
    pub original_volume: f32,
    pub applied_volume: f32,
}

#[derive(Clone)]
pub struct DuckedState {
    pub processes: Vec<DuckedProcess>,
}

impl DuckedState {
    fn fade_down(&self, (duration_ms, steps): (u64, u32)) {
        let fades: Vec<audio::FadeTarget> = self
            .processes
            .iter()
            .map(|p| audio::FadeTarget {
                name: p.name.clone(),
                from: p.original_volume,
                to: p.applied_volume,
            })
            .collect();
        audio::fade_volumes(&fades, duration_ms, steps);
    }

    fn fade_back(&self, (duration_ms, steps): (u64, u32)) {
        let fades: Vec<audio::FadeTarget> = self
            .processes
            .iter()
            .map(|p| audio::FadeTarget {
                name: p.name.clone(),
                from: p.applied_volume,
                to: p.original_volume,
            })
            .collect();
        audio::fade_volumes(&fades, duration_ms, steps);
    }
}

/// Restores ducked targets instantly (no fade). Used on exit and before an
/// update restarts the app.
pub fn restore_now(state: &AppState) {
    let taken = state.ducked.lock().unwrap().take();
    if let Some(ducked) = taken {
        for process in &ducked.processes {
            audio::set_process_volume(&process.name, process.original_volume);
        }
    }
}

pub fn spawn(app: AppHandle) {
    std::thread::Builder::new()
        .name("duck-loop".into())
        .spawn(move || {
            let mut engine = Engine {
                app,
                last_playing_at: None,
            };
            loop {
                std::thread::sleep(TICK);
                engine.tick();
            }
        })
        .expect("failed to spawn duck loop");
}

struct Engine {
    app: AppHandle,
    last_playing_at: Option<Instant>,
}

impl Engine {
    fn tick(&mut self) {
        let state = self.app.state::<AppState>();
        let settings: Option<Settings> = {
            let settings = state.settings.lock().unwrap();
            settings.active.then(|| settings.clone())
        };
        let ducked = state.ducked.lock().unwrap().clone();

        let Some(settings) = settings else {
            if let Some(d) = ducked {
                d.fade_back(STOP_FADE);
                self.store(None);
            }
            self.last_playing_at = None;
            return;
        };

        let playing = audio::is_media_playing(
            &settings.trigger_apps,
            &settings.targets,
            &settings.ignored_sources,
        );
        if playing {
            self.last_playing_at = Some(Instant::now());
        }

        match ducked {
            None if playing && !settings.targets.is_empty() => {
                let duck_vol = settings.duck_volume();
                let processes = settings
                    .targets
                    .iter()
                    .map(|name| {
                        let original = audio::get_process_volume(name).unwrap_or(1.0);
                        DuckedProcess {
                            name: name.clone(),
                            original_volume: original,
                            // Never raise a target that is already quieter.
                            applied_volume: duck_vol.min(original),
                        }
                    })
                    .collect();
                let ducked = DuckedState { processes };
                ducked.fade_down(DUCK_FADE);
                self.store(Some(ducked));
            }
            None => {}
            Some(d)
                if d.processes
                    .iter()
                    .map(|p| &p.name)
                    .ne(settings.targets.iter()) =>
            {
                // Target list changed: restore the old set; the next tick
                // re-ducks the new one if media is still playing.
                d.fade_back(RETARGET_FADE);
                self.store(None);
            }
            Some(mut d) if playing => {
                // Duck level changed from the slider while ducked: apply it live.
                let duck_vol = settings.duck_volume();
                let mut changed = false;
                for process in d.processes.iter_mut() {
                    let want = duck_vol.min(process.original_volume);
                    if (want - process.applied_volume).abs() > 0.001 {
                        audio::set_process_volume(&process.name, want);
                        process.applied_volume = want;
                        changed = true;
                    }
                }
                if changed {
                    self.store(Some(d));
                }
            }
            Some(d) => {
                // Debounce: hold the duck through short gaps (track changes,
                // seeking) and only restore after the delay.
                let delay = Duration::from_millis(settings.unduck_delay_ms);
                let waited_long_enough =
                    self.last_playing_at.is_none_or(|at| at.elapsed() >= delay);
                if waited_long_enough {
                    d.fade_back(RESTORE_FADE);
                    self.store(None);
                }
            }
        }
    }

    /// Saves the new ducked state; when ducking starts or stops, refreshes the
    /// tray and tells the UI (which no longer polls for it).
    fn store(&self, ducked: Option<DuckedState>) {
        let is_ducked = ducked.is_some();
        let state = self.app.state::<AppState>();
        let was_ducked = std::mem::replace(&mut *state.ducked.lock().unwrap(), ducked).is_some();
        if is_ducked != was_ducked {
            tray::update(&self.app);
            let _ = self.app.emit("ducked-changed", is_ducked);
        }
    }
}
