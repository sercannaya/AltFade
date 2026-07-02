<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useI18n } from "vue-i18n";

interface AudioSession {
  name: string;
  icon: string | null;
}

interface SessionPeak {
  name: string;
  peak: number;
}

interface MediaSource {
  id: string;
  playing: boolean;
}

interface NowPlaying {
  source: string;
  title: string;
  artist: string;
  playing: boolean;
}

interface Status {
  ducked: boolean;
  now_playing: NowPlaying | null;
}

interface Settings {
  targets: string[];
  duck_percent: number;
  unduck_delay_ms: number;
  trigger_apps: string[];
  hotkey_enabled: boolean;
  active: boolean;
  lang: string;
}

const { t, locale } = useI18n();
const isDark = ref(true);
const sessions = ref<AudioSession[]>([]);
const peaks = ref<Record<string, number>>({});
const targets = ref<string[]>([]);
const mediaSources = ref<MediaSource[]>([]);
const triggerApps = ref<string[]>([]);
const isActive = ref(false);
const isDucked = ref(false);
const nowPlaying = ref<NowPlaying | null>(null);
const autostartEnabled = ref(false);
const hotkeyEnabled = ref(true);
const loading = ref(false);
const duckPercent = ref(20);
const unduckDelay = ref(1500);
const micaActive = ref(false);
const accent = ref<string | null>(null);
const updateVersion = ref<string | null>(null);
const updating = ref(false);

let statusTimer: ReturnType<typeof setInterval> | undefined;
let peaksTimer: ReturnType<typeof setInterval> | undefined;
let unlistenActive: UnlistenFn | undefined;
let unlistenUpdate: UnlistenFn | undefined;

const accentPresets = ["#7c6fff", "#4f9cf9", "#2dd4a7", "#f472b6", "#f59e0b"];
const showAccentMenu = ref(false);

function lighten(hex: string, amount: number): string {
  const n = parseInt(hex.slice(1), 16);
  const channel = (value: number) => Math.min(255, Math.round(value + (255 - value) * amount));
  const r = channel((n >> 16) & 0xff);
  const g = channel((n >> 8) & 0xff);
  const b = channel(n & 0xff);
  return `#${((r << 16) | (g << 8) | b).toString(16).padStart(6, "0")}`;
}

const accentStyle = computed(() =>
  accent.value
    ? { "--accent": accent.value, "--accent-hv": lighten(accent.value, 0.18) }
    : {}
);

function setAccent(color: string | null) {
  accent.value = color;
  if (color) localStorage.setItem("accent", color);
  else localStorage.removeItem("accent");
}

const statusKey = computed(() =>
  !isActive.value ? "statusOff" : isDucked.value ? "statusDucked" : "statusArmed"
);

// Always-offered trigger suggestions; matched by substring on the backend,
// so "Spotify" covers both Spotify.exe and the Store package.
const knownSources = ["Spotify"];

// Friendly chip labels for raw AppUserModelIds. Store AUMIDs look like
// "Publisher.AppName_hash!AppId"; the raw id stays as the stored value.
function sourceLabel(id: string): string {
  if (id.toLowerCase().includes("spotify")) return "Spotify";
  const parts = id.split("!");
  let name = parts.length > 1 ? parts[parts.length - 1] : id;
  if (name === "App" || /^[0-9A-F]{8,}$/i.test(name)) {
    const pkgMatch = parts[0].match(/^[^.]+\.([^_]+)/);
    if (pkgMatch) name = pkgMatch[1];
  }
  return name.replace(/\.exe$/i, "");
}

interface DisplaySource {
  id: string;
  playing: boolean;
  live: boolean;
}

// Live media sessions, plus saved triggers whose app is closed, plus
// well-known suggestions — so chips don't vanish when an app exits.
const displaySources = computed<DisplaySource[]>(() => {
  const chips: DisplaySource[] = mediaSources.value.map((m) => ({ ...m, live: true }));
  const covered = (name: string) =>
    chips.some((c) => c.id.toLowerCase().includes(name.toLowerCase()));
  for (const saved of triggerApps.value) {
    if (!covered(saved)) chips.push({ id: saved, playing: false, live: false });
  }
  for (const known of knownSources) {
    if (
      !covered(known) &&
      !triggerApps.value.some((t) => t.toLowerCase() === known.toLowerCase())
    ) {
      chips.push({ id: known, playing: false, live: false });
    }
  }
  return chips;
});

