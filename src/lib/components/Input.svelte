<script>
  import { onMount } from "svelte";
  import * as api from "../api.js";
  import {
    INPUT_DEFAULTS, INPUT_OPTIONS, parseInputBlocks, inputLua, deviceLua,
    kbActiveList, isTouchpadName, KB_PRESETS, KB_VARIANTS, GRP_OPTIONS
  } from "../hypr.js";
  import { prefs, errorMsg, flashApplied } from "../stores.js";
  import { setCursor } from "../overrides.js";
  import ToggleRow from "./ui/ToggleRow.svelte";
  import SelectRow from "./ui/SelectRow.svelte";
  import SliderRow from "./ui/SliderRow.svelte";
  import IconBtn from "./ui/IconBtn.svelte";
  import * as Select from "./ui/select/index.js";

  // "" is a real XKB variant (= the layout's default), but bits-ui reads ""
  // as "nothing selected" — so it travels under a sentinel, same as SelectRow.
  const V_EMPTY = " empty";
  const vEnc = (v) => (v === "" || v == null ? V_EMPTY : String(v));
  const vDec = (v) => (v === V_EMPTY ? "" : v);

  // the sizes the Mocu cursor draws (ewe-conf refuses others); 60 and 96
  // are listed only when set from the command line, so they still show
  const CURSOR_SIZES = [
    { label: "Default", value: 24 },
    { label: "Large", value: 36 },
    { label: "Larger", value: 48 },
    { label: "Largest", value: 72 }
  ];
  $: curSize = Number($prefs.cursorSize ?? 24);
  $: sizeOptions = CURSOR_SIZES.some((o) => o.value === curSize)
    ? CURSOR_SIZES
    : [...CURSOR_SIZES, { label: `${curSize} pixels`, value: curSize }].sort((a, b) => a.value - b.value);

  const icUp = '<path d="m18 15-6-6-6 6"/>';
  const icDown = '<path d="m6 9 6 6 6-6"/>';
  const icX = '<path d="M18 6 6 18"/><path d="m6 6 12 12"/>';

  let inp = { ...INPUT_DEFAULTS };
  let loaded = false;
  let mice = [];
  let hasTouchpad = false;
  let perWindowKb = false;
  let devOverrides = {}; // device name → { sensitivity, natural_scroll, … }
  let devTarget = ""; // "" = all pointing devices (global input{})
  let addQuery = "";
  let showAdd = false;
  // full xkb registry from the system ([{c, n, variants: [{c, n}]}]);
  // null until loaded — the curated KB_PRESETS/KB_VARIANTS fill in meanwhile
  let registry = null;

  onMount(async () => {
    // the per-device overrides FIRST: every write sends `devices` along, and
    // sending it before this file was read wiped every override (2026-09-20)
    try {
      const j = JSON.parse((await api.readConfig("quickshell/input-devices.json")) || "{}");
      if (j && typeof j === "object") devOverrides = j;
    } catch {}
    try {
      const batch = INPUT_OPTIONS.map((o) => "getoption input:" + o[0]).join("; ");
      const text = await api.hyprctl(["--batch", batch]);
      inp = parseInputBlocks(text, inp);
      loaded = true;
      // Nothing is written here. Opening this page used to persist the live
      // values at once, which rewrote ewe.conf [desktop.input] and input.lua
      // (and asked for a sync) just from looking. The first real change
      // sends the whole object anyway, so a first run still ends up complete.
    } catch (e) {
      errorMsg.set(String(e));
    }
    try {
      const d = await api.devices();
      mice = (d.mice || []).map((x) => x.name);
      hasTouchpad = mice.some(isTouchpadName);
    } catch {}
    try {
      perWindowKb = await api.perWindowKb("status");
    } catch {}
    try {
      const r = await api.xkbRegistry();
      if (Array.isArray(r) && r.length) registry = r;
    } catch {} // no registry file → curated fallback arrays
  });

  // RFC-001 Phase 4 (0.6): input.lua + input-devices.json are ewe-conf build
  // artifacts — one `set desktop.input` derives both; inputLua/deviceLua
  // remain only for the live hyprctl eval.
  function writeInputFile() {
    api.setConf("desktop.input", { ...inp, devices: devOverrides }).catch((e) => errorMsg.set(String(e)));
  }

  async function applyInput(patch) {
    inp = { ...inp, ...patch };
    try {
      await api.runEvals([inputLua(inp)]);
      flashApplied();
    } catch (e) {
      errorMsg.set(String(e));
    }
    writeInputFile();
  }

  async function applyDevice(name, patch) {
    const o = { ...(devOverrides[name] || {}), ...patch };
    devOverrides = { ...devOverrides, [name]: o };
    try {
      await api.runEvals([deviceLua(name, o)]);
      flashApplied();
    } catch (e) {
      errorMsg.set(String(e));
    }
    writeInputFile();
  }

  // ── keyboard layouts (kb_layout/kb_variant are parallel lists) ────────────
  $: kbActive = kbActiveList(inp);

  async function applyLayouts(list) {
    if (!list.length) return;
    const anyVar = list.some((l) => l.variant !== "");
    await applyInput({
      kb_layout: list.map((l) => l.code).join(","),
      kb_variant: anyVar ? list.map((l) => l.variant).join(",") : ""
    });
    // the per-window daemon snapshots the layout list at startup
    api.perWindowKb("restart").then((v) => (perWindowKb = v)).catch(() => {});
  }

  const kbAdd = (code) => {
    showAdd = false;
    addQuery = "";
    applyLayouts([...kbActive, { code, variant: "" }]);
  };
  const kbRemove = (i) => {
    if (kbActive.length > 1) applyLayouts(kbActive.filter((_, j) => j !== i));
  };
  const kbMove = (from, to) => {
    if (to < 0 || to >= kbActive.length) return;
    const l = kbActive.slice();
    const [it] = l.splice(from, 1);
    l.splice(to, 0, it);
    applyLayouts(l);
  };
  const kbSetVariant = (i, v) => {
    const l = kbActive.slice();
    l[i] = { code: l[i].code, variant: v };
    applyLayouts(l);
  };
  $: presets = registry || KB_PRESETS;
  $: nameOf = (code) => presets.find((p) => p.c === code)?.n || code;
  $: variantOpts = (code) => {
    const entry = registry && registry.find((p) => p.c === code);
    if (entry)
      return [
        { label: "Default", value: "" },
        ...entry.variants.map((v) => ({ label: v.n, value: v.c }))
      ];
    return (KB_VARIANTS[code] || [""]).map((v) => ({ label: v === "" ? "Default" : v, value: v }));
  };
  $: addCandidates = presets.filter(
    (p) =>
      !kbActive.some((l) => l.code === p.c) &&
      (addQuery.trim() === "" || p.n.toLowerCase().includes(addQuery.trim().toLowerCase()) || p.c.includes(addQuery.trim().toLowerCase()))
  );

  // kb_options managed as a token set (grp:* switch shortcut)
  function kbOptToken(prefix) {
    return String(inp.kb_options || "").split(",").find((t) => t.indexOf(prefix) === 0) || "";
  }
  function setKbOptPrefix(prefix, token) {
    const toks = String(inp.kb_options || "").split(",").filter((t) => t !== "" && t.indexOf(prefix) !== 0);
    if (token !== "") toks.push(token);
    applyInput({ kb_options: toks.join(",") });
  }

  async function setPerWindowKb(on) {
    try {
      perWindowKb = await api.perWindowKb(on ? "on" : "off");
    } catch (e) {
      errorMsg.set(String(e));
    }
  }

  // ── mouse: global vs per-device ───────────────────────────────────────────
  $: pointerMice = mice.filter((n) => !isTouchpadName(n));
  $: effSens = devTarget === "" ? Number(inp.sensitivity) : Number(devOverrides[devTarget]?.sensitivity ?? inp.sensitivity);
  function setMouse(patch) {
    if (devTarget === "") applyInput(patch);
    else applyDevice(devTarget, patch);
  }
  import Page from "./ui/Page.svelte";
  import Group from "./ui/Group.svelte";
  import Row from "./ui/Row.svelte";
  import Icon from "./ui/Icon.svelte";
  import PickList from "./ui/PickList.svelte";
  import Sheet from "./ui/Sheet.svelte";
