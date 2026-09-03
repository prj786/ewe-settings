<script>
  import { onMount } from "svelte";
  import { prefs } from "../stores.js";
  import {
    ANIM_CURVES, ANIM_LEAVES, ANIM_PRESETS, ANIM_STYLE_LABELS,
    animPresetMatch, animStateFromPreset
  } from "../hypr.js";
  import { anim, loadAnim, applyAnim, patchLeaf } from "../animations.js";
  import { setAnimationSpeed } from "../overrides.js";
  import ToggleRow from "./ui/ToggleRow.svelte";
  import SliderRow from "./ui/SliderRow.svelte";
  import SelectRow from "./ui/SelectRow.svelte";

  // Off lives in the master toggle; the pills only pace what is on. The same
  // multiplier drives the QML shell, so "Fast" here is fast everywhere.
  const speeds = [
    { label: "Fast", value: 2 },
    { label: "Normal", value: 1 },
    { label: "Slow", value: 0.6 }
  ];

  const curveOpts = ANIM_CURVES.map((c) => ({ label: c.label, value: c.id }));
  const styleOpts = (styles) => styles.map((s) => ({ label: ANIM_STYLE_LABELS[s], value: s }));

  let busy = false;
  async function run(fn) {
    busy = true;
    try { await fn(); } catch (e) { console.error(e); }
    busy = false;
  }

  $: mult = Number($prefs.animationSpeed ?? 1);
  $: allOn = !!($anim && $anim.enabled && mult > 0);
  $: activePreset = allOn && $anim ? animPresetMatch($anim) : "";

  async function setEnabled(v) {
    // Coming back from off restores a sane multiplier: the in-shell fallback's
    // "Off" is animationSpeed = 0, which would keep everything frozen here.
    if (v && mult <= 0) await setAnimationSpeed(1);
    await applyAnim({ ...$anim, enabled: v }, v ? "Animations on" : "Animations off");
  }

  onMount(loadAnim);
</script>

<div class="mx-auto max-w-3xl space-y-6 p-5 sm:p-8">
  <h1 class="text-lg font-semibold">Animations</h1>

  {#if $anim}
    <section>
      <div class="card divide-y divide-hairline">
        <ToggleRow
          title="Animations"
          sub="Windows, workspaces, fades and panels."
          on={allOn}
          dim={busy}
          toggled={(v) => run(() => setEnabled(v))}
        />
        <div class="flex flex-wrap items-center justify-between gap-x-3 gap-y-1.5 px-4 py-3 {allOn ? '' : 'pointer-events-none opacity-50'}">
          <div>
            <div class="text-sm font-medium">Speed</div>
            <div class="text-xs text-dim dark:text-dim">Paces the shell and every duration below.</div>
          </div>
          <div class="flex flex-wrap gap-1.5">
            {#each speeds as s (s.value)}
              <button
                class="rounded-full px-3 py-1 text-xs font-medium transition-colors
                  {Math.abs(mult - s.value) < 0.01
                  ? 'text-white'
                  : 'bg-elevated/70 text-dim hover:bg-hover /60 '}"
                style={Math.abs(mult - s.value) < 0.01 ? "background: var(--accent)" : ""}
                disabled={busy}
                on:click={() => run(() => setAnimationSpeed(s.value))}
              >
                {s.label}
              </button>
            {/each}
          </div>
        </div>
      </div>
    </section>

    <section class={allOn ? "" : "pointer-events-none opacity-50"}>
      <div class="section-title">Character</div>
      <div class="grid grid-cols-2 gap-2.5">
        {#each ANIM_PRESETS as p (p.id)}
          <button
            class="card p-3 text-left transition-colors
              {activePreset === p.id
              ? '!border-[var(--accent)] ring-1 ring-[var(--accent)]'
              : 'hover:bg-elevated dark:hover:bg-elevated/60'}"
            disabled={busy}
            on:click={() => run(() => applyAnim(animStateFromPreset(p), p.name + " applied"))}
          >
            <span class="block text-sm font-medium">{p.name}</span>
            <span class="block text-xs text-dim dark:text-dim">{p.sub}</span>
          </button>
        {/each}
      </div>
      {#if allOn && !activePreset}
        <p class="mt-2 text-xs text-dim dark:text-dim">Custom — your own mix from the controls below.</p>
      {/if}
    </section>

    <section class={allOn ? "" : "pointer-events-none opacity-50"}>
      <div class="section-title">Fine-tune</div>
      <div class="space-y-3">
        {#each ANIM_LEAVES as d (d.leaf)}
          {@const a = $anim.anims[d.leaf]}
          <div class="card divide-y divide-hairline">
            <ToggleRow
              title={d.label}
              on={!!a.on}
              dim={busy}
              toggled={(v) => run(() => patchLeaf(d.leaf, { on: v }))}
            />
            {#if a.on}
              <SliderRow
                label="Duration"
                value={a.ms}
                from={80}
                to={800}
                step={20}
                unit=" ms"
                dim={busy}
                moved={(v) => run(() => patchLeaf(d.leaf, { ms: v }))}
              />
              <SelectRow
                label="Curve"
                options={curveOpts}
                value={a.curve}
                dim={busy}
                picked={(v) => run(() => patchLeaf(d.leaf, { curve: v }))}
              />
              {#if d.styles}
                <SelectRow
                  label="Style"
                  options={styleOpts(d.styles)}
                  value={a.style || ""}
                  dim={busy}
                  picked={(v) => run(() => patchLeaf(d.leaf, { style: v }))}
                />
              {/if}
              {#if (a.style || "") === "popin"}
                <SliderRow
                  label="Pop-in scale"
                  value={a.pct ?? 88}
                  from={60}
                  to={98}
                  step={1}
                  unit="%"
                  dim={busy}
                  moved={(v) => run(() => patchLeaf(d.leaf, { pct: v }))}
                />
              {/if}
            {/if}
          </div>
        {/each}
      </div>
      <p class="mt-3 text-xs text-dim dark:text-dim">
        Every change applies instantly — open a window or switch a workspace to feel it.
      </p>
    </section>
  {/if}
</div>
