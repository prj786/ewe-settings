// Write-through glue for the Animations pane: one store, one apply path.
// State lives in ~/.config/quickshell/animations.json; every change rewrites
// hypr/generated/animations.lua and live-applies the same statements, so what
// you feel is exactly what the next reload reads (SETTINGS-BACKEND contract).

import { get, writable } from "svelte/store";
import * as api from "./api.js";
import { prefs, flashApplied, errorMsg } from "./stores.js";
import {
  animationsLuaLines, animationsLuaText, defaultAnimState, mergeAnimState
} from "./hypr.js";

export const anim = writable(null); // null until loadAnim() has run

const JSON_PATH = "quickshell/animations.json";
const LUA_PATH = "hypr/generated/animations.lua";
let owned = false; // animations.json exists — the pane owns Hyprland animations

/** Seed the store from disk (defaults when the file is absent/corrupt). */
export async function loadAnim() {
  let st = null;
  try { st = JSON.parse(await api.readConfig(JSON_PATH)); } catch { /* first run */ }
  owned = !!st;
  anim.set(st ? mergeAnimState(st) : defaultAnimState());
}

/** Persist + live-apply a full next state. Nothing is written until the user
 *  actually changes something, so untouched installs keep hyprland.lua's own
 *  defaults (and the fallback multiplier block) untouched. */
export async function applyAnim(next, msg = "Applied") {
  anim.set(next);
  const mult = Number(get(prefs).animationSpeed ?? 1);
  try {
    await api.writeConfig(JSON_PATH, JSON.stringify(next, null, 2) + "\n");
    await api.writeConfig(LUA_PATH, animationsLuaText(next, mult));
    await api.runEvals(animationsLuaLines(next, mult));
    // the shell reads animations.json too (Theme.dur*/ease follow the pane) —
    // ask it to re-read so both halves change feel in the same breath
    await api.pokeShell();
    owned = true;
    flashApplied(msg);
  } catch (e) {
    errorMsg.set(String(e));
  }
}

/** Patch one leaf and apply. */
export async function patchLeaf(leaf, patch) {
  const st = get(anim);
  const next = { ...st, anims: { ...st.anims, [leaf]: { ...st.anims[leaf], ...patch } } };
  await applyAnim(next);
}

/** Called by overrides.setAnimationSpeed: once animations.lua exists it bakes
 *  the multiplier in, so a speed change must regenerate it too. Returns false
 *  when the pane has never written anything (legacy multiplier path only). */
export async function reapplyForSpeed() {
  if (!owned) {
    try { JSON.parse(await api.readConfig(JSON_PATH)); } catch { return false; }
    if (!get(anim)) await loadAnim();
    owned = true;
  }
  await applyAnim(get(anim));
  return true;
}
