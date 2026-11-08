// Find a setting: the search at the top of the side navigation.
//
// The index is written out here rather than scraped from the pages, because
// most rows only exist once their page has loaded its data (Network waits
// for NetworkManager, Displays for Hyprland). Each entry names its page (a
// pane id from lib/pages.js), the row's title exactly as the page shows it
// (`label`, which the row carries as data-setting), the group it sits in
// (`section`, the group's data-section), and the words people search with
// (`keys`, synonyms such as "dark" or "wifi"). scripts/check-search.mjs
// holds every label to its page file, so a renamed row fails the check
// instead of silently dropping out of the results.

import { writable } from "svelte/store";
import { PAGES, pageById } from "./pages.js";

// [page, label, section, keys]
const ROWS = [
  // Look
  ["appearance", "Scheme", "Scheme", "theme dark light mode night palette colors ewe dark ewe light"],
  ["appearance", "From wallpaper", "Scheme", "wallpaper colors picture generate"],
  ["appearance", "Import scheme", "", "base16 base24 catppuccin omarchy gogh yaml theme"],
  ["appearance", "Accent color", "Accent color", "colour highlight ewellow custom"],
  ["appearance", "Corners", "Look", "radius rounded square round shape"],
  ["appearance", "Density", "Look", "compact comfortable roomy spacing size"],
  ["appearance", "Outlines", "Look", "border stroke lines"],
  ["appearance", "Glass", "Glass", "transparency transparent blur see-through bar dock"],
  ["appearance", "Opacity", "Glass", "bar opacity transparency solid"],
  ["appearance", "Tint window borders", "Windows", "border accent color"],
  ["appearance", "App blur", "Windows", "blur transparency see-through windows"],
  ["appearance", "Window transparency", "Windows", "opacity transparent inactive"],
  ["appearance", "Event sounds", "Sound", "sound chime audio notification beep"],
  ["wallpaper", "Choose file", "", "picture image video background"],
  ["wallpaper", "Set for", "Options", "display monitor screen"],
  ["wallpaper", "Fit", "Options", "fill center tile stretch"],
  ["wallpaper", "Mute video wallpapers", "Options", "sound audio video"],
  ["animations", "Animations", "Overall", "motion effects off"],
  ["animations", "Speed", "Overall", "fast slow"],
  ["animations", "Character", "Character", "snappy smooth minimal style feel"],
  ["animations", "Window open", "Fine-tune", "duration curve style"],
  ["animations", "Window close", "Fine-tune", "duration curve style"],
  ["animations", "Window fade", "Fine-tune", "duration curve"],
  ["animations", "Workspace switch", "Fine-tune", "duration curve slide"],
  ["animations", "Panels and popups", "Fine-tune", "duration curve menus"],
  ["animations", "Border color", "Fine-tune", "duration colour"],
  ["accessibility", "Increase contrast", "Seeing", "high contrast vision"],
  ["accessibility", "Reduce transparency", "Seeing", "solid opaque blur glass"],
  ["accessibility", "Text size", "Seeing", "font bigger larger zoom scale"],
  ["accessibility", "Reduce motion", "Motion", "animations movement"],
  // Desktop
  ["layout", "Tiling", "Windows", "floating tile stacking"],
  ["layout", "Tiling style", "Windows", "dwindle master scrolling monocle"],
  ["layout", "Column width", "Windows", "scrolling"],
  ["layout", "Inner gaps", "Gaps and borders", "gap space padding"],
  ["layout", "Outer gaps", "Gaps and borders", "gap space margin"],
  ["layout", "Border width", "Gaps and borders", "border thickness"],
  ["layout", "Corner radius", "Gaps and borders", "rounding rounded corners"],
  ["layout", "Top bar", "Top bar", "panel bar hide"],
  ["layout", "Bar icons", "Top bar", "size bar height"],
  ["layout", "Sound", "Status row", "volume bar"],
  ["layout", "Microphone in use", "Status row", "mic bar"],
  ["layout", "Network", "Status row", "wifi bar"],
  ["layout", "Bluetooth", "Status row", "bar"],
  ["layout", "Battery", "Status row", "bar percentage"],
  ["layout", "Power profile", "Status row", "bar"],
  ["layout", "Keyboard layout", "Status row", "bar language"],
  ["layout", "System tray", "Status row", "tray bar icons"],
  ["layout", "Tiling or floating", "Status row", "bar"],
  ["layout", "Plugins in the bar", "Status row", "plugins widgets komble"],
  ["layout", "Dock", "Dock", "dock taskbar launcher plugin"],
  ["windowrules", "Add rule", "", "workspace float tile app"],
  ["displays", "Arrangement", "Arrangement", "arrange position drag monitors multiple"],
  ["displays", "Workspaces", "Workspaces", "separate shared multiple monitors"],
  ["displays", "Resolution", "", "screen size pixels"],
  ["displays", "Refresh rate", "", "hz fps"],
  ["displays", "Scale", "", "hidpi zoom size fractional"],
  ["displays", "Rotation", "", "rotate portrait orientation"],
  ["displays", "Mirror", "", "duplicate clone projector"],
  ["displays", "Variable refresh rate", "", "vrr adaptive sync freesync gsync"],
  ["displays", "10-bit color", "", "hdr color depth banding"],
  ["displays", "Primary display", "", "main"],
  ["displays", "Reset displays", "", "wake profile black screen"],
  ["startup", "Add app", "", "autostart login launch"],
  // Devices
  ["network", "Wi-Fi", "Wi-Fi", "wifi wireless internet ssid hotspot"],
  ["network", "Wired", "Wired", "ethernet cable lan"],
  ["network", "Active connections", "Active connections", "ip address connected"],
  ["network", "Saved networks", "Saved networks", "dns ip mac mtu static manual"],
  ["network", "VPN", "VPN", "wireguard openvpn l2tp ipsec work"],
  ["network", "Add VPN", "VPN", "wireguard openvpn import"],
  ["network", "Name on the network", "This computer", "hostname computer name rename"],
  ["network", "SSH hosts", "SSH hosts", "ssh servers"],
  ["bluetooth", "Bluetooth", "This computer", "bt wireless on off"],
  ["bluetooth", "Visible to other devices", "This computer", "discoverable"],
  ["bluetooth", "My devices", "My devices", "paired headphones keyboard mouse connect"],
  ["bluetooth", "Nearby devices", "Nearby devices", "pair search scan"],
  ["input", "Keyboard layouts", "Keyboard layouts", "keyboard language layout input us georgian russian"],
  ["input", "Switch layouts with", "Typing", "keyboard alt shift caps toggle language"],
  ["input", "Remember the layout for each window", "Typing", "keyboard per window"],
  ["input", "Num Lock on at start", "Typing", "keyboard numlock numpad"],
  ["input", "Key repeat rate", "Typing", "keyboard repeat"],
  ["input", "Repeat delay", "Typing", "keyboard repeat"],
  ["input", "Pointer speed", "Mouse", "mouse sensitivity cursor"],
  ["input", "Acceleration", "Mouse", "mouse flat adaptive"],
  ["input", "Natural scrolling", "Mouse", "mouse reverse scroll direction"],
  ["input", "Left-handed buttons", "Mouse", "mouse left hand"],
  ["input", "Scroll speed", "Mouse", "mouse wheel"],
  ["input", "Cursor size", "Cursor", "pointer mouse arrow big bigger large accessibility"],
  ["input", "Cursor color", "Cursor", "pointer mouse arrow colour white black"],
  ["input", "Tap to click", "Touchpad", "touchpad trackpad tap"],
  ["input", "Natural scrolling", "Touchpad", "touchpad trackpad reverse scroll"],
  ["input", "Turn off while typing", "Touchpad", "touchpad trackpad palm"],
  ["input", "Two-finger right-click", "Touchpad", "touchpad trackpad clickfinger"],
  ["input", "Middle-button emulation", "Touchpad", "touchpad trackpad middle click"],
  ["input", "Tap and drag", "Touchpad", "touchpad trackpad"],
  ["input", "Drag lock", "Touchpad", "touchpad trackpad"],
  ["input", "Scroll speed", "Touchpad", "touchpad trackpad"],
  ["power", "Suspend even when docked", "When the lid closes", "lid laptop close sleep"],
  ["power", "Charge", "Battery", "battery percent"],
  ["power", "Health", "Battery", "battery cycles wear"],
  ["power", "Charge limit", "Battery", "battery threshold 80 longevity"],
  ["power", "Brightness", "Keyboard backlight", "keyboard light backlight"],
  ["power", "Power profile", "Performance", "performance balanced power saver"],
  ["power", "Low power mode", "On battery", "battery saver"],
  ["power", "Game mode", "Gaming", "game gaming games steam fps boost gamemode performance ultra"],
  ["power", "Game mode level", "Gaming", "ultra balanced game gaming fps performance"],
  ["power", "Turn on while a game runs", "Gaming", "automatic game steam gamescope"],
  ["power", "Close background apps", "Gaming", "game kill processes memory ram ultra"],
  ["saver", "Screensaver", "Screensaver", "idle clock blank"],
  ["saver", "Start after", "Screensaver", "idle minutes timeout"],
  ["saver", "Style", "Screensaver", "clock blank"],
  ["saver", "Require password", "Locking", "lock password"],
  ["saver", "Lock after the screensaver starts", "Locking", "lock timeout"],
  ["saver", "Preview", "", "screensaver show"],
  // System
  ["defaults", "Browser", "", "default web links"],
  ["defaults", "Mail", "", "default email mailto"],
  ["defaults", "Text editor", "", "default text notes"],
  ["defaults", "Image viewer", "", "default pictures photos"],
  ["defaults", "Video player", "", "default videos movies"],
  ["defaults", "File manager", "", "default folders files"],
  ["time", "Date and time", "Clock", "clock now"],
  ["time", "Network time", "Clock", "ntp sync clock"],
  ["time", "Set the time zone automatically", "Time zone", "timezone automatic location"],
  ["time", "Region", "Time zone", "timezone continent"],
  ["time", "Zone", "Time zone", "timezone city"],
  ["time", "Display language", "Language", "language locale translation"],
  ["user", "Picture and name", "", "avatar photo face full name"],
  ["user", "Picture shape", "", "avatar circle rounded square"],
  ["user", "Nextcloud account", "Nextcloud account", "account cloud sign in login"],
  ["user", "Sync status", "Settings sync", "backup restore sync"],
  ["user", "Google", "Google", "gmail drive"],
  ["system", "ewe", "", "version"],
  ["system", "Hyprland", "", "compositor version"],
  ["system", "Memory", "", "ram"],
  ["system", "Disk", "", "storage space"],
  ["system", "Plugins", "Plugins", "komble extensions add-ons"],
  ["system", "Session health", "Session health", "portal services"],
  ["system", "Restart the shell", "Shell", "restart reload bar dock panels"]
];

