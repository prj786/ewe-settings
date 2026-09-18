<script>
  import { onMount } from "svelte";
  import { Dialog } from "bits-ui";
  import * as api from "../api.js";
  import { prefs, pane, errorMsg, flashApplied, toast } from "../stores.js";
  import { theme, refreshTheme } from "../theme.js";
  import { ACCENTS } from "../hypr.js";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import Page from "./ui/Page.svelte";
  import Group from "./ui/Group.svelte";
  import Row from "./ui/Row.svelte";
  import Seg from "./ui/Seg.svelte";
  import Icon from "./ui/Icon.svelte";
  import Alert from "./ui/Alert.svelte";
  import Sheet from "./ui/Sheet.svelte";
  import SchemeCard from "./ui/SchemeCard.svelte";
  import AccentPicker from "./ui/AccentPicker.svelte";
  import SliderRow from "./ui/SliderRow.svelte";
  import ToggleRow from "./ui/ToggleRow.svelte";
  import * as Select from "./ui/select/index.js";
  import { setAccent, setTransparency, setPrefs, applyBorder } from "../overrides.js";

  // Everything on this page goes through the commands the rest of ewe uses:
  // `ewe-theme scheme …` for schemes (it writes ewe.conf through ewe-conf,
  // whose hooks repaint the shell and the toolkits) and `ewe-conf set` for
  // the look presets, the bar and Glass. What the page shows is read back
  // from `ewe-theme show` (lib/theme.js), never from a copy kept here.
  $: input = ($theme && $theme.input) || {};
  $: glassPct = Math.round(Number(input.bar_opacity ?? 100));

  let busy = false;
  async function run(fn) {
    busy = true;
    try {
      return await fn();
    } catch (e) {
      errorMsg.set(String(e));
    } finally {
      busy = false;
    }
  }

  // ── schemes ────────────────────────────────────────────────────────────
  let schemes = []; // `scheme list`: built-ins first, then imported and wallpaper schemes
  let detail = {}; // slug → { roles, adjusted } from `scheme show`
  let cards = [];
  async function loadSchemes() {
    try {
      const r = await api.themeScheme("list");
      schemes = r.schemes || [];
    } catch {
      schemes = [];
    }
    // the previews and adjusted roles, one `show` each, side by side
    await Promise.all(
      schemes.map(async (s) => {
        try {
          const d = await api.themeScheme("show", s.slug);
          detail = { ...detail, [s.slug]: { roles: d.roles, adjusted: d.adjusted || [], palette: d.scheme?.palette } };
        } catch {}
      })
    );
  }
  onMount(() => {
    loadSchemes();
    api.blurAvailable().then((b) => (blurOk = b !== false)).catch(() => {});
  });

  /** A scheme verb, then everything that follows from it re-read. */
  async function scheme(...args) {
    return run(async () => {
      const r = await api.themeScheme(...args);
      await Promise.all([loadSchemes(), refreshTheme()]);
      return r;
    });
  }
  const apply = (s) => scheme("apply", s.slug).then((r) => r && flashApplied(`Applied **${s.name}**`));

  async function exportScheme(s) {
    const r = await run(() => api.themeScheme("export", s.slug));
    if (r && r.yaml) {
      await navigator.clipboard.writeText(r.yaml);
      toast(`Copied **${s.name}** as YAML. Paste it into a file to edit it.`, "success");
    }
  }
  async function duplicate(s) {
    const r = await scheme("duplicate", s.slug);
    const d = r && r.duplicated;
    if (d) toast(`Duplicated as **${d.name}**`, "success", 0, { label: "Apply", run: () => apply(d) });
  }
  let removing = null;
  async function remove() {
    const s = removing;
    removing = null;
    const r = await scheme("remove", s.slug);
    if (r) flashApplied(`Removed **${s.name}**`);
  }
  async function fromWallpaper(light = false) {
    const r = await scheme("from-wallpaper", "--apply", ...(light ? ["--light"] : []));
    if (r && r.scheme) flashApplied(`Applied **${r.scheme.name}** from the wallpaper`);
  }

  // Arrow keys move between scheme cards and apply (a radio group).
  function cardKey(e, i) {
    const d = e.key === "ArrowRight" || e.key === "ArrowDown" ? 1 : e.key === "ArrowLeft" || e.key === "ArrowUp" ? -1 : 0;
    if (!d || e.target !== cards[i]) return;
    e.preventDefault();
    const n = (i + d + schemes.length) % schemes.length;
    cards[n]?.focus();
    apply(schemes[n]);
  }
  $: currentIdx = schemes.findIndex((s) => s.current);

  // ── Import scheme: a Sheet with a file or a URL ─────────────────────────
  let importOpen = false;
  let importFile = "";
  let importUrl = "";
  let importName = "";
  let importFlavour = "";
  let importError = "";
  $: importSrc = importFile || importUrl.trim();
  $: isJson = /\.json($|\?)/i.test(importSrc);
  const FLAVOURS = [
    { value: " auto", label: "Automatic" },
    { value: "latte", label: "Latte (light)" },
    { value: "frappe", label: "Frappé" },
    { value: "macchiato", label: "Macchiato" },
    { value: "mocha", label: "Mocha" }
  ];
  function openImport() {
    importFile = importUrl = importName = importFlavour = importError = "";
    importOpen = true;
  }
  async function chooseFile() {
    const f = await openDialog({
      multiple: false,
      title: "Import a scheme",
      filters: [{ name: "Schemes (YAML, TOML, JSON)", extensions: ["yaml", "yml", "toml", "json"] }]
    });
    if (f) {
      importFile = typeof f === "string" ? f : f.path;
      importUrl = "";
    }
  }
  async function doImport() {
    importError = "";
    const args = ["import", importSrc];
    if (importName.trim()) args.push("--name", importName.trim());
    if (isJson && importFlavour && importFlavour !== " auto") args.push("--flavour", importFlavour);
    busy = true;
    try {
      const r = await api.themeScheme(...args);
      importOpen = false;
      await loadSchemes();
      const s = r && r.imported;
      if (s) toast(`Imported **${s.name}**`, "success", 0, { label: "Apply", run: () => apply(s) });
    } catch (e) {
      importError = String(e);
    }
    busy = false;
  }

  // ── Edit colors (the palette of a user scheme; kept from the old page) ──
  let editing = null; // scheme summary
  const ROLES = [
    ["base00", "Background"], ["base01", "Panels"], ["base02", "Cards and selection"], ["base03", "Lines and muted text"],
    ["base05", "Text"], ["base07", "Bright text"], ["base08", "Danger"], ["base0A", "Warning"], ["base0B", "Success"]
  ];
  let roleTimer;
  function editRole(key, hex) {
    clearTimeout(roleTimer);
    const slug = editing.slug;
    roleTimer = setTimeout(() => scheme("set", key, hex, "--slug", slug), 300);
  }

  // ── accent ─────────────────────────────────────────────────────────────
  $: currentScheme = schemes.find((s) => s.current) || null;
  $: accent = String(input.accent || (currentScheme && currentScheme.accent) || "#eeb407").toLowerCase();
  // the generator's moves on the accent roles, said in words (Accent picker)
  $: accentMoves = ((currentScheme && detail[currentScheme.slug]?.adjusted) || []).filter((a) =>
    ["accent-text", "focus-ring", "on-accent", "accent-hover", "accent-pressed", "glass-accent"].includes(a.role)
  );
  async function pickAccent(hex) {
    await run(async () => {
      // a built-in scheme wears ewe.conf's accent; a user scheme keeps its own
      if (!currentScheme || currentScheme.builtin) await setAccent(hex);
      else await api.themeScheme("set", "accent", hex);
      await Promise.all([loadSchemes(), refreshTheme()]);
    });
  }

  // ── look presets, bar and Glass: ewe.conf [desktop.theme] / [desktop.bar] ─
  const corners = [["none", "Square"], ["small", "Small"], ["medium", "Medium"], ["large", "Large"]];
  const densities = [["compact", "Compact"], ["comfortable", "Comfortable"], ["roomy", "Roomy"]];
  const strokes = [["none", "None"], ["thin", "Thin"], ["thick", "Thick"]];
  async function setConf(key, value) {
    await run(async () => {
      await api.setConf(key, value);
      await refreshTheme();
    });
  }
  // Bar opacity is written on release (a write rebuilds the tokens and reloads
  // Hyprland); the slider shows the value live meanwhile.
  let blurOk = true;
  const GLASS = 80; // the Glass preset (opacity-glass)
  $: truthy = (v) => v === true || String(v).toLowerCase() === "true";
