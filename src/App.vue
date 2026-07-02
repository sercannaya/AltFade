<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "vue-i18n";

interface AudioSession {
  name: string;
  icon: string | null;
}

interface Settings {
  target: string | null;
  duck_percent: number;
  active: boolean;
  lang: string;
}

const { t, locale } = useI18n();
const isDark = ref(true);
const sessions = ref<AudioSession[]>([]);
const selectedProcess = ref<string | null>(null);
const isActive = ref(false);
const autostartEnabled = ref(false);
const loading = ref(false);
const duckPercent = ref(20);

function toggleTheme() {
  isDark.value = !isDark.value;
  localStorage.setItem("theme", isDark.value ? "dark" : "light");
}

async function toggleLang() {
  locale.value = locale.value === "tr" ? "en" : "tr";
  await invoke("set_language", { lang: locale.value }).catch(console.error);
}

async function refreshSessions() {
  loading.value = true;
  try {
    sessions.value = await invoke<AudioSession[]>("get_audio_sessions");
  } catch (e) {
    console.error(e);
  } finally {
    loading.value = false;
  }
}

async function selectProcess(name: string) {
  selectedProcess.value = name;
  await invoke("set_target_process", { name }).catch(console.error);
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

async function toggleAutostart() {
  const next = !autostartEnabled.value;
  try {
    await invoke("set_autostart_enabled", { enabled: next });
    autostartEnabled.value = next;
  } catch (e) {
    console.error(e);
  }
}

onMounted(async () => {
  isDark.value = localStorage.getItem("theme") !== "light";
  try {
    const settings = await invoke<Settings>("get_settings");
    selectedProcess.value = settings.target;
    duckPercent.value = settings.duck_percent;
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
  await refreshSessions();
});
</script>

<template>
  <div class="app" :class="{ light: !isDark }">

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
        <button class="icon-btn" @click="toggleLang" title="TR / EN">
          {{ locale === 'tr' ? 'EN' : 'TR' }}
        </button>
        <button class="icon-btn" @click="toggleTheme" :title="isDark ? 'Light mode' : 'Dark mode'">
          {{ isDark ? '☀' : '🌙' }}
        </button>
      </div>
    </header>

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
          :class="{ selected: selectedProcess === s.name }"
          @click="selectProcess(s.name)"
        >
          <img v-if="s.icon" :src="s.icon" class="exe-icon" alt="" />
          <span v-else class="exe-icon-fallback">🎮</span>
          <span class="exe-name">{{ s.name }}</span>
          <span v-if="selectedProcess === s.name" class="badge">{{ t('badge') }}</span>
        </button>
      </div>
    </section>

    <!-- Target + Volume -->
    <section class="card">
      <div class="info-row">
        <span class="label">{{ t('target') }}</span>
        <span class="value" :class="{ dim: !selectedProcess }">
          {{ selectedProcess ?? t('none') }}
        </span>
      </div>
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
    </section>

    <!-- Main Toggle -->
    <button
      class="toggle-btn"
      :class="{ active: isActive }"
      :disabled="!selectedProcess"
      @click="toggleDucking"
    >
      <span class="dot" :class="{ on: isActive }" />
      {{ isActive ? t('stop') : t('start') }}
    </button>

    <!-- Footer -->
    <footer class="footer">
      <label class="autostart-row">
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
}
</style>

<style scoped>
.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
  padding: 18px;
  gap: 12px;
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
  padding: 12px 14px;
  flex-shrink: 0;
}
.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 10px;
}
.card-title {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--text-dim);
}

/* Session list */
.session-list {
  display: flex;
  flex-direction: column;
  gap: 5px;
  max-height: 188px;
  overflow-y: auto;
}
.session-list::-webkit-scrollbar { width: 3px; }
.session-list::-webkit-scrollbar-thumb { background: var(--border); border-radius: 3px; }

.empty { color: var(--text-dim); font-size: 13px; text-align: center; padding: 16px 0; }

.session-item {
  display: flex;
  align-items: center;
  gap: 9px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 8px 11px;
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
.badge { font-size: 10px; background: var(--accent); color: #fff; border-radius: 4px; padding: 2px 7px; flex-shrink: 0; }

/* Info + slider */
.info-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 10px;
}
.slider-row {
  display: flex;
  align-items: center;
  gap: 10px;
}
.label { font-size: 12px; color: var(--text-dim); white-space: nowrap; }
.value { font-family: monospace; font-size: 13px; color: var(--accent); }
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
  min-width: 34px;
  text-align: right;
}

/* Toggle button */
.toggle-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  width: 100%;
  padding: 14px;
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
  padding-top: 10px;
  flex-shrink: 0;
}
.autostart-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 13px;
  color: var(--text-dim);
  cursor: pointer;
}

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
</style>
