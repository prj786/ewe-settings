// The write-through glue shared by the Appearance and Layout panes: both feed
// the SAME generated/user.lua (gaps, border, rounding, tinted border, window
// transparency, animation overrides, tiling rule), so its regeneration lives
// here rather than in either pane.

import { get, writable } from "svelte/store";
import * as api from "./api.js";
import { userLuaText, animLuaLines, transparencyLua, hex6 } from "./hypr.js";
import { prefs, effectiveAccent, flashApplied, errorMsg } from "./stores.js";

export const layout = writable({ gapsIn: 6, gapsOut: 14, borderSize: 1, rounding: 12 });

const firstInt = (block) => {
  const m = String(block).match(/-?\d+/);
  return m ? parseInt(m[0]) : undefined;
};

/** Seed the layout store from the live compositor state. */
export async function loadLayout() {
  try {
    const text = await api.hyprctl([
      "--batch",
      "getoption general:gaps_in; getoption general:gaps_out; getoption general:border_size; getoption decoration:rounding"
    ]);
    const blocks = text.split(/\n\s*\n/).filter((b) => b.trim() !== "");
    const [gi, go, bs, rd] = blocks.map(firstInt);
    layout.update((l) => ({
      gapsIn: gi ?? l.gapsIn,
      gapsOut: go ?? l.gapsOut,
      borderSize: bs ?? l.borderSize,
      rounding: rd ?? l.rounding
    }));
  } catch (e) {
    console.error(e);
  }
}

function overridesInput() {
  const p = get(prefs);
  const l = get(layout);
  return {
    gapsIn: l.gapsIn,
    gapsOut: l.gapsOut,
    borderSize: l.borderSize,
    rounding: l.rounding,
    tintBorders: !!p.tintBorders,
    accent: get(effectiveAccent),
    windowTransparency: p.windowTransparency !== false,
    animationSpeed: p.animationSpeed ?? 1,
    tilingEnabled: p.tilingEnabled !== false
  };
}

/** Regenerate generated/user.lua from the current prefs + layout state. */
export async function writeUserLua() {
  await api.writeConfig("hypr/generated/user.lua", userLuaText(overridesInput()));
}

/** Persist a prefs patch (user-theme.json; the backend pokes the shell). */
export async function setPrefs(patch) {
  prefs.set(await api.writePrefs(patch));
}

async function evals(stmts) {
  try {
    await api.runEvals(stmts);
    flashApplied();
  } catch (e) {
    errorMsg.set(String(e));
  }
}

/** Gaps / border / rounding: live apply + persist. */
export async function applyGaps() {
  const o = overridesInput();
  await evals([
    `hl.config({ general = { gaps_in = ${o.gapsIn}, gaps_out = ${o.gapsOut}, border_size = ${o.borderSize} } })`,
    `hl.config({ decoration = { rounding = ${o.rounding} } })`
  ]);
  await writeUserLua();
}

export async function setAnimationSpeed(m) {
  await setPrefs({ animationSpeed: Number(m) });
  await evals(animLuaLines(Number(m)));
  await writeUserLua();
}

export async function setTransparency(on) {
  await setPrefs({ windowTransparency: on });
  await evals([transparencyLua(on)]);
  await writeUserLua();
}

/** A window rule cannot be withdrawn at runtime, so both directions reload. */
export async function setTiling(on) {
  await setPrefs({ tilingEnabled: on });
  await writeUserLua();
  await api.reloadHyprland();
  flashApplied(on ? "Tiling on" : "New windows will float");
}

export async function applyBorder() {
  const o = overridesInput();
  if (o.tintBorders)
    await evals([
      `hl.config({ general = { col = { active_border = "rgba(${hex6(o.accent)}ff)" } } })`
    ]);
  await writeUserLua();
}

/** Accent: prefs + Hyprland border tint + GTK/Qt via colorscheme.sh. */
export async function setAccent(hexColor) {
  await setPrefs({ accent: hexColor });
  await applyBorder();
  const p = get(prefs);
  await api.applyColorscheme(p.colorScheme || "dark", get(effectiveAccent));
}

export async function setColorScheme(mode) {
  await setPrefs({ colorScheme: mode });
  await api.applyColorscheme(mode, get(effectiveAccent));
  flashApplied("Applied to GTK/Qt apps");
}

