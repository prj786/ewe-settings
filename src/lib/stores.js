import { writable, derived } from "svelte/store";

export const prefs = writable({});
export const pane = writable("appearance");
export const version = writable("");
export const shellUp = writable(true);

// The accent the user picked is always the effective one. "" means they never
// picked: the default accent (--accent-default, from the token file or
// ewe-theme show) then stands instead of an invented colour.
export const effectiveAccent = derived(prefs, (p) => p.accent || "");

// The default accent as a hex, read from the tokens rather than repeated here.
export function accentDefault() {
  try {
    return getComputedStyle(document.documentElement).getPropertyValue("--accent-default").trim().toLowerCase();
  } catch {
    return "";
  }
}

// Everything `ewe-theme` derives its token set FROM, as one string.
//
// The token injection in App.svelte used to be keyed on the accent alone, so
// changing corner, density or stroke rewrote ewe.conf and moved the shell —
// and then stopped at this app's boundary, because the accent had not changed
// and the guard returned early. The radii and row heights only arrived on the
// next launch, which is exactly what "shape and density need a restart" was.
// These four keys already ride along in user-theme.json, so watching them
// costs nothing extra.
export const themeKey = derived(prefs, (p) =>
  [
    p.accent || "",
    p.themeCorner || "",
    p.themeDensity || "",
    p.themeStroke || "",
    p.neutralTint === undefined ? "" : String(p.neutralTint)
  ].join("|")
);

// Transient status shared by every pane: a confirmation Toast and a
// dismissable error banner (an Inline alert) — the same two signals the
// in-shell Settings had.
export const errorMsg = writable("");

// The Toast (design/system/components/Toast): one at a time, a new one
// replaces the current one. 5 s, or 8 s when it carries an action; the
// component pauses the timer while it is hovered or focused. `message` may
// name the thing in **bold**. tone: "info" | "success" | "warning" | "danger".
export const currentToast = writable(null);
let toastSeq = 0;
export function toast(message, tone = "info", ms = 0, action = null) {
  const timeout = ms || (action ? 8000 : 5000);
  currentToast.set({ id: ++toastSeq, message: String(message), tone, action, timeout });
}
export function dismissToast(id) {
  currentToast.update((t) => (t && (id == null || t.id === id) ? null : t));
}
/** "Applied" and friends: a past-tense confirmation, no action. */
export function flashApplied(msg = "Applied") {
  toast(msg, "success");
}
