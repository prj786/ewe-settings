<script>
  import { onMount, onDestroy } from "svelte";
  import * as api from "../api.js";
  import {
    snapshot, specFromMonitor, keyFor, luaArgs, monitorsLuaText,
    modeMapFor, modeRes, specW, verifyAgainst
  } from "../hypr.js";
  import { errorMsg, flashApplied } from "../stores.js";
  import Card from "./ui/Card.svelte";
  import SelectRow from "./ui/SelectRow.svelte";
  import ToggleRow from "./ui/ToggleRow.svelte";
  import KV from "./ui/KV.svelte";

  let monitors = [];
  let profiles = {};
  let lastKey = "";
  let loaded = false;

  // confirm-or-revert state (risky changes: mode/scale/rotate/mirror/enable)
  let revertSpecs = null;
  let revertLeft = 0;
  let countdown;

  $: specs = snapshot(monitors, profiles);

  async function refresh() {
    try {
      monitors = await api.monitors();
    } catch (e) {
      errorMsg.set(String(e));
    }
  }

  async function loadProfiles() {
    try {
      const j = JSON.parse((await api.readConfig("quickshell/display-profiles.json")) || "{}");
      if (j && j.profiles) profiles = j.profiles;
      if (j && j.lastKey) lastKey = j.lastKey;
    } catch {}
  }

  onMount(async () => {
    await Promise.all([refresh(), loadProfiles()]);
    loaded = true;
  });
  onDestroy(() => clearInterval(countdown));

  const isVirtual = (list) => (list || []).some((m) => /^(WAYLAND|HEADLESS)-/.test(m.name || ""));

  async function applySpecs(list) {
    // last-ditch guard, same as HyprMon: never disable every output
    if (list.every((s) => s.disabled)) {
      errorMsg.set("Refused to apply: profile would disable every display");
      return false;
    }
    try {
      await api.runEvals(list.map((s) => `hl.monitor(${luaArgs(s)})`));
      return true;
    } catch (e) {
      errorMsg.set(String(e));
      return false;
    }
  }

  /** Save profile + regenerate monitors.lua, then let the shell re-read it so
   *  its own hotplug/AC re-assert logic never acts on a stale copy. */
  async function commit(list) {
    if (isVirtual(monitors)) return; // a nested/test session must never persist
    const key = keyFor(list);
    profiles = { ...profiles, [key]: list };
    lastKey = key;
    await api.writeConfig(
      "quickshell/display-profiles.json",
      JSON.stringify({ version: 1, lastKey, profiles }, null, 2)
    );
    await api.writeConfig("hypr/generated/monitors.lua", monitorsLuaText(profiles, lastKey));
    await api.pokeShell();
  }

  function specsWith(name, patch) {
    return specs.map((s) => (s.name === name ? { ...s, ...patch } : { ...s }));
  }

  /** Risky: apply live, then 10 s to confirm — timeout restores the previous
   *  known-good state, so a blacked-out screen always comes back by itself. */
  async function riskyChange(name, patch) {
    const prev = specs.map((s) => ({ ...s }));
    const ok = await applySpecs(specsWith(name, patch));
    if (!ok) {
      await applySpecs(prev);
      return;
    }
    revertSpecs = prev;
    revertLeft = 10;
    clearInterval(countdown);
    countdown = setInterval(() => {
      revertLeft--;
      if (revertLeft <= 0) doRevert();
    }, 1000);
    setTimeout(refresh, 600);
  }

  async function keepChange() {
    clearInterval(countdown);
    revertSpecs = null;
    // commit what is actually live (re-queried after the apply) rather than the
    // requested specs — the compositor may have picked a close-enough mode
    await refresh();
    await commit(snapshot(monitors, profiles));
    flashApplied("Saved");
  }

  async function doRevert() {
    clearInterval(countdown);
    const s = revertSpecs;
    revertSpecs = null;
    if (s) {
      await applySpecs(s);
      setTimeout(refresh, 600);
    }
  }

  /** Safe (VRR / bit depth): apply + persist straight away. */
  async function directChange(name, patch) {
    const next = specsWith(name, patch);
    if (await applySpecs(next)) {
      await commit(next);
      flashApplied();
      setTimeout(refresh, 600);
    }
  }

  async function setPrimary(name) {
    const next = specs.map((s) => ({ ...s, primary: s.name === name }));
    await commit(next);
    flashApplied("Primary display saved");
  }

  async function autoArrange() {
    const all = specs.map((s) => ({ ...s }));
    const active = all.filter((s) => !s.disabled);
    active.sort((a, b) => b.primary - a.primary || a.x - b.x);
    let x = 0;
    for (const s of active) {
      s.x = x;
      s.y = 0;
      x += specW(s);
    }
    if (await applySpecs(all)) {
      await commit(all);
      flashApplied("Layout saved");
      setTimeout(refresh, 600);
    }
  }

  /** Escape hatch when a screen goes dark: dpms on + re-apply saved profile. */
  async function resetDisplays() {
    await api.dpmsOn();
    await refresh();
    const prof = profiles[keyFor((monitors || []).map(specFromMonitor))];
    if (prof) await applySpecs(prof);
    await api.wallpaperReapply();
    flashApplied("Displays reset");
    setTimeout(refresh, 800);
  }

  const rotations = [
    { label: "Normal", value: 0 },
    { label: "90°", value: 1 },
    { label: "180°", value: 2 },
    { label: "270°", value: 3 }
  ];
  const scales = [1, 1.25, 1.5, 1.6, 1.75, 2, 2.5].map((s) => ({ label: `${Math.round(s * 100)}%`, value: s }));
