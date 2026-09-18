<script>
  import { onMount, onDestroy } from "svelte";
  import * as api from "../api.js";
  import {
    snapshot, specFromMonitor, keyFor, luaArgs, monitorsLuaText,
    modeMapFor, modeRes, specW, specH, verifyAgainst, scaleOptions, nearestValidScale
  } from "../hypr.js";
  import { errorMsg, flashApplied } from "../stores.js";
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
  // a new resolution may not divide by the current scale — snap the way
  // Hyprland would, so what we ask for is what comes back
  function pickResolution(s, res, hzs) {
    if (!hzs.length) return;
    const [w, h] = res.split("x").map(Number);
    riskyChange(s.name, { mode: hzs[0].mode, scale: nearestValidScale(w, h, s.scale) });
  }
  import Page from "./ui/Page.svelte";
  import Group from "./ui/Group.svelte";
  import Row from "./ui/Row.svelte";
  import Icon from "./ui/Icon.svelte";
  import { Dialog } from "bits-ui";
</script>

<svelte:window on:pointermove={moveDrag} on:pointerup={endDrag} on:pointercancel={endDrag} />

<Page title="Displays" desc="Resolution, refresh rate, scale and where each display sits.">
  <svelte:fragment slot="actions">
    <button class="ewe-btn ewe-btn--secondary" on:click={autoArrange}>Arrange side by side</button>
    <button class="ewe-btn ewe-btn--ghost" title="Wake sleeping displays and apply the saved profile again" on:click={resetDisplays}>
      Reset displays
    </button>
  </svelte:fragment>

  {#if !loaded}
    <p class="note">Reading displays…</p>
  {:else if specs.length === 0}
    <div class="ewe-list">
      <div class="ewe-empty">
        <span class="ewe-empty__icon"><Icon name="monitor" /></span>
        <div class="ewe-empty__title">No displays reported</div>
        <div class="ewe-empty__desc">Hyprland didn't list any. Check that the desktop session is running.</div>
      </div>
    </div>
  {/if}

  {#if loaded && activeSpecs.length > 1}
    <Group title="Arrangement">
      <Row sub="Drag a display to the side it sits on. Displays snap edge to edge." />
      <div class="arrange" bind:clientWidth={cw}>
        <div class="relative touch-none select-none" style={`height:${CH}px`}>
          {#each arr.rects as r (r.s.name)}
            <div
              role="button"
              tabindex="-1"
              aria-label="{r.s.name}, {modeRes(r.s.mode)}"
              class="arrange__screen"
              class:is-primary={r.s.primary}
              class:is-dragging={dragging && dragging.name === r.s.name}
              style={`left:${r.px}px;top:${r.py}px;width:${r.pw}px;height:${r.ph}px`}
              on:pointerdown={(e) => startDrag(e, r)}
            >
              <span class="arrange__name">{r.s.name}</span>
              <span class="arrange__res">{modeRes(r.s.mode)}</span>
            </div>
          {/each}
        </div>
      </div>
    </Group>
  {/if}

  {#each specs as s (s.name)}
    {@const mon = monitors.find((m) => m.name === s.name)}
    {@const mm = modeMapFor(mon, s)}
    {@const curRes = modeRes(s.mode)}
    {@const vrrOk = vrrCaps[s.name] !== false}
    <Group title={s.desc ? `${s.name} · ${s.desc}` : s.name}>
      {#if s.disabled}
        <Row title="Turned off">
          <button class="ewe-btn ewe-btn--primary ewe-btn--sm" on:click={() => riskyChange(s.name, { disabled: false })}>
            Turn on
          </button>
        </Row>
      {:else}
        <SelectRow
          label="Resolution"
          options={mm.resList.map((r) => ({ label: r.replace("x", " × "), value: r }))}
          value={curRes}
          picked={(v) => pickResolution(s, v, mm.byRes[v] || [])}
        />
        <SelectRow
          label="Refresh rate"
          options={(mm.byRes[curRes] || []).map((o) => ({ label: o.label, value: o.mode }))}
          value={s.mode}
          picked={(v) => riskyChange(s.name, { mode: v })}
        />
        <SelectRow
          label="Scale"
          sub="Only the scales this resolution divides by evenly. Hyprland refuses the rest."
          options={scaleOptions(curRes, s.scale)}
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
            sub="Show another display's picture on this one."
            options={[{ label: "Off", value: "" }, ...specs.filter((o) => o.name !== s.name && !o.disabled).map((o) => ({ label: o.name, value: o.name }))]}
            value={s.mirror}
            picked={(v) => riskyChange(s.name, { mirror: v })}
          />
        {/if}
        <ToggleRow
          title="Variable refresh rate"
          sub={vrrOk ? "Adaptive sync (VRR)." : "This display doesn't support adaptive sync."}
          dim={!vrrOk}
          on={s.vrr}
          toggled={() => vrrOk && directChange(s.name, { vrr: !s.vrr })}
        />
        <ToggleRow
          title="10-bit color"
          sub="Smoother gradients without banding. Some apps don't handle it well."
          on={s.bitdepth === 10}
          toggled={() => directChange(s.name, { bitdepth: s.bitdepth === 10 ? 8 : 10 })}
        />
        <Row title="Primary display" sub="Arranging starts from it, at the top left.">
          {#if s.primary}
            <span class="ewe-badge ewe-badge--accent"><span class="ewe-badge__label">Primary</span></span>
          {:else}
            <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" on:click={() => setPrimary(s.name)}>Make primary</button>
          {/if}
        </Row>
        <KV k="Position" v={`${s.x}, ${s.y}`} mono />
        {#if specs.length > 1}
          <Row title="Turn off this display">
            <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" on:click={() => riskyChange(s.name, { disabled: true })}>
              Turn off
            </button>
          </Row>
        {/if}
      {/if}
    </Group>
  {/each}

  <p class="note">
    One profile is saved for each set of connected displays and applied again at start-up, when you
    plug a display in and when you dock. Risky changes undo themselves unless you keep them.
  </p>
</Page>

<!-- Confirm-or-revert (Dialog card): the question is the decision, the
     primary action repeats its verb, and doing nothing (Esc, the timer)
     brings the previous settings back. -->
<Dialog.Root open={!!revertSpecs} onOpenChange={(v) => !v && revertSpecs && doRevert()}>
  <Dialog.Portal>
    <Dialog.Overlay class="scrim" />
    <Dialog.Content class="ewe-dialog is-floating" interactOutsideBehavior="ignore">
      <div class="ewe-dialog__head">
        <span class="ewe-dialog__icon"><Icon name="monitor" /></span>
        <div class="ewe-dialog__titles">
          <Dialog.Title class="ewe-dialog__title">Keep these display settings?</Dialog.Title>
          <Dialog.Description class="ewe-dialog__desc">
            The previous settings come back in <span class="font-mono tabular-nums">{revertLeft}</span>&nbsp;s.
          </Dialog.Description>
        </div>
      </div>
      <div class="ewe-dialog__foot">
        <button class="ewe-btn ewe-btn--secondary" on:click={doRevert}>Revert</button>
        <button class="ewe-btn ewe-btn--primary" on:click={keepChange}>Keep settings</button>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
