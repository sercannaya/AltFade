import type { MediaSource } from "../types";

// Always-offered trigger suggestions; matched by substring on the backend,
// so "Spotify" covers both Spotify.exe and the Store package.
export const KNOWN_SOURCES = ["Spotify"];

/**
 * Friendly chip label for a raw AppUserModelId. Store AUMIDs look like
 * "Publisher.AppName_hash!AppId"; the raw id stays the stored value.
 */
export function sourceLabel(id: string): string {
  if (id.toLowerCase().includes("spotify")) return "Spotify";
  const parts = id.split("!");
  let name = parts.length > 1 ? parts[parts.length - 1] : id;
  if (name === "App" || /^[0-9A-F]{8,}$/i.test(name)) {
    const pkgMatch = parts[0].match(/^[^.]+\.([^_]+)/);
    if (pkgMatch) name = pkgMatch[1];
  }
  return name.replace(/\.exe$/i, "");
}

/** Trigger entries match by case-insensitive substring, like the backend. */
export function matchesAny(id: string, needles: string[]): boolean {
  const lower = id.toLowerCase();
  return needles.some((n) => lower.includes(n.toLowerCase()));
}

export interface DisplaySource extends MediaSource {
  /** False for chips kept around after their app closed. */
  live: boolean;
}

/**
 * Live media sessions, plus saved triggers and ignored sources whose app is
 * closed, plus well-known suggestions, so chips don't vanish when an app exits.
 */
export function displaySources(
  live: MediaSource[],
  triggers: string[],
  ignored: string[]
): DisplaySource[] {
  const chips: DisplaySource[] = live.map((m) => ({ ...m, live: true }));
  const covered = (name: string) =>
    chips.some((c) => c.id.toLowerCase().includes(name.toLowerCase()));
  for (const saved of triggers) {
    if (!covered(saved)) chips.push({ id: saved, playing: false, live: false });
  }
  for (const ign of ignored) {
    if (!chips.some((c) => c.id.toLowerCase() === ign.toLowerCase()))
      chips.push({ id: ign, playing: false, live: false });
  }
  for (const known of KNOWN_SOURCES) {
    if (!covered(known) && !triggers.some((t) => t.toLowerCase() === known.toLowerCase())) {
      chips.push({ id: known, playing: false, live: false });
    }
  }
  return chips;
}