</script>

<div class="mx-auto max-w-3xl space-y-6 p-5 sm:p-8">
  <div class="flex flex-wrap items-center justify-between gap-2">
    <h1 class="text-lg font-semibold">Displays</h1>
    <div class="flex gap-2">
      <button class="btn-ghost text-xs" on:click={autoArrange}>Auto-arrange</button>
      <button class="btn-ghost text-xs" title="Wake sleeping outputs and re-apply the saved profile" on:click={resetDisplays}>
        Reset displays
      </button>
    </div>
  </div>

  {#if !loaded}
    <p class="text-sm text-zinc-400">Reading monitors…</p>
  {:else if specs.length === 0}
    <p class="text-sm text-zinc-400">No monitors reported. Is the shell session running?</p>
  {/if}

  {#each specs as s (s.name)}
    {@const mon = monitors.find((m) => m.name === s.name)}
    {@const mm = modeMapFor(mon, s)}
    {@const curRes = modeRes(s.mode)}
    <section>
      <div class="section-title">
        {s.name}
        {#if s.desc}<span class="ml-2 font-normal normal-case tracking-normal text-zinc-400">{s.desc}</span>{/if}
      </div>
      <Card>
        {#if s.disabled}
          <div class="flex items-center justify-between px-4 py-3">
            <span class="text-sm text-zinc-400">Disabled</span>
            <button class="btn-primary !py-1 text-xs" on:click={() => riskyChange(s.name, { disabled: false })}>
              Enable
            </button>
          </div>
        {:else}
          <SelectRow
            label="Resolution"
            options={mm.resList.map((r) => ({ label: r.replace("x", " × "), value: r }))}
            value={curRes}
            picked={(v) => {
              const hzs = mm.byRes[v] || [];
              if (hzs.length) riskyChange(s.name, { mode: hzs[0].mode });
            }}
          />
          <SelectRow
            label="Refresh rate"
            options={(mm.byRes[curRes] || []).map((o) => ({ label: o.label, value: o.mode }))}
            value={s.mode}
            picked={(v) => riskyChange(s.name, { mode: v })}
          />
          <SelectRow
            label="Scale"
            options={scales.some((o) => Math.abs(o.value - s.scale) < 0.001)
              ? scales
              : [{ label: `${Math.round(s.scale * 100)}%`, value: s.scale }, ...scales]}
            value={s.scale}
            picked={(v) => riskyChange(s.name, { scale: Number(v) })}
          />
          <SelectRow
            label="Rotation"
            options={rotations}
            value={s.transform}
            picked={(v) => riskyChange(s.name, { transform: Number(v) })}
          />
          {#if specs.length > 1}
            <SelectRow
              label="Mirror"
              sub="Show another display's image on this one."
              options={[{ label: "Off", value: "" }, ...specs.filter((o) => o.name !== s.name && !o.disabled).map((o) => ({ label: o.name, value: o.name }))]}
              value={s.mirror}
              picked={(v) => riskyChange(s.name, { mirror: v })}
            />
          {/if}
          <ToggleRow
            title="Variable refresh rate"
            sub="VRR / Adaptive sync."
            on={s.vrr}
            toggled={() => directChange(s.name, { vrr: !s.vrr })}
          />
          <ToggleRow
            title="10-bit colour"
            sub="Higher banding-free gradients; some apps misbehave."
            on={s.bitdepth === 10}
            toggled={() => directChange(s.name, { bitdepth: s.bitdepth === 10 ? 8 : 10 })}
          />
          <div class="flex items-center justify-between px-4 py-3">
            <div>
              <div class="text-sm font-medium">Primary display</div>
              <div class="text-xs text-zinc-400 dark:text-zinc-500">Anchors auto-arrange at 0,0.</div>
            </div>
            {#if s.primary}
              <span class="text-xs font-medium" style="color: var(--accent)">Primary</span>
            {:else}
              <button class="btn-ghost !py-1 text-xs" on:click={() => setPrimary(s.name)}>Make primary</button>
            {/if}
          </div>
          <KV k="Position" v={`${s.x}, ${s.y}`} />
          {#if specs.length > 1}
            <div class="flex items-center justify-between px-4 py-3">
              <span class="text-sm text-zinc-400">Turn off this display</span>
              <button class="btn-ghost !py-1 text-xs" on:click={() => riskyChange(s.name, { disabled: true })}>
                Disable
              </button>
            </div>
          {/if}
        {/if}
      </Card>
    </section>
  {/each}

  <p class="text-xs text-zinc-400 dark:text-zinc-500">
    One profile is saved per set of connected displays and re-applied automatically at boot,
    on hotplug and when docking. Risky changes revert by themselves unless you confirm.
  </p>
</div>

{#if revertSpecs}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-6 backdrop-blur-sm">
    <div class="card w-full max-w-sm !bg-white p-6 text-center shadow-2xl dark:!bg-zinc-900">
      <h2 class="text-base font-semibold">Keep these display settings?</h2>
      <p class="mt-1 text-sm text-zinc-400">
        Reverting to the previous configuration in {revertLeft}&nbsp;s.
      </p>
      <div class="mt-5 flex gap-2">
        <button class="btn-ghost flex-1" on:click={doRevert}>Revert</button>
        <button class="btn-primary flex-1" on:click={keepChange}>Keep</button>
      </div>
    </div>
  </div>
{/if}
