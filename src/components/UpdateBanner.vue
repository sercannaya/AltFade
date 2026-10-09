<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { installUpdate, state } from "../store";

const { t } = useI18n();
</script>

<template>
  <section v-if="state.updateVersion" class="card update-banner">
    <span class="update-text">{{ t('updateAvailable') }} <b>v{{ state.updateVersion }}</b></span>
    <button class="update-btn" :disabled="state.updating" @click="installUpdate">
      {{ state.updating ? t('updating') : t('updateNow') }}
    </button>
  </section>
</template>

<style scoped>
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
</style>