</script>

<Page title="Keyboard and mouse" desc="Keyboard layouts and typing, the mouse and the touchpad.">
  {#if !loaded}
    <Group title="Keyboard layouts"><Row sub="Reading the input settings…" /></Group>
  {:else}
    <Group title="Keyboard layouts" desc="The first one is used when you sign in.">
      <svelte:fragment slot="action">
        <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" on:click={() => ((addQuery = ""), (showAdd = true))}>
          <Icon name="plus" />Add layout…
        </button>
      </svelte:fragment>
      {#each kbActive as l, i (l.code + i)}
        <div class="ewe-row" data-setting={i === 0 ? "Keyboard layouts" : undefined}>
          <span class="ewe-badge layout-code"><span class="ewe-badge__label">{l.code.toUpperCase()}</span></span>
          <div class="ewe-row__text"><div class="ewe-row__title">{nameOf(l.code)}</div></div>
          <div class="ewe-row__trail">
            <Select.Root
              type="single"
              value={vEnc(l.variant)}
              onValueChange={(raw) => kbSetVariant(i, vDec(raw))}
            >
              <Select.Trigger size="sm" class="select-trigger" aria-label="Variant of {nameOf(l.code)}">
                {variantOpts(l.code).find((v) => v.value === l.variant)?.label || l.variant || "Default"}
              </Select.Trigger>
              <Select.Content>
                {#each variantOpts(l.code) as v (vEnc(v.value))}
                  <Select.Item value={vEnc(v.value)} label={v.label} />
                {/each}
              </Select.Content>
            </Select.Root>
            <span class="iconbtn-row">
              <IconBtn name="arrowUp" title="Move {nameOf(l.code)} up" disabled={i === 0} go={() => kbMove(i, i - 1)} />
              <IconBtn name="arrowDown" title="Move {nameOf(l.code)} down" disabled={i === kbActive.length - 1} go={() => kbMove(i, i + 1)} />
              <IconBtn name="x" title="Remove {nameOf(l.code)}" danger disabled={kbActive.length <= 1} go={() => kbRemove(i)} />
            </span>
          </div>
        </div>
      {/each}
    </Group>

    <Group title="Typing">
      <SelectRow
        label="Switch layouts with"
        sub="Super+Space always works too"
        options={GRP_OPTIONS}
        value={kbOptToken("grp:")}
        picked={(v) => setKbOptPrefix("grp:", v)}
      />
      <ToggleRow
        title="Remember the layout for each window"
        sub="Every window keeps its own keyboard layout"
        on={perWindowKb}
        toggled={() => setPerWindowKb(!perWindowKb)}
      />
      <ToggleRow
        title="Num Lock on at start"
        on={!!inp.numlock_by_default}
        toggled={() => applyInput({ numlock_by_default: !inp.numlock_by_default })}
      />
      <SliderRow label="Key repeat rate" sub="Characters per second while a key is held" value={Number(inp.repeat_rate)} from={5} to={80} unit="/s" moved={(v) => applyInput({ repeat_rate: Math.round(v) })} />
      <SliderRow label="Repeat delay" sub="How long to hold a key before it repeats" value={Number(inp.repeat_delay)} from={150} to={1000} step={10} unit=" ms" moved={(v) => applyInput({ repeat_delay: Math.round(v) })} />
    </Group>

    <Group title="Mouse">
      {#if pointerMice.length > 1}
        <SelectRow
          label="Configure"
          sub="Settings for one device override the ones for all"
          options={[{ label: "All pointing devices", value: "" }, ...pointerMice.map((n) => ({ label: n, value: n }))]}
          value={devTarget}
          picked={(v) => (devTarget = v)}
        />
      {/if}
      <SliderRow label="Pointer speed" value={effSens} from={-1} to={1} step={0.05} moved={(v) => setMouse({ sensitivity: Math.round(v * 100) / 100 })} />
      <SelectRow
        label="Acceleration"
        options={[
          { label: "Adaptive", value: "" },
          { label: "Flat", value: "flat" }
        ]}
        sub="Flat moves the pointer the same distance however fast you move"
        value={devTarget === "" ? inp.accel_profile : (devOverrides[devTarget]?.accel_profile ?? "")}
        picked={(v) => setMouse({ accel_profile: v })}
      />
      <ToggleRow
        title="Natural scrolling"
        sub="The content follows your fingers, like on a phone"
        on={devTarget === "" ? !!inp.natural_scroll : !!(devOverrides[devTarget]?.natural_scroll ?? inp.natural_scroll)}
        toggled={() =>
          setMouse({
            natural_scroll: devTarget === "" ? !inp.natural_scroll : !(devOverrides[devTarget]?.natural_scroll ?? inp.natural_scroll)
          })}
      />
      <ToggleRow
        title="Left-handed buttons"
        on={devTarget === "" ? !!inp.left_handed : !!(devOverrides[devTarget]?.left_handed ?? inp.left_handed)}
        toggled={() =>
          setMouse({
            left_handed: devTarget === "" ? !inp.left_handed : !(devOverrides[devTarget]?.left_handed ?? inp.left_handed)
          })}
      />
      <SliderRow label="Scroll speed" value={Number(inp.scroll_factor)} from={0.1} to={3} step={0.1} moved={(v) => applyInput({ scroll_factor: Math.round(v * 10) / 10 })} />
    </Group>

    <Group title="Cursor" desc="Apps that were already open use the new cursor after they restart.">
      <SelectRow
        label="Cursor size"
        sub="A bigger pointer is easier to find on a large or busy screen"
        options={sizeOptions}
        value={curSize}
        picked={(v) => setCursor({ cursorSize: Number(v) })}
      />
      <SelectRow
        label="Cursor color"
        sub="Automatic is white with a dark scheme and black with a light one"
        options={[
          { label: "Automatic", value: "auto" },
          { label: "White", value: "white" },
          { label: "Black", value: "black" }
        ]}
        value={$prefs.cursorColor || "auto"}
        picked={(v) => setCursor({ cursorColor: v })}
      />
    </Group>

    {#if hasTouchpad}
      <Group title="Touchpad">
        <ToggleRow title="Tap to click" on={!!inp.tp_tap} toggled={() => applyInput({ tp_tap: !inp.tp_tap })} />
        <ToggleRow title="Natural scrolling" sub="The content follows your fingers" on={!!inp.tp_natural_scroll} toggled={() => applyInput({ tp_natural_scroll: !inp.tp_natural_scroll })} />
        <ToggleRow title="Turn off while typing" on={!!inp.tp_dwt} toggled={() => applyInput({ tp_dwt: !inp.tp_dwt })} />
        <ToggleRow
          title="Two-finger right-click"
          sub="Two fingers click right and three click middle, instead of corner zones"
          on={!!inp.tp_clickfinger}
          toggled={() => applyInput({ tp_clickfinger: !inp.tp_clickfinger })}
        />
        <ToggleRow title="Middle-button emulation" sub="Click left and right together for a middle click" on={!!inp.tp_mbe} toggled={() => applyInput({ tp_mbe: !inp.tp_mbe })} />
        <ToggleRow title="Tap and drag" on={!!inp.tp_tap_drag} toggled={() => applyInput({ tp_tap_drag: !inp.tp_tap_drag })} />
        <ToggleRow title="Drag lock" sub="Lifting your finger mid-drag doesn't drop what you're dragging" on={!!inp.tp_drag_lock} toggled={() => applyInput({ tp_drag_lock: !inp.tp_drag_lock })} />
        <SliderRow label="Scroll speed" value={Number(inp.tp_scroll_factor)} from={0.1} to={3} step={0.1} moved={(v) => applyInput({ tp_scroll_factor: Math.round(v * 10) / 10 })} />
      </Group>
    {/if}
  {/if}
</Page>

<!-- Add a keyboard layout: pick it in a Sheet -->
<Sheet open={showAdd} title="Add a keyboard layout" onClose={() => (showAdd = false)}>
  <PickList
    bind:query={addQuery}
    placeholder="Search layouts"
    items={addCandidates.slice(0, 60).map((p) => ({ id: p.c, title: p.n, trail: p.c.toUpperCase(), p }))}
    pick={(it) => kbAdd(it.p.c)}
  />
</Sheet>
