<script>
  import { onMount } from "svelte";
  import * as api from "../api.js";
  import { prefs, pane } from "../stores.js";
  import { ACCENTS } from "../hypr.js";
  import { Checkbox } from "./ui/checkbox/index.js";
  import SliderRow from "./ui/SliderRow.svelte";
  import ToggleRow from "./ui/ToggleRow.svelte";
  import {
    setAccent,
    setTransparency,
    setPrefs,
    applyBorder
  } from "../overrides.js";

  // There is ONE ewe look now (2026-09-04) — it is not picked from a list, it
  // is DERIVED from the accent below. What is still a choice is its shape and
  // how tightly it packs, and those are keys in ewe.conf [desktop.theme], so
  // they sync with the rest of the machine instead of living in a second file
  // that `ewe-conf pull` would quietly overwrite.
  // Defaults mirror the engine's (2026-09 revamp): `round` corners and no
  // outline on controls — separation is the background step, not an edge.
  const shapeGroups = [
    { key: "corner", title: "Corners", dflt: "round",
      opts: [["none", "Square"], ["small", "Slight"], ["medium", "Rounded"], ["large", "Soft"], ["round", "Round"]] },
    { key: "density", title: "Density", dflt: "comfortable",
      opts: [["compact", "Compact"], ["comfortable", "Comfortable"], ["roomy", "Roomy"]] },
    { key: "stroke", title: "Outlines", dflt: "none",
      opts: [["none", "None"], ["thin", "Hairline"], ["thick", "Bold"]] }
  ];
  // Bar & dock transparency: 0 = solid, 100 = see-through. Stored as
  // desktop.theme.bar_opacity (the inverse) through ewe-conf like the shape
  // knobs; between 1 and 90 the compositor blurs behind the bar and dock
  // (not on VMs / NVIDIA, where blur is off by policy). Every other panel
  // stays opaque. Debounced: a slider fires dozens of times per drag and
  // each write rebuilds the tokens and reloads Hyprland.
  let barTransparency = 0;
  let barTimer;
  function slideBar(v) {
    barTransparency = Math.round(v);
    clearTimeout(barTimer);
    barTimer = setTimeout(() => setShape("bar_opacity", 100 - barTransparency), 250);
  }
  // The live values come from the token file, which is what ewe-theme was
  // last built from — never a second copy in this app that could disagree.
  let shape = {};
  async function loadShape() {
    try {
      const t = await api.themeTokens("ewe");
      shape = (t && t.input) || {};
      barTransparency = 100 - Math.max(0, Math.min(100, Number(shape.bar_opacity ?? 100)));
    } catch { shape = {}; }
  }
  onMount(loadShape);
  async function setShape(key, value) {
    await run(() => api.setConf(`desktop.theme.${key}`, value));
    await loadShape();
  }

  let busy = false;
  async function run(fn) {
    busy = true;
    try {
      await fn();
    } catch (e) {
      console.error(e);
    }
    busy = false;
  }
</script>

<div class="pane-body">
  <h1 class="pane-title">Appearance</h1>

  <section>
    <div class="section-title">Shape &amp; density</div>
    <div class="card p-6">
      <div class="space-y-3">
        {#each shapeGroups as g (g.key)}
          <div class="flex gap-2" role="group" aria-label={g.title}>
            {#each g.opts as [val, label] (val)}
              <button
                class="seg {(shape[g.key] || g.dflt) === val ? 'is-active' : ''}"
                aria-pressed={(shape[g.key] || g.dflt) === val}
                disabled={busy}
                on:click={() => setShape(g.key, val)}
              >{label}</button>
            {/each}
          </div>
        {/each}
      </div>
      <div class="-mx-6 mt-3">
        <SliderRow label="Bar & dock transparency" value={barTransparency} from={0} to={100} unit=" %" dim={busy} moved={slideBar} />
        <ToggleRow
          title="Blur apps"
          sub="Every window at 85 % with what is behind it blurred — terminal, browser, files and the ewe apps alike. Fullscreen stays solid. A fixed level on purpose."
          dim={busy}
          on={String(shape.app_blur ?? false) === "true"}
          toggled={() => setShape("app_blur", !(String(shape.app_blur ?? false) === "true"))}
        />
      </div>
      <p class="mt-3 text-sm text-dim">
        Every colour in ewe is derived from your accent — there is no palette to pick. These set the shape of it: corner radius, spacing and control heights, and whether controls draw their own outline; then how see-through the top bar and dock are (what is behind them is blurred, except on VMs and NVIDIA where blur is off; the control centre and other panels stay solid). They live in ewe.conf, so they follow you to your other machines.
      </p>
    </div>
  </section>

  <section>
    <div class="section-title">Accent colour</div>
    <div class="card p-6">
      <div class="mb-4 flex flex-wrap gap-3">
        {#each ACCENTS as a (a.hex)}
          <button
            title={a.name}
            aria-label={a.name}
            class="swatch {($prefs.accent || '#0a84ff').toLowerCase() === a.hex ? 'is-active' : ''}"
            style="background: {a.hex}"
            disabled={busy}
            on:click={() => run(() => setAccent(a.hex))}
          ></button>
        {/each}
      </div>
      <p class="text-sm text-dim">
        Applies to the shell, window borders and GTK/Qt apps.
      </p>
    </div>
  </section>

  <section>
    <div class="section-title">Windows & animations</div>
    <div class="card py-4">
      <!-- Light mode is parked until it is actually fully light — the DE is
           dark-only for now, so no colour-scheme toggle here. -->
      <div class="row">
        <div>
          <div class="row-title">Tint window borders</div>
          <div class="row-sub">Active window border follows the accent.</div>
        </div>
        <Checkbox
          checked={!!$prefs.tintBorders}
          disabled={busy}
          aria-label="Tint window borders"
          onCheckedChange={(v) =>
            run(async () => {
              await setPrefs({ tintBorders: v });
              await applyBorder();
            })}
        />
      </div>
      <div class="row">
        <div>
          <div class="row-title">Window transparency</div>
          <div class="row-sub">Unfocused windows slightly translucent.</div>
        </div>
        <Checkbox
          checked={$prefs.windowTransparency !== false}
          disabled={busy}
          aria-label="Window transparency"
          onCheckedChange={(v) => run(() => setTransparency(v))}
        />
      </div>
      <div class="row">
        <div>
          <div class="row-title">Event sounds</div>
          <div class="row-sub">Chimes for notifications, volume, screenshots and power events.</div>
        </div>
        <Checkbox
          checked={$prefs.eventSounds !== false}
          disabled={busy}
          aria-label="Event sounds"
          onCheckedChange={(v) => run(() => setPrefs({ eventSounds: v }))}
        />
      </div>
      <div class="row">
        <div>
          <div class="row-title">Animations</div>
          <div class="row-sub">Speed, presets, curves and styles moved to their own page.</div>
        </div>
        <button class="link-action" on:click={() => pane.set("animations")}>
          Open Animations
        </button>
      </div>
    </div>
  </section>
</div>
