import { writable } from "svelte/store";
import * as api from "./api.js";

/**
 * The live look. tokens.css is compiled in (Ewe Dark, the default accent) so
 * the app paints something before this runs and in a browser without Tauri.
 * `ewe-theme show` builds the same token map from THIS machine's ewe.conf —
 * the active scheme, accent, look presets, accessibility modes — and every
 * value lands as an inline custom property on <html>, which beats the
 * compiled file. Nothing here knows which scheme is active: the generator
 * already did the derivation, this only carries the result across.
 *
 * Call refreshTheme() after anything that can change the look (a scheme,
 * the accent, a preset, an accessibility mode, bar opacity). It also runs at
 * start-up and whenever the window regains focus, so a change made from the
 * shell or the command line reaches an open Settings window too.
 */
export const theme = writable(null);

let seq = 0;
export async function refreshTheme() {
  const mine = ++seq;
  let t;
  try {
    t = await api.themeTokens("ewe");
  } catch {
    return null; // no ewe-theme (dev browser, or ewe not deployed): keep tokens.css
  }
  if (mine !== seq || !t || !t.css_vars) return null;
  const root = document.documentElement;
  for (const [k, v] of Object.entries(t.css_vars)) root.style.setProperty(k, v);

  // Motion (README "Motion"): the four durations divided by the animation
  // speed; Off makes them 0. Reduce motion is already folded in by the
  // generator (base and slow become fast).
  const m = t.motion || {};
  const speed = Number(m.speed ?? 1);
  const dur = (ms) => (speed > 0 ? Math.round(Number(ms) / speed) : 0) + "ms";
  if (m.durFast != null) {
    root.style.setProperty("--dur-fast", dur(m.durFast));
    root.style.setProperty("--dur-base", dur(m.durBase));
    root.style.setProperty("--dur-slow", dur(m.durSlow));
    root.style.setProperty("--dur-dim", dur(m.durDim));
  }

  const variant = (t.input && t.input.variant) || (t.scheme && t.scheme.variant) || "dark";
  root.style.colorScheme = variant === "light" ? "light" : "dark";
  root.classList.toggle("dark", variant !== "light");
  root.dataset.variant = variant;
  theme.set(t);
  return t;
}

export function watchTheme() {
  refreshTheme();
  const onFocus = () => refreshTheme();
  window.addEventListener("focus", onFocus);
  return () => window.removeEventListener("focus", onFocus);
}