const nowPlayingText = computed(() => {
  const np = nowPlaying.value;
  if (!np || !np.playing) return null;
  const track = [np.title, np.artist].filter(Boolean).join(" — ");
  return track || np.source || null;
});

function toggleTheme() {
  isDark.value = !isDark.value;
  localStorage.setItem("theme", isDark.value ? "dark" : "light");
  // Keep the native window theme in sync so the Mica backdrop matches.
  invoke("set_window_theme", { dark: isDark.value }).catch(console.error);
}

async function installUpdate() {
  updating.value = true;
  try {
    await invoke("install_update");
  } catch (e) {
    console.error(e);
    updating.value = false;
  }
}

async function toggleLang() {
  locale.value = locale.value === "tr" ? "en" : "tr";
  await invoke("set_language", { lang: locale.value }).catch(console.error);
}

async function refreshSessions() {
  loading.value = true;
  try {
    sessions.value = await invoke<AudioSession[]>("get_audio_sessions");
    mediaSources.value = await invoke<MediaSource[]>("get_media_sources");
  } catch (e) {
    console.error(e);
  } finally {
    loading.value = false;
  }
}

function isTarget(name: string) {
  return targets.value.includes(name);
}

async function toggleTarget(name: string) {
  targets.value = isTarget(name)
    ? targets.value.filter((n) => n !== name)
    : [...targets.value, name];
  await invoke("set_targets", { names: targets.value }).catch(console.error);
}

function isTrigger(id: string) {
  const lower = id.toLowerCase();
  return triggerApps.value.some((t) => lower.includes(t.toLowerCase()));
}

async function toggleTrigger(id: string) {
  const lower = id.toLowerCase();
  triggerApps.value = isTrigger(id)
    ? triggerApps.value.filter((t) => !lower.includes(t.toLowerCase()))
    : [...triggerApps.value, id];
  await invoke("set_trigger_apps", { apps: triggerApps.value }).catch(console.error);
}

async function toggleDucking() {
  try {
    isActive.value = await invoke<boolean>("toggle_ducking");
  } catch (e) {
    console.error(e);
  }
}

async function saveDuckVolume() {
  await invoke("set_duck_volume", { percent: duckPercent.value }).catch(console.error);
}

async function saveUnduckDelay() {
  await invoke("set_unduck_delay", { ms: unduckDelay.value }).catch(console.error);
}

async function toggleHotkey() {
  const next = !hotkeyEnabled.value;
  try {
    await invoke("set_hotkey_enabled", { enabled: next });
    hotkeyEnabled.value = next;
  } catch (e) {
    console.error(e);
  }
}

async function toggleAutostart() {
  const next = !autostartEnabled.value;
  try {
    await invoke("set_autostart_enabled", { enabled: next });
    autostartEnabled.value = next;
  } catch (e) {
    console.error(e);
  }
}

async function fetchStatus() {
  if (document.hidden) return;
  try {
    const status = await invoke<Status>("get_status");
    isDucked.value = status.ducked;
    nowPlaying.value = status.now_playing;
  } catch {
    /* polling; ignore transient errors */
  }
}

async function fetchPeaks() {
  if (document.hidden) return;
  try {
    const list = await invoke<SessionPeak[]>("get_session_peaks");
    peaks.value = Object.fromEntries(list.map((p) => [p.name, p.peak]));
  } catch {
    /* polling; ignore transient errors */
  }
}

onMounted(async () => {
  const savedTheme = localStorage.getItem("theme");
  isDark.value = savedTheme
    ? savedTheme === "dark"
    : window.matchMedia("(prefers-color-scheme: dark)").matches;
  accent.value = localStorage.getItem("accent");
  invoke("set_window_theme", { dark: isDark.value }).catch(console.error);
  invoke<boolean>("is_mica_active")
    .then((v) => (micaActive.value = v))
    .catch(console.error);

  try {
    const settings = await invoke<Settings>("get_settings");
    targets.value = settings.targets;
    duckPercent.value = settings.duck_percent;
    unduckDelay.value = settings.unduck_delay_ms;
    triggerApps.value = settings.trigger_apps;
    hotkeyEnabled.value = settings.hotkey_enabled;
    isActive.value = settings.active;
    if (settings.lang === "en" || settings.lang === "tr") locale.value = settings.lang;
  } catch (e) {
    console.error(e);
  }
  try {
    autostartEnabled.value = await invoke<boolean>("get_autostart_enabled");
  } catch (e) {
    console.error(e);
  }

  unlistenActive = await listen<boolean>("active-changed", (event) => {
    isActive.value = event.payload;
  });
  unlistenUpdate = await listen<string>("update-available", (event) => {
    updateVersion.value = event.payload;
  });

  await refreshSessions();
  await fetchStatus();
  statusTimer = setInterval(fetchStatus, 1000);
  peaksTimer = setInterval(fetchPeaks, 500);
});

