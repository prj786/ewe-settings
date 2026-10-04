// Add-ons (ewe 0.25): the dock, music, Places, the phone, mail, cast, the
// VPN/SSH tiles, Insomnia and the system monitor are opt-in plugins that
// Komble installs. Panes with controls for one of them read this store and
// show "this is an add-on" instead of switches that would move nothing.
//
// `legacy: true` is the old-ewe escape hatch (no ewe-plugin, or one that
// predates add-ons): every add-on counts as installed, so Settings on an
// older shell looks exactly as it did. `loaded` gates the first paint — the
// probe takes ~100 ms and showing controls that then vanish is worse than a
// short "Checking…".

import { writable } from "svelte/store";
import * as api from "./api.js";

export const addons = writable({ loaded: false, legacy: true, installed: {}, plugins: [], available: [], removed: [] });

let inFlight = null;
/** Re-read `ewe-plugin list --json` (the backend caches it for a few seconds). */
export function loadAddons(refresh = false) {
  if (inFlight) return inFlight;
  inFlight = (async () => {
    try {
      const s = await api.addonsState(refresh);
      addons.set({ ...s, loaded: true });
    } catch (e) {
      // no backend answer at all: behave like an old ewe, hide nothing
      console.error(e);
      addons.set({ loaded: true, legacy: true, installed: {}, plugins: [], available: [], removed: [] });
    } finally {
      inFlight = null;
    }
  })();
  return inFlight;
}

/** Installed AND enabled — or any add-on at all on a pre-add-ons ewe. */
export const hasAddon = (s, id) => !!s.legacy || s.installed?.[id] === true;
/** Installed, on or off. */
export const addonInstalled = (s, id) => !!s.legacy || Object.prototype.hasOwnProperty.call(s.installed || {}, id);

/** Enabled plugins that put a widget in the top bar (BarPluginSlots gates
 *  each by the `plugin:<id>` key of the same show/hide map the built-ins use). */
export const barWidgetPlugins = (s) =>
  (s.plugins || []).filter((p) => p.enabled && p.valid !== false && (p.kinds || []).includes("bar-widget"));

/** Komble → Add-ons. Throws a sentence the error banner can show when Komble is missing. */
export async function openAddons() {
  const ok = await api.openAddons();
  if (!ok) throw new Error("Komble isn't installed, so there is nowhere to browse add-ons.");
}