// a page's own words: its name finds it, and so do these
const PAGE_KEYS = {
  appearance: "theme dark light mode colors accent look",
  wallpaper: "background picture image video desktop",
  animations: "motion effects speed",
  accessibility: "a11y contrast text size motion vision",
  layout: "tiling gaps bar dock windows",
  windowrules: "workspace float rules apps",
  displays: "monitor screen resolution scale",
  startup: "autostart login sign in",
  shortcuts: "keybindings hotkeys keys keyboard",
  network: "wifi wi-fi wireless internet ethernet vpn",
  bluetooth: "headphones pair devices",
  input: "keyboard mouse touchpad trackpad typing layout",
  power: "battery lid suspend sleep",
  saver: "screensaver lock idle sleep",
  defaults: "default apps browser mail open with",
  time: "date clock timezone language region locale",
  user: "account avatar name nextcloud google",
  system: "about version info hardware system"
};

const norm = (s) =>
  String(s || "")
    .toLowerCase()
    .normalize("NFKD")
    .replace(/[̀-ͯ]/g, "")
    .replace(/[^a-z0-9]+/g, " ")
    .trim();
const words = (s) => norm(s).split(" ").filter(Boolean);

/** Every searchable thing: the pages, then their rows. */
export const INDEX = [
  ...PAGES.map((p) => ({ page: p.id, label: p.label, section: "", keys: PAGE_KEYS[p.id] || "", isPage: true })),
  ...ROWS.map(([page, label, section, keys]) => ({ page, label, section, keys, isPage: false }))
].map((e, i) => {
  const p = pageById(e.page);
  return {
    ...e,
    key: `${e.page}:${e.section}:${e.label}`,
    order: i,
    where: e.isPage ? p.group : section(p.label, e.section, e.label),
    _label: words(e.label),
    _compact: norm(e.label).replace(/ /g, ""),
    _all: words(`${e.label} ${e.section} ${p.label} ${e.keys}`)
  };
});
function section(page, sec, label) {
  return sec && sec !== label ? `${page} › ${sec}` : page;
}

