<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { fetchStatus, state } from "../store";
import { usePoll } from "../composables/usePoll";

const { t } = useI18n();

// Ducked state is also pushed by the backend; this poll is for now-playing.
usePoll(fetchStatus, 1000);

const statusKey = computed(() =>
  !state.active ? "statusOff" : state.ducked ? "statusDucked" : "statusArmed"
);

const nowPlayingText = computed(() => {
  const np = state.nowPlaying;
  if (!np || !np.playing) return null;
  const track = [np.title, np.artist].filter(Boolean).join(" — ");
  return track || np.source || null;
});
</script>

<template>
  <section class="card status-card">
    <div class="status-row">
      <span
        class="status-dot"
        :class="{ armed: state.active && !state.ducked, ducking: state.active && state.ducked }"
      />
      <span class="status-text">{{ t(statusKey) }}</span>
      <kbd v-if="state.hotkey" class="kbd">Ctrl+Alt+D</kbd>
    </div>
    <div v-if="nowPlayingText" class="now-playing">♪ {{ nowPlayingText }}</div>
  </section>
</template>

<style scoped>
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
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.45; }
}
</style>
