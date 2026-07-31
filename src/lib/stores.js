import { writable, derived } from "svelte/store";

export const prefs = writable({});
export const pane = writable("appearance");
export const version = writable("");
export const shellUp = writable(true);

// Ambiance forces Ubuntu orange as the effective accent, so the accent picker
// must show as inert there rather than pretending it still applies.
export const themeName = derived(prefs, (p) => p.themeName || "graphite");
export const accentApplies = derived(themeName, (t) => t !== "ambiance");

export const toasts = writable([]);
export function toast(message, type = "info", ms = 3200) {
  const id = Math.random().toString(36).slice(2);
  toasts.update((t) => [...t, { id, message: String(message), type }]);
  setTimeout(() => toasts.update((t) => t.filter((x) => x.id !== id)), ms);
}
export function dismissToast(id) {
  toasts.update((t) => t.filter((x) => x.id !== id));
}