/**
 * The entries a query finds, best first. Every word of the query has to
 * start some word of the entry (its title, group, page or keywords); the
 * row's own title starting with the query ranks highest, then title words,
 * then pages, then keywords. "wi" finds Wi-Fi, "dark" the schemes.
 */
export function findSettings(query, limit = 40) {
  const q = words(query);
  if (!q.length) return [];
  const joined = q.join(" ");
  const hits = [];
  for (const e of INDEX) {
    const compact = q.join("");
    if (!q.every((w) => e._all.some((x) => x.startsWith(w))) && !e._compact.startsWith(compact)) continue;
    const label = e._label.join(" ");
    let score = 0;
    if (label === joined || e._compact === joined.replace(/ /g, "")) score = 100;
    else if (label.startsWith(joined) || e._compact.startsWith(joined.replace(/ /g, ""))) score = 90;
    else if (q.every((w) => e._label.some((x) => x.startsWith(w)))) score = 70;
    else score = 30;
    if (e.isPage) score += 15;
    hits.push({ e, score });
  }
  hits.sort((a, b) => b.score - a.score || a.e.order - b.e.order);
  return hits.slice(0, limit).map((h) => h.e);
}

/** The result being opened: { page, label, section, seq } (App.svelte). */
export const found = writable(null);