</script>

<Page title="Appearance" desc="The scheme, the accent color and the look of controls, the bar and the dock.">
  <!-- ── Scheme ─────────────────────────────────────────────────────── -->
  <Group title="Scheme" well={false}>
    <svelte:fragment slot="action">
      <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" disabled={busy} on:click={() => fromWallpaper(false)}>
        <Icon name="image" />From wallpaper
      </button>
      <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" disabled={busy} on:click={openImport}>
        <Icon name="fileDown" />Import scheme…
      </button>
    </svelte:fragment>
    <div class="scheme-grid" role="radiogroup" aria-label="Scheme">
      {#each schemes as s, i (s.slug)}
        <SchemeCard
          bind:el={cards[i]}
          scheme={s}
          roles={detail[s.slug]?.roles}
          adjusted={detail[s.slug]?.adjusted}
          tabindex={i === (currentIdx < 0 ? 0 : currentIdx) ? 0 : -1}
          disabled={busy}
          onApply={() => apply(s)}
          onExport={() => exportScheme(s)}
          onDuplicate={() => duplicate(s)}
          onRemove={() => (removing = s)}
          onEdit={s.builtin ? null : () => (editing = s)}
          onLight={s.slug === "wallpaper" ? () => fromWallpaper(true) : null}
          onKey={(e) => cardKey(e, i)}
        />
      {/each}
    </div>
    <svelte:fragment slot="after">
      <p class="note">
        Every scheme is a palette. Duplicate one and export it to edit it in any text editor, then import
        it again. Imports read Base16 and Base24 YAML, Omarchy colors.toml, Catppuccin palette.json and Gogh.
      </p>
    </svelte:fragment>
  </Group>

  <!-- ── Accent color ───────────────────────────────────────────────── -->
  <Group title="Accent color">
    <Row
      sub={currentScheme && !currentScheme.builtin
        ? `Buttons, switches, the focus ring and window borders follow it. Changing it changes ${currentScheme.name}.`
        : "Buttons, switches, the focus ring and window borders follow it."}
      block
    >
      <AccentPicker presets={ACCENTS} value={accent} disabled={busy} onChange={pickAccent} />
    </Row>
    {#if accentMoves.length}
      <div class="p-1">
        <Alert tone="info">
          This color can't be used as it is: {accentMoves.map((a) => a.role).join(", ")}
          {accentMoves.length === 1 ? "was" : "were"} adjusted to stay readable.
        </Alert>
      </div>
    {/if}
  </Group>

  <!-- ── Look presets ───────────────────────────────────────────────── -->
  <Group title="Look" desc="Corners, spacing and outlines. They never change colors or text.">
    <Row title="Corners">
      <Seg label="Corners" options={corners} value={input.corner === "round" ? "large" : input.corner || "medium"} disabled={busy} picked={(v) => setConf("desktop.theme.corner", v)} />
    </Row>
    <Row title="Density" sub="How tall controls and rows are: 24, 28 or 32px.">
      <Seg label="Density" options={densities} value={input.density || "comfortable"} disabled={busy} picked={(v) => setConf("desktop.theme.density", v)} />
    </Row>
    <Row title="Outlines" sub="The line around controls, cards and panels. Fields always keep one.">
      <Seg label="Outlines" options={strokes} value={input.stroke || "thin"} disabled={busy} picked={(v) => setConf("desktop.theme.stroke", v)} />
    </Row>
  </Group>

  <!-- ── Bar and dock (Glass) ───────────────────────────────────────── -->
  <Group title="Bar and dock">
    <Row title="Bar size" sub="Normal is 48px tall; large is 64px, with bigger modules and icons.">
      <Seg label="Bar size" options={[["normal", "Normal"], ["large", "Large"]]} value={input.bar_size || "normal"} disabled={busy} picked={(v) => setConf("desktop.bar.size", v)} />
    </Row>
    <SliderRow
      label="Bar opacity"
      sub="How solid the bar, the dock and the lock screen card are. Below 100% the wallpaper shows through, blurred."
      value={glassPct}
      from={0}
      to={100}
      unit="%"
      dim={busy}
      moved={(v) => setConf("desktop.theme.bar_opacity", Math.round(v))}
    />
    <Row title="Glass" sub="The preset: 80%, the lowest opacity where text stays readable on any wallpaper.">
      <button
        class="ewe-btn ewe-btn--secondary ewe-btn--sm"
        disabled={busy || glassPct === GLASS}
        on:click={() => setConf("desktop.theme.bar_opacity", GLASS)}
      >
        {glassPct === GLASS ? "In use" : "Use Glass"}
      </button>
    </Row>
    {#if glassPct < GLASS || (!blurOk && glassPct < 90 && glassPct < 100)}
      <div class="p-1">
        {#if !blurOk && glassPct < 90}
          <Alert tone="warning" title="Text can be hard to read on bright wallpapers">
            This computer can't blur behind the bar, so the wallpaper stays sharp. Use 90% or more.
          </Alert>
        {:else}
          <Alert tone="warning" title="Text can be hard to read on bright wallpapers">
            Below 80%, the bar's text can lose contrast. 80% or more keeps it readable.
          </Alert>
        {/if}
      </div>
    {/if}
    <ToggleRow
      title="App blur"
      sub="Every window at 85% with the wallpaper blurred behind it. Fullscreen windows stay solid."
      dim={busy}
      on={truthy(input.app_blur)}
      toggled={() => setConf("desktop.theme.app_blur", !truthy(input.app_blur))}
    />
    <ToggleRow
      title="Window transparency"
      sub="Windows you're not using turn very slightly see-through."
      dim={busy}
      on={truthy(input.window_transparency)}
      toggled={(v) => run(async () => { await setTransparency(v); await refreshTheme(); })}
    />
  </Group>

  <!-- ── Windows and sound ──────────────────────────────────────────── -->
  <Group title="Windows and sound">
    <ToggleRow
      title="Tint window borders"
      sub="The active window's border follows the accent color."
      dim={busy}
      on={!!$prefs.tintBorders}
      toggled={(v) => run(async () => { await setPrefs({ tintBorders: v }); await applyBorder(); })}
    />
    <ToggleRow
      title="Event sounds"
      sub="Chimes for notifications, volume, screenshots and power events."
      dim={busy}
      on={$prefs.eventSounds !== false}
      toggled={(v) => run(() => setPrefs({ eventSounds: v }))}
    />
    <Row title="Animations" sub="Speed, character, curves and styles have their own page.">
      <button class="ewe-link" on:click={() => pane.set("animations")}>Open Animations<Icon name="caretRight" /></button>
    </Row>
  </Group>
</Page>

<!-- Import scheme (Sheet): a file or a URL; a Toast offers to apply it -->
<Sheet open={importOpen} title="Import scheme" onClose={() => (importOpen = false)}>
  <button type="button" class="ewe-drop import-drop" on:click={chooseFile}>
    <Icon name="fileDown" />
    <span class="ewe-drop__title">{importFile ? "Choose another file…" : "Choose a file…"}</span>
    <span class="ewe-drop__hint">YAML, TOML or JSON: Base16, Base24, Omarchy, Catppuccin or Gogh</span>
  </button>
  {#if importFile}
    <div class="ewe-file">
      <Icon name="fileDown" />
      <div class="ewe-file__body">
        <span class="ewe-file__name" title={importFile}>{importFile.split("/").pop()}</span>
        <span class="ewe-file__meta">{importFile}</span>
      </div>
      <button class="ewe-iconbtn ewe-iconbtn--ghost ewe-iconbtn--sm" aria-label="Clear the file" on:click={() => (importFile = "")}><Icon name="x" /></button>
    </div>
  {/if}
  <label class="ewe-field">
    <span class="ewe-field__label">Or a web address</span>
    <input class="ewe-input" type="url" placeholder="https://…/scheme.yaml" disabled={!!importFile} bind:value={importUrl} />
  </label>
  <label class="ewe-field">
    <span class="ewe-field__label">Name <span class="ewe-field__optional">optional</span></span>
    <input class="ewe-input" placeholder="The name in the file" bind:value={importName} />
  </label>
  {#if isJson}
    <div class="ewe-field">
      <span class="ewe-field__label">Catppuccin flavor</span>
      <Select.Root type="single" value={importFlavour || " auto"} onValueChange={(v) => (importFlavour = v)}>
        <Select.Trigger class="self-start select-trigger" aria-label="Catppuccin flavor">
          {FLAVOURS.find((f) => f.value === (importFlavour || " auto"))?.label}
        </Select.Trigger>
        <Select.Content>
          {#each FLAVOURS as f (f.value)}
            <Select.Item value={f.value} label={f.label} />
          {/each}
        </Select.Content>
      </Select.Root>
    </div>
  {/if}
  {#if importError}
    <Alert tone="danger" title="Couldn't import the scheme">{importError}</Alert>
  {/if}
  <svelte:fragment slot="footer">
    <button class="ewe-btn ewe-btn--secondary" on:click={() => (importOpen = false)}>Cancel</button>
    <button class="ewe-btn ewe-btn--primary" disabled={busy || !importSrc} on:click={doImport}>
      {busy ? "Importing…" : "Import scheme"}
    </button>
  </svelte:fragment>
</Sheet>

<!-- Edit colors: the palette of one user scheme, applied as you pick -->
<Sheet open={!!editing} title={editing ? `Colors of ${editing.name}` : ""} onClose={() => (editing = null)}>
  {#if editing}
    {@const pal = detail[editing.slug]?.palette || {}}
    <div class="palette-grid">
      {#each ROLES as [key, label] (key)}
        <label class="ewe-colorfield">
          <input
            type="color"
            class="ewe-swatch"
            value={pal[key] || "#000000"}
            on:input={(e) => editRole(key, e.currentTarget.value)}
            aria-label={label}
          />
          <span class="min-w-0 flex-1">
            <span class="block truncate">{label}</span>
            <span class="block font-mono text-xs text-dim">{key}</span>
          </span>
        </label>
      {/each}
    </div>
    <p class="note">
      Changes go into this scheme in ewe.conf and show at once. Its variant (dark or light) comes from
      the file; change it with <code>ewe-theme scheme set variant light</code>.
    </p>
  {/if}
</Sheet>

<!-- Remove a user scheme: irreversible, so a danger Dialog names it -->
<Dialog.Root open={!!removing} onOpenChange={(v) => !v && (removing = null)}>
  <Dialog.Portal>
    <Dialog.Overlay class="scrim" />
    <Dialog.Content class="ewe-dialog is-floating">
      <div class="ewe-dialog__head">
        <span class="ewe-dialog__icon ewe-dialog__icon--danger"><Icon name="trash" /></span>
        <div class="ewe-dialog__titles">
          <Dialog.Title class="ewe-dialog__title">Remove {removing?.name}?</Dialog.Title>
          <Dialog.Description class="ewe-dialog__desc">
            The scheme is deleted from ewe.conf on every computer that syncs it. Export it first to keep a copy.
            {#if removing?.current}The desktop switches to Ewe Dark.{/if}
          </Dialog.Description>
        </div>
      </div>
      <div class="ewe-dialog__foot">
        <Dialog.Close class="ewe-btn ewe-btn--secondary">Cancel</Dialog.Close>
        <button class="ewe-btn ewe-btn--danger" on:click={remove}>Remove {removing?.name}</button>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
