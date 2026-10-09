import { computed, ref } from "vue";
import { api } from "../api";

// Per-machine look & feel. Lives in localStorage, not the backend config,
// since nothing outside the UI depends on it.

const isDark = ref(true);
const accent = ref<string | null>(null);
const micaActive = ref(false);

export const ACCENT_PRESETS = ["#7c6fff", "#4f9cf9", "#2dd4a7", "#f472b6", "#f59e0b"];
export const DEFAULT_ACCENT = "#7c6fff";

function lighten(hex: string, amount: number): string {
  const n = parseInt(hex.slice(1), 16);
  const channel = (value: number) => Math.min(255, Math.round(value + (255 - value) * amount));
  const r = channel((n >> 16) & 0xff);
  const g = channel((n >> 8) & 0xff);
  const b = channel(n & 0xff);
  return `#${((r << 16) | (g << 8) | b).toString(16).padStart(6, "0")}`;
}

const accentStyle = computed(() =>
  accent.value ? { "--accent": accent.value, "--accent-hv": lighten(accent.value, 0.18) } : {}
);

function setAccent(color: string | null) {
  accent.value = color;
  if (color) localStorage.setItem("accent", color);
  else localStorage.removeItem("accent");
}

function toggleTheme() {
  isDark.value = !isDark.value;
  localStorage.setItem("theme", isDark.value ? "dark" : "light");
  // Keep the native window theme in sync so the Mica backdrop matches.
  api.setWindowTheme(isDark.value).catch(console.error);
}

function initAppearance() {
  const savedTheme = localStorage.getItem("theme");
  isDark.value = savedTheme
    ? savedTheme === "dark"
    : window.matchMedia("(prefers-color-scheme: dark)").matches;
  accent.value = localStorage.getItem("accent");
  api.setWindowTheme(isDark.value).catch(console.error);
  api
    .isMicaActive()
    .then((v) => (micaActive.value = v))
    .catch(console.error);
}

export function useAppearance() {
  return { isDark, accent, micaActive, accentStyle, setAccent, toggleTheme, initAppearance };
}