/**
 * Find a result's row on its page, scroll to it, mark it for a moment and
 * hand its control the focus. Pages fill in as their data arrives, so this
 * looks for up to four seconds before settling for the group, then the top.
 */
export async function reveal(root, target) {
  if (!root || !target) return;
  const esc = (v) => (window.CSS && CSS.escape ? CSS.escape(v) : String(v).replace(/"/g, '\\"'));
  const inSection = target.section ? `[data-section="${esc(target.section)}"] ` : "";
  const row = () =>
    (inSection && root.querySelector(`${inSection}[data-setting="${esc(target.label)}"]`)) ||
    root.querySelector(`[data-setting="${esc(target.label)}"]`);
  const group = () => target.section && root.querySelector(`[data-section="${esc(target.section)}"]`);
  let el = null;
  for (let i = 0; i < 40 && !el && !target.isPage; i++) {
    el = row();
    // a row that names its whole group ("Wi-Fi", "My devices") may only be
    // the group itself; any other row gets a moment longer to load
    if (!el && (target.label === target.section ? i >= 3 : i >= 15)) el = group();
    if (!el) await new Promise((r) => setTimeout(r, 100));
  }
  const scroller = root.querySelector(".page-scroll");
  if (!el) {
    scroller?.scrollTo({ top: 0 });
    return;
  }
  const still = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
  el.scrollIntoView({ block: "center", behavior: still ? "auto" : "smooth" });
  el.classList.add("is-found");
  setTimeout(() => el.classList.remove("is-found"), 2400);
  // the row's own control, so Space or the arrows change it at once
  const ctl = el.matches("button, input, select, [role='switch'], [role='radio']")
    ? el
    : el.querySelector(
        "[role='switch'], [role='radio'][tabindex='0'], [role='slider'], button:not([disabled]), input:not([disabled]), select, [tabindex='0']"
      );
  if (ctl) ctl.focus({ preventScroll: true });
}
