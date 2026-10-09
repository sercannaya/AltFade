<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { saveDuckVolume, saveUnduckDelay, state, toggleHotkey } from "../store";

const { t } = useI18n();

function formatSeconds(ms: number): string {
  return (ms / 1000).toFixed(2).replace(/\.?0+$/, "");
}
</script>

<template>
  <section class="card">
    <div class="slider-row">
      <span class="label">{{ t('duckVolume') }}</span>
      <input
        type="range"
        min="5"
        max="50"
        step="1"
        v-model.number="state.duckPercent"
        @change="saveDuckVolume"
        class="slider"
      />
      <span class="vol-badge">%{{ state.duckPercent }}</span>
    </div>
    <div class="slider-row">
      <span class="label">{{ t('unduckDelay') }}</span>
      <input
        type="range"
        min="0"
        max="5000"
        step="250"
        v-model.number="state.unduckDelay"
        @change="saveUnduckDelay"
        class="slider"
      />
      <span class="vol-badge">{{ formatSeconds(state.unduckDelay) }}s</span>
    </div>
    <label class="switch-row">
      <span class="label">{{ t('hotkey') }}</span>
      <button
        class="switch"
        :class="{ on: state.hotkey }"
        @click="toggleHotkey"
        role="switch"
        :aria-checked="state.hotkey"
      />
    </label>
  </section>
</template>

<style scoped>
.slider-row {
  display: flex;
  align-items: center;
  gap: 10px;
}
.slider-row + .slider-row, .slider-row + .switch-row { margin-top: 8px; }
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
</style>
