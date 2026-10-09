<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { isIgnored, isTrigger, state, toggleIgnore, toggleTrigger } from "../store";
import { displaySources, sourceLabel } from "../lib/sources";

const { t } = useI18n();

const chips = computed(() =>
  displaySources(state.mediaSources, state.triggerApps, state.ignoredSources)
);
</script>

<template>
  <section class="card">
    <div class="card-header">
      <span class="card-title">{{ t('triggers') }}</span>
    </div>
    <p class="hint">{{ t('triggersHint') }}</p>
    <div class="trigger-list">
      <div v-if="chips.length === 0" class="empty small">{{ t('noMedia') }}</div>
      <div
        v-for="m in chips"
        :key="m.id"
        class="trigger-item"
        :class="{ selected: isTrigger(m.id) && !isIgnored(m.id), offline: !m.live, ignored: isIgnored(m.id) }"
        :title="m.id"
      >
        <button
          class="trigger-main"
          :disabled="isIgnored(m.id)"
          @click="toggleTrigger(m.id)"
        >
          <span v-if="m.playing" class="trigger-play">♪</span>{{ sourceLabel(m.id) }}
        </button>
        <button
          class="trigger-ignore"
          :title="isIgnored(m.id) ? t('unignore') : t('ignore')"
          @click="toggleIgnore(m.id)"
        >{{ isIgnored(m.id) ? '↺' : '✕' }}</button>
      </div>
    </div>
  </section>
</template>

<style scoped>
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
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 999px;
  max-width: 100%;
  overflow: hidden;
  transition: background 0.12s, border-color 0.12s, color 0.12s, opacity 0.12s;
}
.trigger-item:hover { border-color: var(--accent); }
.trigger-item.selected { border-color: var(--accent); background: color-mix(in srgb, var(--accent) 14%, transparent); }
.trigger-item.offline { border-style: dashed; }
.trigger-item.ignored { opacity: 0.5; border-style: dashed; }
.trigger-main {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  background: transparent;
  border: none;
  padding: 4px 4px 4px 11px;
  cursor: pointer;
  color: var(--text-dim);
  font-size: 12px;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.trigger-main:hover:not(:disabled) { color: var(--text); }
.trigger-item.selected .trigger-main { color: var(--text); }
.trigger-item.ignored .trigger-main { text-decoration: line-through; cursor: default; }
.trigger-ignore {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  padding: 4px 9px 4px 6px;
  cursor: pointer;
  color: var(--text-dim);
  font-size: 11px;
  line-height: 1;
}
.trigger-ignore:hover { color: var(--text); }
.trigger-play { color: var(--success); font-size: 11px; }
</style>
