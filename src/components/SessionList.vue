<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { fetchPeaks, isTarget, refreshSessions, state, toggleTarget } from "../store";
import { usePoll } from "../composables/usePoll";

const { t } = useI18n();

usePoll(fetchPeaks, 500);

/** Peak level as a 0..1 scale factor, rounded so tiny jitter doesn't restyle. */
function level(name: string): number {
  return Math.round(Math.min(1, state.peaks[name] ?? 0) * 100) / 100;
}
</script>

<template>
  <section class="card">
    <div class="card-header">
      <span class="card-title">{{ t('sessions') }}</span>
      <button class="icon-btn" :disabled="state.loading" @click="refreshSessions">
        <span :class="{ spin: state.loading }">↻</span>
      </button>
    </div>
    <div class="session-list">
      <div v-if="state.sessions.length === 0" class="empty">
        {{ state.loading ? t('loading') : t('noSessions') }}
      </div>
      <button
        v-for="s in state.sessions"
        :key="s.name"
        class="session-item"
        :class="{ selected: isTarget(s.name) }"
        @click="toggleTarget(s.name)"
      >
        <img v-if="s.icon" :src="s.icon" class="exe-icon" alt="" />
        <span v-else class="exe-icon-fallback">🎮</span>
        <span class="exe-name">{{ s.name }}</span>
        <span class="meter">
          <span class="meter-fill" :style="{ transform: `scaleX(${level(s.name)})` }" />
        </span>
        <span v-if="isTarget(s.name)" class="badge">{{ t('badge') }}</span>
      </button>
    </div>
    <div class="info-row">
      <span class="label">{{ t('target') }}</span>
      <span class="value" :class="{ dim: state.targets.length === 0 }">
        {{ state.targets.length ? state.targets.join(', ') : t('none') }}
      </span>
    </div>
  </section>
</template>

<style scoped>
.session-list {
  display: flex;
  flex-direction: column;
  gap: 5px;
  max-height: 150px;
  overflow-y: auto;
}
.session-list::-webkit-scrollbar { width: 3px; }
.session-list::-webkit-scrollbar-thumb { background: var(--border); border-radius: 3px; }

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
/* Scaled with transform and deliberately not transitioned: the meters update
   twice a second for as long as anything makes sound, and any transition here
   keeps the compositor producing frames at the display refresh rate
   (see docs/performance.md). */
.meter-fill {
  display: block;
  width: 100%;
  height: 100%;
  background: var(--success);
  border-radius: 2px;
  transform-origin: left center;
}

.badge { font-size: 10px; background: var(--accent); color: #fff; border-radius: 4px; padding: 2px 7px; flex-shrink: 0; }

.info-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin-top: 8px;
}
.value {
  font-family: monospace;
  font-size: 12px;
  color: var(--accent);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.value.dim { color: var(--text-dim); }
</style>