onUnmounted(() => {
  if (statusTimer) clearInterval(statusTimer);
  if (peaksTimer) clearInterval(peaksTimer);
  unlistenActive?.();
  unlistenUpdate?.();
});
</script>

<template>
  <div class="app" :class="{ light: !isDark, mica: micaActive }" :style="accentStyle">

    <!-- Header -->
    <header class="header">
      <div class="brand">
        <img src="/alt-fade-720.png" class="brand-logo" alt="AltFade" />
        <div>
          <h1 class="brand-name">AltFade</h1>
          <p class="brand-sub">{{ t('subtitle') }}</p>
        </div>
      </div>
      <div class="header-actions">
        <div class="accent-wrap">
          <button class="icon-btn" :title="t('accent')" @click="showAccentMenu = !showAccentMenu">
            <span class="accent-dot" />
          </button>
          <div v-if="showAccentMenu" class="accent-menu">
            <button
              v-for="c in accentPresets"
              :key="c"
              class="swatch"
              :class="{ sel: accent === c }"
              :style="{ background: c }"
              @click="setAccent(c)"
            />
            <label class="swatch custom" :title="t('customColor')">
              <input
                type="color"
                :value="accent ?? '#7c6fff'"
                @input="setAccent(($event.target as HTMLInputElement).value)"
              />
            </label>
            <button
              v-if="accent"
              class="swatch reset"
              :title="t('resetColor')"
              @click="setAccent(null)"
            >↺</button>
          </div>
        </div>
        <button class="icon-btn" @click="toggleLang" title="TR / EN">
          {{ locale === 'tr' ? 'EN' : 'TR' }}
        </button>
        <button class="icon-btn" @click="toggleTheme" :title="isDark ? 'Light mode' : 'Dark mode'">
          {{ isDark ? '☀' : '🌙' }}
        </button>
      </div>
    </header>

    <!-- Click-away layer for the accent popover -->
    <div v-if="showAccentMenu" class="popover-overlay" @click="showAccentMenu = false" />

    <!-- Update banner -->
    <section v-if="updateVersion" class="card update-banner">
      <span class="update-text">{{ t('updateAvailable') }} <b>v{{ updateVersion }}</b></span>
      <button class="update-btn" :disabled="updating" @click="installUpdate">
        {{ updating ? t('updating') : t('updateNow') }}
      </button>
    </section>

    <!-- Status -->
    <section class="card status-card">
      <div class="status-row">
        <span
          class="status-dot"
          :class="{ armed: isActive && !isDucked, ducking: isActive && isDucked }"
        />
        <span class="status-text">{{ t(statusKey) }}</span>
        <kbd v-if="hotkeyEnabled" class="kbd">Ctrl+Alt+D</kbd>
      </div>
      <div v-if="nowPlayingText" class="now-playing">♪ {{ nowPlayingText }}</div>
    </section>

    <!-- Sessions -->
    <section class="card">
      <div class="card-header">
        <span class="card-title">{{ t('sessions') }}</span>
        <button class="icon-btn" :disabled="loading" @click="refreshSessions">
          <span :class="{ spin: loading }">↻</span>
        </button>
      </div>
      <div class="session-list">
        <div v-if="sessions.length === 0" class="empty">
          {{ loading ? t('loading') : t('noSessions') }}
        </div>
        <button
          v-for="s in sessions"
          :key="s.name"
          class="session-item"
          :class="{ selected: isTarget(s.name) }"
          @click="toggleTarget(s.name)"
        >
          <img v-if="s.icon" :src="s.icon" class="exe-icon" alt="" />
          <span v-else class="exe-icon-fallback">🎮</span>
          <span class="exe-name">{{ s.name }}</span>
          <span class="meter">
            <span
              class="meter-fill"
              :style="{ width: Math.min(100, Math.round((peaks[s.name] ?? 0) * 100)) + '%' }"
            />
          </span>
          <span v-if="isTarget(s.name)" class="badge">{{ t('badge') }}</span>
        </button>
      </div>
      <div class="info-row">
        <span class="label">{{ t('target') }}</span>
        <span class="value" :class="{ dim: targets.length === 0 }">
          {{ targets.length ? targets.join(', ') : t('none') }}
        </span>
      </div>
    </section>

    <!-- Trigger apps -->
    <section class="card">
      <div class="card-header">
        <span class="card-title">{{ t('triggers') }}</span>
      </div>
      <p class="hint">{{ t('triggersHint') }}</p>
      <div class="trigger-list">
        <div v-if="displaySources.length === 0" class="empty small">{{ t('noMedia') }}</div>
        <button
          v-for="m in displaySources"
          :key="m.id"
          class="trigger-item"
          :class="{ selected: isTrigger(m.id), offline: !m.live }"
          :title="m.id"
          @click="toggleTrigger(m.id)"
        >
          <span v-if="m.playing" class="trigger-play">♪</span>{{ sourceLabel(m.id) }}
        </button>
      </div>
    </section>

    <!-- Settings -->
    <section class="card">
      <div class="slider-row">
        <span class="label">{{ t('duckVolume') }}</span>
        <input
          type="range"
          min="5"
          max="50"
          step="1"
          v-model.number="duckPercent"
          @change="saveDuckVolume"
          class="slider"
        />
        <span class="vol-badge">%{{ duckPercent }}</span>
      </div>
      <div class="slider-row">
        <span class="label">{{ t('unduckDelay') }}</span>
        <input
          type="range"
          min="0"
          max="5000"
          step="250"
          v-model.number="unduckDelay"
          @change="saveUnduckDelay"
          class="slider"
        />
        <span class="vol-badge">{{ (unduckDelay / 1000).toFixed(2).replace(/\.?0+$/, '') }}s</span>
      </div>
      <label class="switch-row">
        <span class="label">{{ t('hotkey') }}</span>
        <button
          class="switch"
          :class="{ on: hotkeyEnabled }"
          @click="toggleHotkey"
          role="switch"
          :aria-checked="hotkeyEnabled"
        />
      </label>
    </section>

    <!-- Main Toggle -->
    <button
      class="toggle-btn"
      :class="{ active: isActive }"
      :disabled="targets.length === 0 && !isActive"
      @click="toggleDucking"
    >
      <span class="dot" :class="{ on: isActive }" />
      {{ isActive ? t('stop') : t('start') }}
    </button>

    <!-- Footer -->
    <footer class="footer">
      <label class="switch-row">
        <span>{{ t('autostart') }}</span>
        <button
          class="switch"
          :class="{ on: autostartEnabled }"
          @click="toggleAutostart"
          role="switch"
          :aria-checked="autostartEnabled"
        />
      </label>
    </footer>

  </div>
