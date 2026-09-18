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
  import Page from "./ui/Page.svelte";
  import Group from "./ui/Group.svelte";
  import Row from "./ui/Row.svelte";
  import Seg from "./ui/Seg.svelte";
</script>

<Page title="Animations" desc="How windows, workspaces and panels move.">
  {#if $anim}
    <Group>
      <ToggleRow
        title="Animations"
        sub="Windows, workspaces, fades and panels."
        on={allOn}
        dim={busy}
        toggled={(v) => run(() => setEnabled(v))}
      />
      <Row title="Speed" sub="Paces the shell and every duration below." dim={!allOn}>
        <Seg
          label="Animation speed"
          options={speeds.map((s) => [s.value, s.label])}
          value={speeds.find((s) => Math.abs(mult - s.value) < 0.01)?.value ?? ""}
          disabled={busy || !allOn}
          picked={(v) => run(() => setAnimationSpeed(v))}
        />
      </Row>
    </Group>

    <Group title="Character" well={false}>
      <div class="choice-grid" role="radiogroup" aria-label="Animation character">
        {#each ANIM_PRESETS as p (p.id)}
          <button
            class="ewe-card ewe-card--compact ewe-card--interactive text-left"
            class:is-selected={activePreset === p.id}
            role="radio"
            aria-checked={activePreset === p.id}
            disabled={busy || !allOn}
            on:click={() => run(() => applyAnim(animStateFromPreset(p), p.name + " applied"))}
          >
            <span class="ewe-card__title">{p.name}</span>
            <span class="ewe-card__desc">{p.sub}</span>
          </button>
        {/each}
      </div>
      <svelte:fragment slot="after">
        {#if allOn && !activePreset}
          <p class="note">Custom: your own mix from the controls below.</p>
        {/if}
      </svelte:fragment>
    </Group>

    <Group title="Fine-tune" well={false}>
      {#each ANIM_LEAVES as d (d.leaf)}
        {@const a = $anim.anims[d.leaf]}
        <div class="ewe-list">
          <ToggleRow
            title={d.label}
            on={!!a.on}
            dim={busy || !allOn}
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
              dim={busy || !allOn}
              moved={(v) => run(() => patchLeaf(d.leaf, { ms: v }))}
            />
            <SelectRow
              label="Curve"
              options={curveOpts}
              value={a.curve}
              dim={busy || !allOn}
              picked={(v) => run(() => patchLeaf(d.leaf, { curve: v }))}
            />
            {#if d.styles}
              <SelectRow
                label="Style"
                options={styleOpts(d.styles)}
                value={a.style || ""}
                dim={busy || !allOn}
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
                dim={busy || !allOn}
                moved={(v) => run(() => patchLeaf(d.leaf, { pct: v }))}
              />
            {/if}
          {/if}
        </div>
      {/each}
      <svelte:fragment slot="after">
        <p class="note">Every change applies at once. Open a window or switch workspaces to see it.</p>
      </svelte:fragment>
    </Group>
  {/if}
</Page>
