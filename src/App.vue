<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

const sessions = ref<string[]>([]);
const selectedProcess = ref<string | null>(null);
const isActive = ref(false);
const autostartEnabled = ref(false);
const loading = ref(false);

async function refreshSessions() {
  loading.value = true;
  try {
    sessions.value = await invoke<string[]>("get_audio_sessions");
  } finally {
    loading.value = false;
  }
}

async function selectProcess(name: string) {
  selectedProcess.value = name;
  await invoke("set_target_process", { name });
}

async function toggleDucking() {
  isActive.value = await invoke<boolean>("toggle_ducking");
}

async function toggleAutostart() {
  const next = !autostartEnabled.value;
  await invoke("set_autostart_enabled", { enabled: next });
  autostartEnabled.value = next;
}

onMounted(async () => {
  autostartEnabled.value = await invoke<boolean>("get_autostart_enabled");
  await refreshSessions();
});
</script>

<template>
  <div class="app">
    <header class="header">
      <div class="logo-row">
        <span class="logo-icon">🔊</span>
        <div>
          <h1 class="title">AltFade</h1>
          <p class="subtitle">Oyun sesini otomatik kıs</p>
        </div>
      </div>
    </header>

    <section class="section">
      <div class="section-header">
        <span class="section-title">Aktif Ses Oturumları</span>
        <button class="btn-icon" :disabled="loading" @click="refreshSessions" title="Yenile">
          <span :class="{ spin: loading }">↻</span>
        </button>
      </div>

      <div class="session-list">
        <div v-if="sessions.length === 0" class="empty">
          {{ loading ? "Yükleniyor…" : "Ses çıkaran uygulama bulunamadı." }}
        </div>
        <button
          v-for="s in sessions"
          :key="s"
          class="session-item"
          :class="{ selected: selectedProcess === s }"
          @click="selectProcess(s)"
        >
          <span class="exe-name">{{ s }}</span>
          <span v-if="selectedProcess === s" class="badge">Hedef</span>
        </button>
      </div>
    </section>

    <section class="section">
      <div class="target-info">
        <span class="label">Seçili Hedef</span>
        <span class="value">{{ selectedProcess ?? "—" }}</span>
      </div>
    </section>

    <button
      class="toggle-btn"
      :class="{ active: isActive }"
      :disabled="!selectedProcess"
      @click="toggleDucking"
    >
      <span class="toggle-indicator" :class="{ on: isActive }" />
      {{ isActive ? "Servis Aktif — Durdur" : "Servisi Başlat" }}
    </button>

    <footer class="footer">
      <label class="autostart-row">
        <span>Windows başlangıcında otomatik çalıştır</span>
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

:root {
  font-family: 'Segoe UI', system-ui, sans-serif;
  font-size: 14px;
  background: #121218;
  color: #e2e2e8;
}

body { overflow: hidden; }
</style>

<style scoped>
.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
  padding: 20px;
  gap: 16px;
}

/* Header */
.header { flex-shrink: 0; }
.logo-row { display: flex; align-items: center; gap: 12px; }
.logo-icon { font-size: 28px; }
.title { font-size: 22px; font-weight: 700; color: #fff; line-height: 1.2; }
.subtitle { font-size: 12px; color: #888; margin-top: 2px; }

/* Section */
.section {
  background: #1c1c26;
  border: 1px solid #2a2a38;
  border-radius: 12px;
  padding: 14px;
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 10px;
}
.section-title { font-weight: 600; font-size: 13px; color: #aaa; text-transform: uppercase; letter-spacing: 0.05em; }

.btn-icon {
  background: none;
  border: 1px solid #2a2a38;
  border-radius: 6px;
  color: #aaa;
  width: 28px;
  height: 28px;
  font-size: 16px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: color 0.2s, border-color 0.2s;
}
.btn-icon:hover { color: #fff; border-color: #555; }
.btn-icon:disabled { opacity: 0.4; cursor: default; }

.spin { display: inline-block; animation: spin 0.8s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }

/* Session list */
.session-list { display: flex; flex-direction: column; gap: 6px; max-height: 220px; overflow-y: auto; }
.session-list::-webkit-scrollbar { width: 4px; }
.session-list::-webkit-scrollbar-thumb { background: #333; border-radius: 4px; }

.empty { color: #555; font-size: 13px; text-align: center; padding: 20px 0; }

.session-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: #111118;
  border: 1px solid #2a2a38;
  border-radius: 8px;
  padding: 10px 12px;
  cursor: pointer;
  text-align: left;
  color: #ccc;
  font-size: 13px;
  transition: background 0.15s, border-color 0.15s;
  width: 100%;
}
.session-item:hover { background: #1a1a28; border-color: #444; }
.session-item.selected { border-color: #6c63ff; background: #1a1830; color: #fff; }

.exe-name { font-family: 'Cascadia Code', 'Consolas', monospace; }
.badge { font-size: 11px; background: #6c63ff; color: #fff; border-radius: 4px; padding: 2px 7px; }

/* Target info */
.target-info { display: flex; align-items: center; justify-content: space-between; }
.label { font-size: 12px; color: #666; }
.value { font-family: monospace; font-size: 13px; color: #a78bfa; }

/* Toggle button */
.toggle-btn {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  width: 100%;
  padding: 14px;
  border-radius: 12px;
  border: none;
  font-size: 15px;
  font-weight: 600;
  cursor: pointer;
  background: #2a2a3a;
  color: #ccc;
  transition: background 0.2s, color 0.2s, transform 0.1s;
}
.toggle-btn:hover:not(:disabled) { background: #33334a; }
.toggle-btn:active:not(:disabled) { transform: scale(0.98); }
.toggle-btn.active { background: #4f46e5; color: #fff; }
.toggle-btn.active:hover { background: #5b52f0; }
.toggle-btn:disabled { opacity: 0.35; cursor: not-allowed; }

.toggle-indicator {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: #555;
  transition: background 0.2s;
}
.toggle-indicator.on { background: #a5f3a0; box-shadow: 0 0 6px #4ade80; }

/* Footer */
.footer {
  flex-shrink: 0;
  border-top: 1px solid #1e1e2a;
  padding-top: 12px;
}

.autostart-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 13px;
  color: #888;
  cursor: pointer;
}

.switch {
  position: relative;
  width: 40px;
  height: 22px;
  border-radius: 11px;
  border: none;
  background: #333;
  cursor: pointer;
  flex-shrink: 0;
  transition: background 0.2s;
}
.switch::after {
  content: '';
  position: absolute;
  top: 3px;
  left: 3px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: #666;
  transition: transform 0.2s, background 0.2s;
}
.switch.on { background: #4f46e5; }
.switch.on::after { transform: translateX(18px); background: #fff; }
</style>