</template>

<style>
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
body { overflow: hidden; }

/* ── Design tokens ────────────────────────────────── */
:root {
  --bg:        #0d0d14;
  --surface:   #161622;
  --border:    #252535;
  --text:      #e2e2ee;
  --text-dim:  #666680;
  --accent:    #7c6fff;
  --accent-hv: #9188ff;
  --success:   #4ade80;
  --warn:      #ffa542;
  --radius:    12px;
  font-family: 'Segoe UI', system-ui, sans-serif;
  font-size: 14px;
}

.app.light {
  --bg:       #f2f2f8;
  --surface:  #ffffff;
  --border:   #dddde8;
  --text:     #18182a;
  --text-dim: #8888a0;
  --accent:   #5b52d0;
  --accent-hv:#4a42c0;
  --success:  #22c55e;
  --warn:     #e8891f;
}

/* Mica: let the system backdrop show through (Windows 11 only; the flag is
   set at runtime, older systems keep the solid background above). */
.app.mica {
  background: color-mix(in srgb, var(--bg) 60%, transparent);
}
.app.mica .card {
  background: color-mix(in srgb, var(--surface) 72%, transparent);
}
</style>

<style scoped>
.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
  padding: 16px;
  gap: 10px;
  background: var(--bg);
  color: var(--text);
  transition: background 0.2s, color 0.2s;
}

/* Header */
.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-shrink: 0;
}
.brand { display: flex; align-items: center; gap: 10px; }
.brand-logo {
  width: 42px;
  height: 42px;
  border-radius: 10px;
  object-fit: cover;
}
.brand-name { font-size: 20px; font-weight: 700; color: var(--text); line-height: 1.2; }
.brand-sub  { font-size: 11px; color: var(--text-dim); margin-top: 1px; }

