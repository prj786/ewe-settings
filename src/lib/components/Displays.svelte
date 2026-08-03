<script>
  import { onMount, onDestroy } from "svelte";
  import * as api from "../api.js";
  import {
    snapshot, specFromMonitor, keyFor, luaArgs, monitorsLuaText,
    modeMapFor, modeRes, specW, specH, verifyAgainst
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
  /** connector → can the hardware do adaptive sync at all (drm vrr_capable). */
  let vrrCaps = {};

  // confirm-or-revert state (risky changes: mode/scale/rotate/mirror/enable/move)
  let revertSpecs = null;
  let revertLeft = 0;
  let countdown;

  $: specs = snapshot(monitors, profiles);
  $: activeSpecs = specs.filter((s) => !s.disabled);

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
    api.vrrCaps().then((c) => (vrrCaps = c || {})).catch(() => {});
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
  async function riskyApply(list) {
    const prev = specs.map((s) => ({ ...s }));
    const ok = await applySpecs(list);
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
  const riskyChange = (name, patch) => riskyApply(specsWith(name, patch));

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

  /** Safe (VRR / bit depth): apply, then VERIFY it actually took before
   *  persisting. Hyprland silently ignores what the hardware can't do, and
   *  saying "Applied" for a no-op is how toggles end up "not working". */
  async function directChange(name, patch) {
    const prev = specs.map((s) => ({ ...s }));
    const next = specsWith(name, patch);
    if (!(await applySpecs(next))) return;
    await new Promise((r) => setTimeout(r, 700));
    await refresh();
    const m = monitors.find((x) => x.name === name);
    const live = m ? specFromMonitor(m) : null;
    const took = live && Object.keys(patch).every((k) => String(live[k]) === String(patch[k]));
    if (!took) {
      await applySpecs(prev); // withdraw the rule so config and reality agree
      errorMsg.set(`${name}: the display did not accept this change (unsupported by the hardware)`);
      setTimeout(refresh, 600);
      return;
    }
    await commit(next);
    flashApplied();
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

  // ── arrangement canvas (drag a display to choose its side) ────────────────
  const CH = 200, PAD = 14;
  let cw = 620;              // canvas width, tracks the card
  let dragging = null;       // { name, frame, px0, py0, x0, y0, w, h, moved }
  let dragXY = null;         // live logical position of the dragged output

  $: arr = arrLayout(activeSpecs, dragging, dragXY, cw);

  function frameFor(rects, width) {
    const minX = Math.min(...rects.map((r) => r.x));
    const minY = Math.min(...rects.map((r) => r.y));
    const maxX = Math.max(...rects.map((r) => r.x + r.w));
    const maxY = Math.max(...rects.map((r) => r.y + r.h));
    const k = Math.min(
      (width - 2 * PAD) / Math.max(1, maxX - minX),
      (CH - 2 * PAD) / Math.max(1, maxY - minY),
      0.12
    );
    return {
      k, minX, minY, maxX, maxY,
      ox: PAD + ((width - 2 * PAD) - (maxX - minX) * k) / 2 - minX * k,
      oy: PAD + ((CH - 2 * PAD) - (maxY - minY) * k) / 2 - minY * k
    };
  }

  function arrLayout(active, drag, dxy, width) {
    if (active.length < 2) return { frame: null, rects: [] };
    const rects = active.map((s) => {
      const live = drag && drag.name === s.name && dxy;
      return { s, x: live ? dxy.x : s.x, y: live ? dxy.y : s.y, w: specW(s), h: specH(s) };
    });
    // the frame is frozen for the duration of a drag so the canvas never
    // rescales under the pointer
    const frame = drag ? drag.frame : frameFor(rects, width);
    for (const r of rects) {
      r.px = frame.ox + r.x * frame.k;
      r.py = frame.oy + r.y * frame.k;
      r.pw = Math.max(26, r.w * frame.k);
      r.ph = Math.max(20, r.h * frame.k);
    }
    return { frame, rects };
  }

  function startDrag(e, r) {
    if (activeSpecs.length < 2 || revertSpecs) return;
    dragging = {
      name: r.s.name, frame: arr.frame,
      px0: e.clientX, py0: e.clientY, x0: r.x, y0: r.y, w: r.w, h: r.h, moved: false
    };
    dragXY = { x: r.x, y: r.y };
    e.currentTarget.setPointerCapture(e.pointerId);
  }

  function moveDrag(e) {
    if (!dragging) return;
    const f = dragging.frame;
    let x = dragging.x0 + (e.clientX - dragging.px0) / f.k;
    let y = dragging.y0 + (e.clientY - dragging.py0) / f.k;
    x = Math.max(f.minX - dragging.w, Math.min(x, f.maxX));
    y = Math.max(f.minY - dragging.h, Math.min(y, f.maxY));
    if (Math.abs(x - dragging.x0) + Math.abs(y - dragging.y0) > 12) dragging.moved = true;
    dragXY = { x, y };
  }

  /** Keep the dropped position but clamp so the two spans still overlap, and
   *  magnetise to exactly-aligned edges when close. */
  function alignCross(v, o, olen, dlen) {
    const need = Math.min(120, olen, dlen);
    let a = Math.max(o - dlen + need, Math.min(v, o + olen - need));
    if (Math.abs(a - o) < 80) a = o;
    else if (Math.abs(a + dlen - (o + olen)) < 80) a = o + olen - dlen;
    return a;
  }

  /** Snap the dropped rect edge-to-edge against the nearest neighbour — a
   *  layout with a gap between displays would trap the cursor at the seam. */
  function snapRect(d, statics) {
    let best = null;
    for (const r of statics) {
      const cands = [
        { x: r.x - d.w, y: alignCross(d.y, r.y, r.h, d.h) },
        { x: r.x + r.w, y: alignCross(d.y, r.y, r.h, d.h) },
        { y: r.y - d.h, x: alignCross(d.x, r.x, r.w, d.w) },
        { y: r.y + r.h, x: alignCross(d.x, r.x, r.w, d.w) }
      ];
      for (const c of cands) {
        const dist = (c.x - d.x) ** 2 + (c.y - d.y) ** 2;
        if (!best || dist < best.dist) best = { x: c.x, y: c.y, dist };
      }
    }
    return best
      ? { x: Math.round(best.x), y: Math.round(best.y) }
      : { x: Math.round(d.x), y: Math.round(d.y) };
  }

  async function endDrag() {
    if (!dragging) return;
    const d = dragging, xy = dragXY;
    dragging = null;
    dragXY = null;
    if (!d.moved || !xy) return;
    const statics = activeSpecs
      .filter((s) => s.name !== d.name)
      .map((s) => ({ x: s.x, y: s.y, w: specW(s), h: specH(s) }));
    const snapped = snapRect({ x: xy.x, y: xy.y, w: d.w, h: d.h }, statics);
    const next = specs.map((s) =>
      s.name === d.name ? { ...s, x: snapped.x, y: snapped.y } : { ...s }
    );
    // normalise to a 0,0 origin, same convention as auto-arrange
    const act = next.filter((s) => !s.disabled);
    const nx = Math.min(...act.map((s) => s.x));
    const ny = Math.min(...act.map((s) => s.y));
    for (const s of act) {
      s.x -= nx;
      s.y -= ny;
    }
    const cur = specs.find((s) => s.name === d.name);
    const moved = next.find((s) => s.name === d.name);
    if (cur && moved.x === cur.x && moved.y === cur.y) return;
    await riskyApply(next);
  }

  const rotations = [
    { label: "Normal", value: 0 },
    { label: "90°", value: 1 },
    { label: "180°", value: 2 },
    { label: "270°", value: 3 }
  ];
  const scales = [1, 1.25, 1.5, 1.6, 1.75, 2, 2.5].map((s) => ({ label: `${Math.round(s * 100)}%`, value: s }));
</script>

<svelte:window on:pointermove={moveDrag} on:pointerup={endDrag} on:pointercancel={endDrag} />

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

  {#if loaded && activeSpecs.length > 1}
    <section>
      <div class="section-title">Arrangement</div>
      <Card>
        <div class="px-4 pt-3 text-xs text-zinc-400 dark:text-zinc-500">
          Drag a display to choose which side it sits on. Displays snap edge to edge.
        </div>
        <div class="px-4 pb-3" bind:clientWidth={cw}>
          <div class="relative touch-none select-none" style={`height:${CH}px`}>
            {#each arr.rects as r (r.s.name)}
              <div
                role="button"
                tabindex="-1"
                class="absolute flex flex-col items-center justify-center overflow-hidden rounded-lg border-2 bg-zinc-100 transition-shadow dark:bg-zinc-800
                       {dragging && dragging.name === r.s.name ? 'z-10 cursor-grabbing shadow-xl' : 'cursor-grab'}
                       {r.s.primary ? 'border-[var(--accent)]' : 'border-zinc-300 dark:border-zinc-600'}"
                style={`left:${r.px}px;top:${r.py}px;width:${r.pw}px;height:${r.ph}px`}
                on:pointerdown={(e) => startDrag(e, r)}
              >
                <span class="pointer-events-none max-w-full truncate px-1 text-xs font-medium">{r.s.name}</span>
                <span class="pointer-events-none text-[10px] text-zinc-400 dark:text-zinc-500">{modeRes(r.s.mode)}</span>
              </div>
            {/each}
          </div>
        </div>
      </Card>
    </section>
  {/if}

  {#each specs as s (s.name)}
    {@const mon = monitors.find((m) => m.name === s.name)}
    {@const mm = modeMapFor(mon, s)}
    {@const curRes = modeRes(s.mode)}
    {@const vrrOk = vrrCaps[s.name] !== false}
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
            sub={vrrOk ? "VRR / Adaptive sync." : "This display's hardware does not support adaptive sync."}
            dim={!vrrOk}
            on={s.vrr}
            toggled={() => vrrOk && directChange(s.name, { vrr: !s.vrr })}
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
