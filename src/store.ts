import { reactive } from "vue";
import { api, events } from "./api";
import { i18n } from "./i18n";
import { matchesAny } from "./lib/sources";
import type { AudioSession, MediaSource, NowPlaying } from "./types";

// App-wide UI state, shared by every component. The backend config is the
// source of truth: actions update this optimistically and persist through api.

export const state = reactive({
  sessions: [] as AudioSession[],
  peaks: {} as Record<string, number>,
  targets: [] as string[],
  mediaSources: [] as MediaSource[],
  triggerApps: [] as string[],
  ignoredSources: [] as string[],
  active: false,
  ducked: false,
  nowPlaying: null as NowPlaying | null,
  autostart: false,
  hotkey: true,
  loading: false,
  duckPercent: 20,
  unduckDelay: 1500,
  updateVersion: null as string | null,
  updating: false,
});

export async function init() {
  try {
    const settings = await api.getSettings();
    state.targets = settings.targets;
    state.duckPercent = settings.duck_percent;
    state.unduckDelay = settings.unduck_delay_ms;
    state.triggerApps = settings.trigger_apps;
    state.ignoredSources = settings.ignored_sources ?? [];
    state.hotkey = settings.hotkey_enabled;
    state.active = settings.active;
    if (settings.lang === "en" || settings.lang === "tr") i18n.global.locale.value = settings.lang;
  } catch (e) {
    console.error(e);
  }
  try {
    state.autostart = await api.getAutostartEnabled();
  } catch (e) {
    console.error(e);
  }
  await events.onActiveChanged((active) => (state.active = active));
  await events.onDuckedChanged((ducked) => (state.ducked = ducked));
  await events.onUpdateAvailable((version) => (state.updateVersion = version));
  await refreshSessions();
}

export async function refreshSessions() {
  state.loading = true;
  try {
    state.sessions = await api.getAudioSessions();
    state.mediaSources = await api.getMediaSources();
  } catch (e) {
    console.error(e);
  } finally {
    state.loading = false;
  }
}

export async function fetchStatus() {
  const status = await api.getStatus();
  state.ducked = status.ducked;
  state.nowPlaying = status.now_playing;
}

export async function fetchPeaks() {
  const list = await api.getSessionPeaks();
  state.peaks = Object.fromEntries(list.map((p) => [p.name, p.peak]));
}

export async function toggleLang() {
  const locale = i18n.global.locale;
  locale.value = locale.value === "tr" ? "en" : "tr";
  await api.setLanguage(locale.value).catch(console.error);
}

export function isTarget(name: string) {
  return state.targets.includes(name);
}

export async function toggleTarget(name: string) {
  state.targets = isTarget(name)
    ? state.targets.filter((n) => n !== name)
    : [...state.targets, name];
  await api.setTargets(state.targets).catch(console.error);
}

export function isTrigger(id: string) {
  return matchesAny(id, state.triggerApps);
}

export async function toggleTrigger(id: string) {
  const lower = id.toLowerCase();
  state.triggerApps = isTrigger(id)
    ? state.triggerApps.filter((t) => !lower.includes(t.toLowerCase()))
    : [...state.triggerApps, id];
  await api.setTriggerApps(state.triggerApps).catch(console.error);
}

export function isIgnored(id: string) {
  return state.ignoredSources.some((s) => s.toLowerCase() === id.toLowerCase());
}

// Full-id ignore list: a source marked here never triggers ducking, even with
// an empty trigger list, so a target game that also publishes a media session
// can't keep ducking itself.
export async function toggleIgnore(id: string) {
  const nowIgnored = !isIgnored(id);
  state.ignoredSources = nowIgnored
    ? [...state.ignoredSources, id]
    : state.ignoredSources.filter((s) => s.toLowerCase() !== id.toLowerCase());
  await api.setIgnoredSources(state.ignoredSources).catch(console.error);
  // An ignored source shouldn't also linger as an explicit trigger.
  if (nowIgnored && isTrigger(id)) await toggleTrigger(id);
}

export async function toggleDucking() {
  try {
    state.active = await api.toggleDucking();
  } catch (e) {
    console.error(e);
  }
}

export async function saveDuckVolume() {
  await api.setDuckVolume(state.duckPercent).catch(console.error);
}

export async function saveUnduckDelay() {
  await api.setUnduckDelay(state.unduckDelay).catch(console.error);
}

export async function toggleHotkey() {
  const next = !state.hotkey;
  try {
    await api.setHotkeyEnabled(next);
    state.hotkey = next;
  } catch (e) {
    console.error(e);
  }
}

export async function toggleAutostart() {
  const next = !state.autostart;
  try {
    await api.setAutostartEnabled(next);
    state.autostart = next;
  } catch (e) {
    console.error(e);
  }
}

export async function installUpdate() {
  state.updating = true;
  try {
    await api.installUpdate();
  } catch (e) {
    console.error(e);
    state.updating = false;
  }
}
