<script>
  import { onMount } from "svelte";
  import * as api from "../api.js";
  import {
    INPUT_DEFAULTS, INPUT_OPTIONS, parseInputBlocks, inputLua, deviceLua,
    kbActiveList, isTouchpadName, KB_PRESETS, KB_VARIANTS, GRP_OPTIONS
  } from "../hypr.js";
  import { errorMsg, flashApplied } from "../stores.js";
  import Card from "./ui/Card.svelte";
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
    try {
      const batch = INPUT_OPTIONS.map((o) => "getoption input:" + o[0]).join("; ");
      const text = await api.hyprctl(["--batch", batch]);
      inp = parseInputBlocks(text, inp);
      loaded = true;
      // persist the live truth right away: covers first run and migration
      writeInputFile();
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
      const j = JSON.parse((await api.readConfig("quickshell/input-devices.json")) || "{}");
      if (j && typeof j === "object") devOverrides = j;
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
</script>

<div class="mx-auto max-w-3xl space-y-6 p-5 sm:p-8">
  <h1 class="text-lg font-semibold">Keyboard & Mouse</h1>

  {#if !loaded}
    <p class="text-sm text-dim">Reading input configuration…</p>
  {:else}
    <section>
      <div class="section-title">Keyboard layouts</div>
      <Card>
        {#each kbActive as l, i (l.code + i)}
          <div class="flex flex-wrap items-center gap-2 px-4 py-2.5">
            <span class="w-10 shrink-0 rounded bg-elevated/70 px-1.5 py-0.5 text-center font-mono text-[11px] uppercase /60">{l.code}</span>
            <span class="min-w-0 flex-1 truncate text-sm">{nameOf(l.code)}</span>
            <Select.Root
              type="single"
              value={vEnc(l.variant)}
              onValueChange={(raw) => kbSetVariant(i, vDec(raw))}
            >
              <Select.Trigger size="sm" class="min-w-24 text-xs">
                <span data-slot="select-value" class="truncate">{l.variant || "Default"}</span>
              </Select.Trigger>
              <Select.Content class="max-h-72 p-1">
                {#each variantOpts(l.code) as v (vEnc(v.value))}
                  <Select.Item value={vEnc(v.value)} label={v.label} />
                {/each}
              </Select.Content>
            </Select.Root>
            <div class="flex items-center gap-0.5">
              <IconBtn icon={icUp} title="Move up" disabled={i === 0} go={() => kbMove(i, i - 1)} />
              <IconBtn icon={icDown} title="Move down" disabled={i === kbActive.length - 1} go={() => kbMove(i, i + 1)} />
              <IconBtn icon={icX} title="Remove" danger disabled={kbActive.length <= 1} go={() => kbRemove(i)} />
            </div>
          </div>
        {/each}
        <div class="px-4 py-3">
          {#if showAdd}
            <input class="input mb-2" placeholder="Search layouts…" bind:value={addQuery} />
            <div class="max-h-48 overflow-y-auto rounded-lg border border-hairline">
              {#each addCandidates.slice(0, 30) as p (p.c)}
                <button class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-sm hover:bg-black/5 dark:hover:bg-white/5" on:click={() => kbAdd(p.c)}>
                  <span class="w-10 rounded bg-elevated/70 px-1.5 py-0.5 text-center font-mono text-[11px] uppercase /60">{p.c}</span>
                  {p.n}
                </button>
              {/each}
            </div>
            <button class="btn-ghost mt-2 !py-1 text-xs" on:click={() => (showAdd = false)}>Cancel</button>
          {:else}
            <button class="btn-ghost !py-1 text-xs" on:click={() => (showAdd = true)}>+ Add layout</button>
          {/if}
        </div>
      </Card>
    </section>

    <section>
      <div class="section-title">Switching & typing</div>
      <Card>
        <SelectRow
          label="Layout switch shortcut"
          sub="Super+Space always works in addition."
          options={GRP_OPTIONS}
          value={kbOptToken("grp:")}
          picked={(v) => setKbOptPrefix("grp:", v)}
        />
        <ToggleRow
          title="Remember layout per window"
          sub="Each window keeps its own keyboard layout."
          on={perWindowKb}
          toggled={() => setPerWindowKb(!perWindowKb)}
        />
        <ToggleRow
          title="Numlock on by default"
          on={!!inp.numlock_by_default}
          toggled={() => applyInput({ numlock_by_default: !inp.numlock_by_default })}
        />
        <SliderRow label="Key repeat rate" value={Number(inp.repeat_rate)} from={5} to={80} unit="/s" moved={(v) => applyInput({ repeat_rate: Math.round(v) })} />
        <SliderRow label="Repeat delay" value={Number(inp.repeat_delay)} from={150} to={1000} step={10} unit=" ms" moved={(v) => applyInput({ repeat_delay: Math.round(v) })} />
      </Card>
    </section>

    <section>
      <div class="section-title">Mouse</div>
      <Card>
        {#if pointerMice.length > 1}
          <SelectRow
            label="Configure"
            sub="Per-device settings override the global ones."
            options={[{ label: "All pointing devices", value: "" }, ...pointerMice.map((n) => ({ label: n, value: n }))]}
            value={devTarget}
            picked={(v) => (devTarget = v)}
          />
        {/if}
        <SliderRow label="Pointer speed" value={effSens} from={-1} to={1} step={0.05} moved={(v) => setMouse({ sensitivity: Math.round(v * 100) / 100 })} />
        <SelectRow
          label="Acceleration"
          options={[
            { label: "Adaptive (default)", value: "" },
            { label: "Flat (no acceleration)", value: "flat" }
          ]}
          value={devTarget === "" ? inp.accel_profile : (devOverrides[devTarget]?.accel_profile ?? "")}
          picked={(v) => setMouse({ accel_profile: v })}
        />
        <ToggleRow
          title="Natural scrolling"
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
      </Card>
    </section>

    {#if hasTouchpad}
      <section>
        <div class="section-title">Touchpad</div>
        <Card>
          <ToggleRow title="Tap to click" on={!!inp.tp_tap} toggled={() => applyInput({ tp_tap: !inp.tp_tap })} />
          <ToggleRow title="Natural scrolling" on={!!inp.tp_natural_scroll} toggled={() => applyInput({ tp_natural_scroll: !inp.tp_natural_scroll })} />
          <ToggleRow title="Disable while typing" on={!!inp.tp_dwt} toggled={() => applyInput({ tp_dwt: !inp.tp_dwt })} />
          <ToggleRow
            title="Two-finger right-click"
            sub="Clickfinger: two fingers = right, three = middle (instead of edge zones)."
            on={!!inp.tp_clickfinger}
            toggled={() => applyInput({ tp_clickfinger: !inp.tp_clickfinger })}
          />
          <ToggleRow title="Middle-button emulation" on={!!inp.tp_mbe} toggled={() => applyInput({ tp_mbe: !inp.tp_mbe })} />
          <ToggleRow title="Tap and drag" on={!!inp.tp_tap_drag} toggled={() => applyInput({ tp_tap_drag: !inp.tp_tap_drag })} />
          <ToggleRow title="Drag lock" on={!!inp.tp_drag_lock} toggled={() => applyInput({ tp_drag_lock: !inp.tp_drag_lock })} />
          <SliderRow label="Scroll speed" value={Number(inp.tp_scroll_factor)} from={0.1} to={3} step={0.1} moved={(v) => applyInput({ tp_scroll_factor: Math.round(v * 10) / 10 })} />
        </Card>
      </section>
    {/if}
  {/if}
</div>
