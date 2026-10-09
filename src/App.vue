<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import { useI18n } from "vue-i18n";
import AppHeader from "./components/AppHeader.vue";
import SessionList from "./components/SessionList.vue";
import SettingsCard from "./components/SettingsCard.vue";
import StatusCard from "./components/StatusCard.vue";
import TriggerList from "./components/TriggerList.vue";
import UpdateBanner from "./components/UpdateBanner.vue";
import { useAppearance } from "./composables/useAppearance";
import { init, refreshSessions, state, toggleAutostart, toggleDucking } from "./store";

const { t } = useI18n();
const { isDark, micaActive, accentStyle, initAppearance } = useAppearance();

// The window mostly lives in the tray; reopening it should show the apps
// playing now, not the ones from when it was last open.
function onVisibilityChange() {
  if (!document.hidden) refreshSessions();
}

onMounted(() => {
  initAppearance();
  init();
  document.addEventListener("visibilitychange", onVisibilityChange);
});
onUnmounted(() => document.removeEventListener("visibilitychange", onVisibilityChange));
</script>

<template>
  <div class="app" :class="{ light: !isDark, mica: micaActive }" :style="accentStyle">
    <AppHeader />
    <UpdateBanner />
    <StatusCard />
    <SessionList />
    <TriggerList />
    <SettingsCard />

    <button
      class="toggle-btn"
      :class="{ active: state.active }"
      :disabled="state.targets.length === 0 && !state.active"
      @click="toggleDucking"
    >
      <span class="dot" :class="{ on: state.active }" />
      {{ state.active ? t('stop') : t('start') }}
    </button>

    <footer class="footer">
      <label class="switch-row">
        <span>{{ t('autostart') }}</span>
        <button
          class="switch"
          :class="{ on: state.autostart }"
          @click="toggleAutostart"
          role="switch"
          :aria-checked="state.autostart"
        />
      </label>
    </footer>
  </div>
</template>

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

.toggle-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  width: 100%;
  padding: 13px;
  border-radius: var(--radius);
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

.footer {
  border-top: 1px solid var(--border);
  padding-top: 8px;
  flex-shrink: 0;
  margin-top: auto;
}
</style>
