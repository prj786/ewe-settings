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
  const shapeGroups = [
    { key: "corner", title: "Corners", dflt: "medium",
      opts: [["none", "Square"], ["small", "Slight"], ["medium", "Rounded"], ["large", "Soft"]] },
    { key: "density", title: "Density", dflt: "comfortable",
      opts: [["compact", "Compact"], ["comfortable", "Comfortable"], ["roomy", "Roomy"]] },
    { key: "stroke", title: "Rules", dflt: "thin",
      opts: [["thin", "Hairline"], ["thick", "Bold"]] }
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

<div class="mx-auto max-w-3xl space-y-6 p-5 sm:p-8">
  <h1 class="text-lg font-semibold">Appearance</h1>

  <section>
    <div class="section-title">Shape &amp; density</div>
    <div class="card p-4 space-y-3">
      {#each shapeGroups as g (g.key)}
        <div>
          <div class="mb-1.5 text-xs font-semibold text-secondary">{g.title}</div>
          <div class="flex gap-2">
            {#each g.opts as [val, label] (val)}
              <button
                class="flex-1 border px-3 py-2 text-sm font-medium transition-colors
                  {(shape[g.key] || g.dflt) === val
                  ? 'border-[var(--brand-bg)] bg-[var(--brand-bg)] text-[var(--fg-on-brand)]'
                  : 'border-hairline hover:bg-hover'}"
                style="border-radius: var(--radius-card)"
                disabled={busy}
                on:click={() => setShape(g.key, val)}
              >{label}</button>
            {/each}
          </div>
        </div>
      {/each}
      <div class="-mx-4 border-t border-hairline">
        <SliderRow label="Bar & dock transparency" value={barTransparency} from={0} to={100} unit=" %" dim={busy} moved={slideBar} />
        <ToggleRow
          title="Blur apps"
          sub="Every window at 85 % with what is behind it blurred — terminal, browser, files and the ewe apps alike. Fullscreen stays solid. A fixed level on purpose."
          dim={busy}
          on={String(shape.app_blur ?? false) === "true"}
          toggled={() => setShape("app_blur", !(String(shape.app_blur ?? false) === "true"))}
        />
      </div>
      <p class="text-xs text-dim dark:text-dim">
        Every colour in ewe is derived from your accent — there is no palette to pick. These set the shape of it: corner radius, spacing and control heights, the weight of every rule, and how see-through the top bar and dock are (what is behind them is blurred, except on VMs and NVIDIA where blur is off; the control centre and other panels stay solid). They live in ewe.conf, so they follow you to your other machines.
      </p>
    </div>
  </section>

  <section>
    <div class="section-title">Accent colour</div>
    <div class="card p-4">
        <div class="mb-3 flex flex-wrap gap-2.5">
          {#each ACCENTS as a (a.hex)}
            <button
              title={a.name}
              class="h-8 w-8 rounded-full transition-transform hover:scale-110
                {($prefs.accent || '#0a84ff').toLowerCase() === a.hex ? 'ring-2 ring-offset-2 ring-offset-white dark:ring-offset-zinc-900' : ''}"
              style="background: {a.hex}; --tw-ring-color: {a.hex}"
              disabled={busy}
              on:click={() => run(() => setAccent(a.hex))}
            ></button>
          {/each}
        </div>
        <p class="text-xs text-dim dark:text-dim">
          Applies to the shell, window borders and GTK/Qt apps.
        </p>
    </div>
  </section>

  <section>
    <div class="section-title">Windows & animations</div>
    <div class="card divide-y divide-hairline">
      <!-- Light mode is parked until it is actually fully light — the DE is
           dark-only for now, so no colour-scheme toggle here. -->
      <div class="flex items-center justify-between gap-3 px-4 py-3">
        <div>
          <div class="text-sm font-medium">Tint window borders</div>
          <div class="text-xs text-dim dark:text-dim">Active window border follows the accent.</div>
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
      <div class="flex items-center justify-between gap-3 px-4 py-3">
        <div>
          <div class="text-sm font-medium">Window transparency</div>
          <div class="text-xs text-dim dark:text-dim">Unfocused windows slightly translucent.</div>
        </div>
        <Checkbox
          checked={$prefs.windowTransparency !== false}
          disabled={busy}
          aria-label="Window transparency"
          onCheckedChange={(v) => run(() => setTransparency(v))}
        />
      </div>
      <div class="flex items-center justify-between gap-3 px-4 py-3">
        <div>
          <div class="text-sm font-medium">Event sounds</div>
          <div class="text-xs text-dim dark:text-dim">Chimes for notifications, volume, screenshots and power events.</div>
        </div>
        <Checkbox
          checked={$prefs.eventSounds !== false}
          disabled={busy}
          aria-label="Event sounds"
          onCheckedChange={(v) => run(() => setPrefs({ eventSounds: v }))}
        />
      </div>
      <div class="flex items-center justify-between gap-3 px-4 py-3">
        <div>
          <div class="text-sm font-medium">Animations</div>
          <div class="text-xs text-dim dark:text-dim">Speed, presets, curves and styles moved to their own page.</div>
        </div>
        <button
          class="shrink-0 rounded-full bg-elevated/70 px-3 py-1 text-xs font-medium text-dim transition-colors hover:bg-hover /60  dark:hover:bg-hover"
          on:click={() => pane.set("animations")}
        >
          Open Animations
        </button>
      </div>
    </div>
  </section>
</div>