.header-actions { display: flex; gap: 6px; }

.icon-btn {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  color: var(--text-dim);
  min-width: 32px;
  height: 32px;
  padding: 0 8px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: color 0.15s, border-color 0.15s;
}
.icon-btn:hover { color: var(--text); border-color: var(--accent); }
.icon-btn:disabled { opacity: 0.4; cursor: default; }

/* Card */
.card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 10px 14px;
  flex-shrink: 0;
}
.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}
.card-title {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--text-dim);
}

/* Update banner */
.update-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  border-color: var(--accent);
}
.update-text { font-size: 12px; color: var(--text); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.update-btn {
  background: var(--accent);
  border: none;
  border-radius: 8px;
  color: #fff;
  font-size: 12px;
  font-weight: 600;
  padding: 6px 14px;
  cursor: pointer;
  flex-shrink: 0;
  transition: background 0.15s;
}
.update-btn:hover:not(:disabled) { background: var(--accent-hv); }
.update-btn:disabled { opacity: 0.6; cursor: default; }

/* Status */
.status-card { padding: 10px 14px; }
.status-row { display: flex; align-items: center; gap: 9px; }
.status-dot {
  width: 10px; height: 10px;
  border-radius: 50%;
  background: var(--text-dim);
  flex-shrink: 0;
  transition: background 0.2s, box-shadow 0.2s;
}
.status-dot.armed  { background: var(--success); box-shadow: 0 0 7px var(--success); }
.status-dot.ducking { background: var(--warn); box-shadow: 0 0 7px var(--warn); animation: pulse 1.4s ease-in-out infinite; }
.status-text { font-size: 13px; font-weight: 600; flex: 1; }
.kbd {
  font-family: 'Cascadia Code', 'Consolas', monospace;
  font-size: 10px;
  color: var(--text-dim);
  border: 1px solid var(--border);
  border-radius: 5px;
  padding: 2px 6px;
  flex-shrink: 0;
}
.now-playing {
  margin-top: 6px;
  font-size: 12px;
  color: var(--accent);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* Session list */
.session-list {
  display: flex;
  flex-direction: column;
  gap: 5px;
  max-height: 150px;
  overflow-y: auto;
}
.session-list::-webkit-scrollbar { width: 3px; }
.session-list::-webkit-scrollbar-thumb { background: var(--border); border-radius: 3px; }

.empty { color: var(--text-dim); font-size: 13px; text-align: center; padding: 14px 0; }
.empty.small { font-size: 12px; padding: 8px 0; }

.session-item {
  display: flex;
  align-items: center;
  gap: 9px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 7px 11px;
  cursor: pointer;
  color: var(--text-dim);
  font-size: 13px;
  text-align: left;
  width: 100%;
  transition: background 0.12s, border-color 0.12s, color 0.12s;
}
.session-item:hover  { border-color: var(--accent); color: var(--text); }
.session-item.selected { border-color: var(--accent); background: color-mix(in srgb, var(--accent) 12%, transparent); color: var(--text); }

.exe-icon { width: 20px; height: 20px; object-fit: contain; flex-shrink: 0; }
.exe-icon-fallback { font-size: 17px; flex-shrink: 0; }
.exe-name { flex: 1; font-family: 'Cascadia Code', 'Consolas', monospace; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

.meter {
  width: 44px;
  height: 4px;
  border-radius: 2px;
  background: var(--border);
  overflow: hidden;
  flex-shrink: 0;
}
.meter-fill {
  display: block;
  height: 100%;
  background: var(--success);
  border-radius: 2px;
  transition: width 0.25s linear;
}

.badge { font-size: 10px; background: var(--accent); color: #fff; border-radius: 4px; padding: 2px 7px; flex-shrink: 0; }

/* Trigger list */
.hint { font-size: 11px; color: var(--text-dim); margin-bottom: 8px; }
.trigger-list {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  max-height: 64px;
  overflow-y: auto;
}
.trigger-list::-webkit-scrollbar { width: 3px; }
.trigger-list::-webkit-scrollbar-thumb { background: var(--border); border-radius: 3px; }
.trigger-item {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 999px;
  padding: 4px 11px;
  cursor: pointer;
  color: var(--text-dim);
  font-size: 12px;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  transition: background 0.12s, border-color 0.12s, color 0.12s;
}
.trigger-item:hover { border-color: var(--accent); color: var(--text); }
.trigger-item.selected { border-color: var(--accent); background: color-mix(in srgb, var(--accent) 14%, transparent); color: var(--text); }
.trigger-item.offline { border-style: dashed; }
.trigger-play { color: var(--success); font-size: 11px; }

/* Info + sliders */
.info-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin-top: 8px;
}
.slider-row {
  display: flex;
  align-items: center;
  gap: 10px;
}
.slider-row + .slider-row, .slider-row + .switch-row { margin-top: 8px; }
.label { font-size: 12px; color: var(--text-dim); white-space: nowrap; }
.value {
  font-family: monospace;
  font-size: 12px;
  color: var(--accent);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.value.dim { color: var(--text-dim); }

.slider {
  flex: 1;
  accent-color: var(--accent);
  height: 4px;
  cursor: pointer;
}
.vol-badge {
  font-size: 12px;
  font-weight: 700;
  color: var(--accent);
  min-width: 40px;
  text-align: right;
}

/* Toggle button */
.toggle-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  width: 100%;
  padding: 13px;
  border-radius: var(--radius);
  border: none;
  font-size: 15px;
  font-weight: 600;
  cursor: pointer;
  background: var(--surface);
  border: 1px solid var(--border);
  color: var(--text-dim);
  transition: background 0.2s, color 0.2s, border-color 0.2s, transform 0.1s;
  flex-shrink: 0;
}
.toggle-btn:hover:not(:disabled) { border-color: var(--accent); color: var(--text); }
.toggle-btn:active:not(:disabled) { transform: scale(0.98); }
.toggle-btn.active { background: var(--accent); border-color: var(--accent); color: #fff; }
.toggle-btn.active:hover { background: var(--accent-hv); }
.toggle-btn:disabled { opacity: 0.3; cursor: not-allowed; }

.dot {
  width: 9px; height: 9px;
  border-radius: 50%;
  background: var(--text-dim);
  transition: background 0.2s, box-shadow 0.2s;
  flex-shrink: 0;
}
.dot.on { background: #a5f3a0; box-shadow: 0 0 7px #4ade80; }

/* Footer */
.footer {
  border-top: 1px solid var(--border);
  padding-top: 8px;
  flex-shrink: 0;
  margin-top: auto;
}
.switch-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 13px;
  color: var(--text-dim);
  cursor: pointer;
}

/* Accent picker (header popover) */
.accent-wrap { position: relative; }
.accent-dot {
  width: 14px; height: 14px;
  border-radius: 50%;
  background: var(--accent);
  display: inline-block;
}
.popover-overlay {
  position: fixed;
  inset: 0;
  z-index: 10;
}
.accent-menu {
  position: absolute;
  top: 38px;
  right: 0;
  z-index: 20;
  display: flex;
  align-items: center;
  gap: 7px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 10px 12px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
}
.swatch {
  width: 20px; height: 20px;
  border-radius: 50%;
  border: 2px solid transparent;
  cursor: pointer;
  padding: 0;
  flex-shrink: 0;
  transition: transform 0.12s, border-color 0.12s;
}
.swatch:hover { transform: scale(1.15); }
.swatch.sel { border-color: var(--text); }
.swatch.custom {
  position: relative;
  overflow: hidden;
  background: conic-gradient(#f87171, #fbbf24, #4ade80, #60a5fa, #c084fc, #f87171);
}
.swatch.custom input {
  position: absolute;
  inset: -4px;
  opacity: 0;
  cursor: pointer;
}
.swatch.reset {
  background: var(--surface);
  border: 1px solid var(--border);
  color: var(--text-dim);
  font-size: 11px;
  display: flex;
  align-items: center;
  justify-content: center;
}
.swatch.reset:hover { color: var(--text); border-color: var(--accent); }

.switch {
  position: relative;
  width: 40px; height: 22px;
  border-radius: 11px;
  border: none;
  background: var(--border);
  cursor: pointer;
  flex-shrink: 0;
  transition: background 0.2s;
}
.switch::after {
  content: '';
  position: absolute;
  top: 3px; left: 3px;
  width: 16px; height: 16px;
  border-radius: 50%;
  background: var(--text-dim);
  transition: transform 0.2s, background 0.2s;
}
.switch.on { background: var(--accent); }
.switch.on::after { transform: translateX(18px); background: #fff; }

/* Animations */
.spin { display: inline-block; animation: spin 0.7s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.45; }
}
</style>
