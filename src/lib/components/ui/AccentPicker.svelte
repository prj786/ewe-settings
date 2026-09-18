<script>
  /**
   * Accent picker (design/system/components/AccentPicker): the presets as
   * 28px swatches in a radio group, the selected one with a check in black
   * or white (whichever contrasts) and a double ring, and a custom swatch
   * that opens the system color chooser. Arrow keys move between swatches.
   */
  import Icon from "./Icon.svelte";
  import { accentDefault } from "../../stores.js";
  export let presets = []; // [{ name, hex, ink }] from ewe-theme show
  export let value = "";
  export let disabled = false;
  export let onChange = () => {};

  $: cur = String(value || "").toLowerCase();
  $: isPreset = presets.some((p) => p.hex === cur);
  // relative luminance → the ink with more contrast on this swatch
  function ink(hex) {
    const h = String(hex).replace("#", "");
    if (h.length < 6) return "var(--black)";
    const lin = (c) => {
      c /= 255;
      return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
    };
    const [r, g, b] = [0, 2, 4].map((i) => lin(parseInt(h.slice(i, i + 2), 16)));
    const L = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    return (L + 0.05) / 0.05 > 1.05 / (L + 0.05) ? "var(--black)" : "var(--neutral-0)";
  }
  let items = [];
  function key(e, i) {
    const d = e.key === "ArrowRight" || e.key === "ArrowDown" ? 1 : e.key === "ArrowLeft" || e.key === "ArrowUp" ? -1 : 0;
    if (!d) return;
    e.preventDefault();
    const n = (i + d + presets.length) % presets.length;
    items[n]?.focus();
    onChange(presets[n].hex);
  }
</script>

<div class="ewe-swatches" role="radiogroup" aria-label="Accent color">
  {#each presets as p, i (p.hex)}
    <button
      bind:this={items[i]}
      type="button"
      class="ewe-swatch"
      class:is-selected={cur === p.hex}
      role="radio"
      aria-checked={cur === p.hex}
      aria-label={p.name}
      title={p.name}
      tabindex={cur === p.hex || (!isPreset && i === 0) ? 0 : -1}
      style="--swatch: {p.hex}; --swatch-ink: {p.ink || ink(p.hex)}"
      {disabled}
      on:click={() => cur !== p.hex && onChange(p.hex)}
      on:keydown={(e) => key(e, i)}
    >
      {#if cur === p.hex}<Icon name="check" />{/if}
    </button>
  {/each}
  <!-- a custom color: the swatch is the system color chooser's button -->
  <label
    class="ewe-swatch ewe-swatch--custom"
    class:is-selected={!isPreset && cur}
    style={!isPreset && cur ? `--swatch: ${cur}; background: ${cur}; border-style: solid; color: ${ink(cur)}` : ""}
    title={!isPreset && cur ? `Custom ${cur}` : "Pick any color"}
  >
    <Icon name={!isPreset && cur ? "check" : "plus"} />
    <input
      type="color"
      class="sr-only"
      value={cur || accentDefault()}
      {disabled}
      aria-label={!isPreset && cur ? `Custom color ${cur}` : "Pick any color"}
      on:change={(e) => onChange(e.currentTarget.value)}
    />
  </label>
  <span class="accent-hex" aria-hidden="true">{cur}</span>
</div>
