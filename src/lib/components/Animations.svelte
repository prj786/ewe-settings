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
    { label: "Slow", value: 0.6 },
    { label: "Normal", value: 1 },
    { label: "Fast", value: 2 }
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

  // Fine-tune: one row per animation, its settings folded under it. The
  // row says what it is set to ("280 ms · Snap · Pop in") while folded.
  let open = {};
  const curveName = (id) => ANIM_CURVES.find((c) => c.id === id)?.label || id || "";
  const summary = (d, a) =>
    !a || !a.on
      ? "Off"
      : [`${a.ms} ms`, curveName(a.curve), d.styles ? ANIM_STYLE_LABELS[a.style || ""] : ""].filter(Boolean).join(" · ");
  import Page from "./ui/Page.svelte";
  import Group from "./ui/Group.svelte";
  import Row from "./ui/Row.svelte";
  import Seg from "./ui/Seg.svelte";
  import Toggle from "./ui/Toggle.svelte";
  import Icon from "./ui/Icon.svelte";
</script>

<Page title="Animations" desc="How windows, workspaces and panels move.">
  {#if $anim}
    <Group title="Overall">
      <ToggleRow
        title="Animations"
        sub="Windows, workspaces, fades and panels"
        on={allOn}
        dim={busy}
        toggled={(v) => run(() => setEnabled(v))}
      />
      <Row title="Speed" sub="Paces the whole desktop, and every duration below" dim={!allOn}>
        <Seg
          label="Animation speed"
          options={speeds.map((s) => [s.value, s.label])}
          value={speeds.find((s) => Math.abs(mult - s.value) < 0.01)?.value ?? ""}
          disabled={busy || !allOn}
          picked={(v) => run(() => setAnimationSpeed(v))}
        />
      </Row>
    </Group>

    <Group
      title="Character"
      desc={allOn && !activePreset ? "Custom: your own mix from Fine-tune below." : ""}
      well={false}
    >
      <div class="choice-grid" role="radiogroup" aria-label="Animation character" data-setting="Character">
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
    </Group>

    <Group title="Fine-tune" desc="Changes apply at once. Open a window or switch workspaces to see them.">
      {#each ANIM_LEAVES as d (d.leaf)}
        {@const a = $anim.anims[d.leaf]}
        <Row title={d.label} sub={summary(d, a)} dim={busy || !allOn}>
          <Toggle on={!!a.on} disabled={busy || !allOn} label={d.label} toggled={(v) => run(() => patchLeaf(d.leaf, { on: v }))} />
          <button
            type="button"
            class="ewe-iconbtn ewe-iconbtn--ghost ewe-iconbtn--sm"
            aria-expanded={!!open[d.leaf]}
            aria-label="{open[d.leaf] ? 'Hide' : 'Show'} the settings for {d.label}"
            title={open[d.leaf] ? "Hide settings" : "Show settings"}
            disabled={!a.on || !allOn}
            on:click={() => (open = { ...open, [d.leaf]: !open[d.leaf] })}
          >
            <Icon name={open[d.leaf] && a.on ? "caretUp" : "caretDown"} />
          </button>
        </Row>
        {#if open[d.leaf] && a.on && allOn}
          <SliderRow
            label="Duration"
            value={a.ms}
            from={80}
            to={800}
            step={20}
            unit=" ms"
            indent
            dim={busy}
            moved={(v) => run(() => patchLeaf(d.leaf, { ms: v }))}
          />
          <SelectRow
            label="Curve"
            options={curveOpts}
            value={a.curve}
            indent
            dim={busy}
            picked={(v) => run(() => patchLeaf(d.leaf, { curve: v }))}
          />
          {#if d.styles}
            <SelectRow
              label="Style"
              options={styleOpts(d.styles)}
              value={a.style || ""}
              indent
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
              indent
              dim={busy}
              moved={(v) => run(() => patchLeaf(d.leaf, { pct: v }))}
            />
          {/if}
        {/if}
      {/each}
    </Group>
  {:else}
    <Group title="Overall"><Row sub="Reading the animations…" /></Group>
  {/if}
</Page>
