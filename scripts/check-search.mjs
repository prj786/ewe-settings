// npm run check — Find a setting's index against the pages it points at.
//
// The index in src/lib/search.js is written by hand (most rows only exist
// once their page has loaded data), so this holds it to the source: every
// row title it lists must appear as a string in its page (or the module the
// page takes its labels from), and every group it names must be a
// <Group title="…"> there. Then it runs the searches people actually type.
// Exits 1 on the first kind of failure, printing every miss.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { INDEX, findSettings } from "../src/lib/search.js";
import { PAGES } from "../src/lib/pages.js";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (p) => readFileSync(join(root, p), "utf8");

// pane id → the files its labels can come from
const FILES = {
  appearance: ["src/lib/components/Appearance.svelte"],
  wallpaper: ["src/lib/components/WallpaperPane.svelte"],
  animations: ["src/lib/components/Animations.svelte", "src/lib/hypr.js"],
  accessibility: ["src/lib/components/Accessibility.svelte"],
  layout: ["src/lib/components/Layout.svelte"],
  windowrules: ["src/lib/components/WindowRules.svelte"],
  displays: ["src/lib/components/Displays.svelte"],
  startup: ["src/lib/components/Startup.svelte"],
  shortcuts: ["src/lib/components/Shortcuts.svelte"],
  network: ["src/lib/components/Network.svelte"],
  bluetooth: ["src/lib/components/Bluetooth.svelte"],
  input: ["src/lib/components/Input.svelte"],
  power: ["src/lib/components/PowerPane.svelte"],
  saver: ["src/lib/components/Saver.svelte"],
  defaults: ["src/lib/components/DefaultApps.svelte", "src/lib/hypr.js"],
  time: ["src/lib/components/TimePlace.svelte"],
  user: ["src/lib/components/UserPane.svelte"],
  system: ["src/lib/components/SystemInfo.svelte"]
};

const misses = [];
for (const p of PAGES) {
  if (!FILES[p.id]) misses.push(`page ${p.id}: no file listed in check-search.mjs`);
  else if (!read(FILES[p.id][0]).includes(`title="${p.label}"`))
    misses.push(`page ${p.id}: its <Page title> is not "${p.label}" (the nav label and the title must match)`);
}
for (const e of INDEX.filter((e) => !e.isPage)) {
  const text = (FILES[e.page] || []).map(read).join("\n");
  // Default apps title-case their keys ("Text Editor" → "Text editor")
  const variants = [e.label, e.label.replace(/\b\w/g, (c) => c.toUpperCase())];
  if (!variants.some((v) => text.includes(`"${v}"`) || text.includes(`"${v}…`) || text.includes(`setting="${v}"`)))
    misses.push(`${e.page}: no row "${e.label}"`);
  if (e.section && !text.includes(`title="${e.section}"`)) misses.push(`${e.page}: no group "${e.section}"`);
}

// what people type → what must come first (or at least be there)
const SEARCHES = [
  ["wallpaper", "wallpaper", "Wallpaper"],
  ["dark", "appearance", "Appearance"],
  ["dark", "appearance", "Scheme", "any"],
  ["wifi", "network", "Wi-Fi", "any"],
  ["wi-fi", "network", "Wi-Fi", "any"],
  ["keyboard", "input", "Keyboard and mouse", "any"],
  ["keyboard", "shortcuts", "Keyboard shortcuts", "any"],
  ["default apps", "defaults", "Default apps"],
  ["glass", "appearance", "Glass"],
  ["touchpad", "input", "Keyboard and mouse", "any"],
  ["hostname", "network", "Name on the network", "any"],
  ["restart", "system", "Restart the shell", "any"]
];
for (const [q, page, label, how] of SEARCHES) {
  const r = findSettings(q);
  const hit = (e) => e && e.page === page && e.label === label;
  if (how === "any" ? !r.some(hit) : !hit(r[0]))
    misses.push(`search "${q}": expected ${label} (${page}) ${how === "any" ? "among the results" : "first"}, got ${r.slice(0, 3).map((e) => e.label).join(", ") || "nothing"}`);
}
if (findSettings("zzqx").length) misses.push(`search "zzqx": expected nothing`);

if (misses.length) {
  console.error(misses.map((m) => "✗ " + m).join("\n"));
  process.exit(1);
}
console.log(`✓ ${INDEX.length} entries match their pages; ${SEARCHES.length + 1} searches find what they should`);
