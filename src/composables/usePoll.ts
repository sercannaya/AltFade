import { onMounted, onUnmounted } from "vue";

/**
 * Runs `task` every `intervalMs` while the page is visible.
 *
 * The backend marks the webview hidden whenever the window is hidden or
 * minimized (src-tauri/src/window.rs), so `document.hidden` is reliable and a
 * tray-resident app costs nothing here. Ticks are chained rather than using
 * setInterval, so a slow backend call can never stack up overlapping requests.
 */
export function usePoll(task: () => Promise<void>, intervalMs: number) {
  let timer: ReturnType<typeof setTimeout> | undefined;
  let running = false;
  // Bumped on every stop so a tick still awaiting `task` can't reschedule.
  let generation = 0;

  async function tick(gen: number) {
    if (gen !== generation) return;
    try {
      await task();
    } catch {
      /* polling; ignore transient errors */
    }
    if (gen === generation) timer = setTimeout(() => tick(gen), intervalMs);
  }

  function start() {
    if (running) return;
    running = true;
    tick(++generation);
  }

  function stop() {
    running = false;
    generation++;
    clearTimeout(timer);
  }

  function onVisibilityChange() {
    if (document.hidden) stop();
    else start();
  }

  onMounted(() => {
    document.addEventListener("visibilitychange", onVisibilityChange);
    if (!document.hidden) start();
  });
  onUnmounted(() => {
    document.removeEventListener("visibilitychange", onVisibilityChange);
    stop();
  });
}
