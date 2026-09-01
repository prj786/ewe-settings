// The write-through glue shared by the Appearance and Layout panes: both feed
// the SAME generated/user.lua (gaps, border, rounding, tinted border, window
// transparency, animation overrides, tiling rule), so its regeneration lives
// here rather than in either pane.

import { get, writable } from "svelte/store";
import * as api from "./api.js";
import { animLuaLines, transparencyLua, hex6 } from "./hypr.js";
import { prefs, effectiveAccent, flashApplied, errorMsg } from "./stores.js";
import { reapplyForSpeed } from "./animations.js";

export const layout = writable({ gapsIn: 6, gapsOut: 14, borderSize: 1, rounding: 12, mode: "dwindle", columnWidth: 0.5 });

const firstInt = (block) => {
  const m = String(block).match(/-?\d+/);
  return m ? parseInt(m[0]) : undefined;
};

/** True once the layout store reflects the real compositor state. Until then
 *  it holds compiled-in defaults, and regenerating user.lua from those would
 *  silently RESET the user's gaps/border/rounding — the classic "changed the
 *  accent, next reload my gaps were gone" bug. */
let layoutLoaded = false;

/** Seed the layout store from the live compositor state. */
export async function loadLayout() {
  try {
    const text = await api.hyprctl([
      "--batch",
      "getoption general:gaps_in; getoption general:gaps_out; getoption general:border_size; getoption decoration:rounding; getoption general:layout; getoption scrolling:column_width"
    ]);
    const blocks = text.split(/\n\s*\n/).filter((b) => b.trim() !== "");
    const [gi, go, bs, rd] = blocks.slice(0, 4).map(firstInt);
    // window layout: "str: dwindle" / "float: 0.500000" in getoption output
    const modeM = String(blocks[4] || "").match(/str:\s*(\w+)/);
    const cwM = String(blocks[5] || "").match(/float:\s*([0-9.]+)/);
    layout.update((l) => ({
      gapsIn: gi ?? l.gapsIn,
      gapsOut: go ?? l.gapsOut,
      borderSize: bs ?? l.borderSize,
      rounding: rd ?? l.rounding,
      mode: modeM ? modeM[1] : l.mode,
      columnWidth: cwM ? parseFloat(cwM[1]) : l.columnWidth
    }));
    layoutLoaded = true;
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

/** RFC-001 Phase 4 (0.6): user.lua is an ewe-conf build artifact — this
 *  sends only the four layout numbers; tint/accent/transparency/speed/tiling
 *  flow in from their own conf domains (setPrefs lands them first). Still
 *  seeds the layout store from the live compositor before the first write,
 *  so stale compiled-in defaults never overwrite the user's gaps. */
export async function writeUserLua() {
  if (!layoutLoaded) await loadLayout();
  const l = get(layout);
  await api.setConf("desktop.layout", {
    gaps_in: l.gapsIn, gaps_out: l.gapsOut, border_size: l.borderSize, rounding: l.rounding,
    mode: l.mode || "dwindle", column_width: l.columnWidth ?? 0.5
  });
}

/** Window layout (0.8): dwindle · master · scrolling (Hyprland's PaperWM-style
 *  tape). Live-applied like the gaps, persisted through desktop.layout. */
export async function setLayoutMode(mode) {
  layout.update((l) => ({ ...l, mode }));
  const l = get(layout);
  const stmts = [`hl.config({ general = { layout = "${mode}" } })`];
  if (mode === "scrolling")
    stmts.push(`hl.config({ scrolling = { column_width = ${l.columnWidth ?? 0.5}, follow_focus = true, fullscreen_on_one_column = true } })`);
  await evals(stmts);
  await writeUserLua();
}
export async function setColumnWidth(v) {
  layout.update((l) => ({ ...l, columnWidth: v }));
  await evals([`hl.config({ scrolling = { column_width = ${v} } })`]);
  await writeUserLua();
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
  await writeUserLua();
  // Once the Animations pane owns generated/animations.lua the multiplier is
  // baked into it, so a speed change regenerates + re-applies that file (it
  // wins over user.lua). Only pane-less installs still eval the legacy block.
  if (!(await reapplyForSpeed())) await evals(animLuaLines(Number(m)));
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

