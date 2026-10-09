<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { ACCENT_PRESETS, DEFAULT_ACCENT, useAppearance } from "../composables/useAppearance";
import { toggleLang } from "../store";

const { t, locale } = useI18n();
const { isDark, accent, setAccent, toggleTheme } = useAppearance();
const showAccentMenu = ref(false);
</script>

<template>
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
            v-for="c in ACCENT_PRESETS"
            :key="c"
            class="swatch"
            :class="{ sel: accent === c }"
            :style="{ background: c }"
            @click="setAccent(c)"
          />
          <label class="swatch custom" :title="t('customColor')">
            <input
              type="color"
              :value="accent ?? DEFAULT_ACCENT"
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
</template>

<style scoped>
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

/* Accent picker popover */
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
</style>
