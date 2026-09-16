<script>
  import { onMount } from "svelte";
  import * as api from "../api.js";
  import { prefs, pane } from "../stores.js";
  import { ACCENTS } from "../hypr.js";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { flashApplied, errorMsg } from "../stores.js";
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

  // ── colours: accent-derived (the default), a scheme, or the wallpaper ──
  // A scheme is a whole Base24 palette in ewe.conf ([[desktop.theme.schemes]]);
  // ewe-theme owns every write and ewe-conf's hooks repaint everything, so
  // this pane only asks and shows. Nothing is bundled — the user imports.
  let schemes = []; // [{slug, name, variant, accent, current, swatch[5]}]
  let schemeSlug = "accent"; // ewe.conf desktop.theme.scheme
  let editOpen = false;
  let currentPalette = null; // {base00…} of the active scheme, for the editor
  $: colourMode = schemeSlug === "accent" ? "accent" : schemeSlug === "wallpaper" ? "wallpaper" : "scheme";
  $: currentScheme = schemes.find((s) => s.slug === schemeSlug) || null;
  async function loadSchemes() {
    try {
      const r = await api.themeScheme("list");
      schemes = r.schemes || [];
      schemeSlug = r.current || "accent";
      if (schemeSlug !== "accent") {
        const s = await api.themeScheme("show", schemeSlug);
        currentPalette = (s.scheme && s.scheme.palette) || null;
      } else currentPalette = null;
    } catch (e) {
      schemes = [];
    }
  }
  onMount(loadSchemes);
  async function scheme(...args) {
    busy = true;
    try {
      const r = await api.themeScheme(...args);
      await loadSchemes();
      await loadShape();
      // the accent swatches read $prefs.accent; keep it on the effective one
      if (shape.accent) await setPrefs({ accent: shape.accent });
      return r;
    } catch (e) {
      errorMsg.set(String(e));
    } finally {
      busy = false;
    }
  }
  async function importScheme() {
    const f = await openDialog({
      multiple: false,
      title: "Import a colour scheme",
      filters: [{ name: "Colour schemes", extensions: ["yaml", "yml", "toml", "json"] }]
    });
    if (!f) return;
    const r = await scheme("import", typeof f === "string" ? f : f.path, "--apply");
    if (r && r.imported) flashApplied(`${r.imported.name} applied`);
  }
  async function pickAccent(hex) {
    if (colourMode === "accent") return run(() => setAccent(hex));
    await scheme("set", "accent", hex);
  }
  async function exportScheme() {
    const r = await api.themeScheme("export", schemeSlug);
    if (r && r.yaml) {
      await navigator.clipboard.writeText(r.yaml);
      flashApplied("Palette copied as Base24 YAML");
    }
  }
  const ROLES = [
    ["base00", "Background"], ["base01", "Panels"], ["base02", "Cards & selection"], ["base03", "Lines & muted"],
    ["base05", "Text"], ["base07", "Bright text"], ["base08", "Danger"], ["base0A", "Warning"], ["base0B", "Success"]
  ];
  let roleTimer;
  function editRole(key, hex) {
    clearTimeout(roleTimer);
    roleTimer = setTimeout(() => scheme("set", key, hex), 300);
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
        These set the shape of the look: corner radius, spacing and control heights, and whether controls draw their own outline; then how see-through the top bar and dock are (what is behind them is blurred, except on VMs and NVIDIA where blur is off; the control centre and other panels stay solid). They live in ewe.conf, so they follow you to your other machines.
      </p>
    </div>
  </section>

  <section>
    <div class="section-title">Colours</div>
    <div class="card p-6">
      <div class="flex gap-2" role="group" aria-label="Where the colours come from">
        <button class="seg {colourMode === 'accent' ? 'is-active' : ''}" disabled={busy} on:click={() => scheme("apply", "accent")}>Accent</button>
        <button class="seg {colourMode === 'scheme' ? 'is-active' : ''}" disabled={busy || !schemes.some((s) => s.slug !== 'wallpaper')}
          on:click={() => { const s = schemes.find((x) => x.slug !== 'wallpaper'); if (s) scheme('apply', s.slug); }}>Scheme</button>
        <button class="seg {colourMode === 'wallpaper' ? 'is-active' : ''}" disabled={busy} on:click={() => scheme("from-wallpaper", "--apply")}>Wallpaper</button>
      </div>

      {#if colourMode === "accent"}
        <p class="mt-3 text-sm text-dim">Every colour derives from one accent: the shell, window borders, GTK and Qt apps, the terminal.</p>
      {:else if colourMode === "wallpaper"}
        <p class="mt-3 text-sm text-dim">The palette is pulled out of the wallpaper and follows it when it changes. The accent below is the picture's strongest colour; pick another to override it.</p>
      {:else}
        <p class="mt-3 text-sm text-dim">A whole palette. Import Base16/Base24 YAML, an Omarchy <code>colors.toml</code>, Catppuccin's <code>palette.json</code> or a Gogh theme; a light scheme is honoured end to end.</p>
      {/if}

      <!-- the accent swatches work in every mode: on a scheme they override its accent -->
      <div class="mt-4 flex flex-wrap items-center gap-3">
        {#each ACCENTS as a (a.hex)}
          <button
            title={a.name}
            aria-label={a.name}
            class="swatch {String(shape.accent || $prefs.accent || '#0a84ff').toLowerCase() === a.hex ? 'is-active' : ''}"
            style="background: {a.hex}"
            disabled={busy}
            on:click={() => pickAccent(a.hex)}
          ></button>
        {/each}
        <label class="flex items-center gap-2 text-sm text-dim">
          <input type="color" class="h-7 w-9 cursor-pointer rounded-md border-0 bg-transparent p-0" value={String(shape.accent || '#0a84ff')} disabled={busy}
            on:change={(e) => pickAccent(e.currentTarget.value)} aria-label="Any accent colour" />
          any
        </label>
      </div>

      {#if schemes.length || colourMode !== "accent"}
        <div class="mt-5 grid gap-2" style="grid-template-columns: repeat(auto-fill, minmax(150px, 1fr))">
          {#each schemes as s (s.slug)}
            <button
              class="rounded-xl border p-2 text-left transition {s.current ? 'border-[var(--brand-stroke-1)]' : 'border-transparent hover:border-[var(--stroke-2)]'}"
              disabled={busy}
              on:click={() => scheme("apply", s.slug)}
              title={s.source || s.name}
            >
              <div class="flex h-7 overflow-hidden rounded-lg">
                {#each s.swatch as c}<span class="flex-1" style="background: {c}"></span>{/each}
              </div>
              <div class="mt-1.5 flex items-center gap-1.5">
                <span class="min-w-0 flex-1 truncate text-sm">{s.name}</span>
                <span class="text-[10px] uppercase tracking-wide text-dim">{s.variant}</span>
              </div>
            </button>
          {/each}
        </div>
      {/if}

      <div class="mt-4 flex flex-wrap gap-2">
        <button class="btn-ghost" disabled={busy} on:click={importScheme}>Import…</button>
        {#if colourMode !== "accent" && currentScheme}
          <button class="btn-ghost" disabled={busy} on:click={exportScheme}>Copy as YAML</button>
          <button class="btn-ghost" disabled={busy} on:click={() => (editOpen = !editOpen)}>{editOpen ? "Done editing" : "Edit palette"}</button>
          <button class="btn-ghost" disabled={busy} on:click={() => scheme("remove", currentScheme.slug)}>Remove</button>
        {/if}
        {#if colourMode === "wallpaper"}
          <button class="btn-ghost" disabled={busy} on:click={() => scheme("from-wallpaper", "--apply", "--light")}>Light version</button>
        {/if}
      </div>

      {#if editOpen && currentPalette}
        <div class="mt-4 grid gap-2" style="grid-template-columns: repeat(auto-fill, minmax(170px, 1fr))">
          {#each ROLES as [key, label] (key)}
            <label class="flex items-center gap-2 rounded-lg bg-elevated px-2 py-1.5 text-sm">
              <input type="color" class="h-7 w-9 cursor-pointer rounded-md border-0 bg-transparent p-0" value={currentPalette[key] || "#000000"}
                on:input={(e) => editRole(key, e.currentTarget.value)} aria-label={label} />
              <span class="min-w-0 flex-1 truncate">{label}</span>
              <span class="font-mono text-xs text-dim">{key}</span>
            </label>
          {/each}
        </div>
        <p class="mt-2 text-xs text-dim">Edits go into this scheme in ewe.conf and repaint live. Variant (dark/light) comes from the imported file; change it with <code>ewe-theme scheme set variant light</code>.</p>
      {/if}
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
