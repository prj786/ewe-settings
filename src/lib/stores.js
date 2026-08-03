import { writable, derived } from "svelte/store";

export const prefs = writable({});
export const pane = writable("appearance");
export const version = writable("");
export const shellUp = writable(true);

// One look ("flock") — the accent the user picked is always the effective one.
export const effectiveAccent = derived(prefs, (p) => p.accent || "#0a84ff");

// Transient status shared by every pane: a green "Applied" flash and a red
// dismissable error banner — the same two signals the in-shell Settings had.
export const appliedMsg = writable("");
export const errorMsg = writable("");
let appliedTimer;
export function flashApplied(msg = "Applied") {
  appliedMsg.set(msg);
  clearTimeout(appliedTimer);
  appliedTimer = setTimeout(() => appliedMsg.set(""), 2200);
}

export const toasts = writable([]);
export function toast(message, type = "info", ms = 3200) {
  const id = Math.random().toString(36).slice(2);
  toasts.update((t) => [...t, { id, message: String(message), type }]);
  setTimeout(() => toasts.update((t) => t.filter((x) => x.id !== id)), ms);
}
export function dismissToast(id) {
  toasts.update((t) => t.filter((x) => x.id !== id));
}
