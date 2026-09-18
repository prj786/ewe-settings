<script>
  /**
   * Scheme card (design/system/components/SchemeCard): a miniature desktop
   * drawn in the scheme's own roles — base, bar, a window with text lines and
   * an accent button, a popup — its name, and dots for its accent, success
   * and danger. A radio in the Appearance scheme group; clicking applies it.
   * Selected: check; built in: lock; every card has the overflow menu
   * (Apply, Export, Duplicate; Remove on user schemes). An Inline alert names
   * the roles the generator moved to keep contrast.
   */
  import * as Menu from "./dropdown-menu/index.js";
  import Icon from "./Icon.svelte";
  export let scheme; // summary from `scheme list`: slug, name, variant, builtin, current, accent, swatch
  export let roles = null; // roles from `scheme show`, when loaded
  export let adjusted = []; // [{role, …}] from `scheme show`
  export let disabled = false;
  export let tabindex = -1;
  export let el = null;
  export let onApply = () => {};
  export let onExport = () => {};
  export let onDuplicate = () => {};
  export let onRemove = () => {};
  export let onEdit = null; // user schemes: the palette's colors (the old Edit palette)
  export let onLight = null; // the wallpaper scheme: a light version
  export let onKey = () => {};

  // roles, or the list's five swatches until `show` answers
  $: sw = scheme.swatch || [];
  $: r = roles || {
    "surface-base": sw[0], "surface-raised": sw[1], "surface-overlay": sw[1], "border-subtle": sw[1],
    "text-primary": sw[2], "text-disabled": sw[2], accent: sw[3], success: sw[3], danger: sw[4]
  };
  $: vars = [
    ["--p-base", r["surface-base"]], ["--p-raised", r["surface-raised"]], ["--p-overlay", r["surface-overlay"]],
    ["--p-border", r["border-subtle"]], ["--p-text", r["text-primary"]], ["--p-muted", r["text-disabled"]],
    ["--p-accent", r.accent]
  ].filter(([, v]) => v).map(([k, v]) => `${k}:${v}`).join(";");
  $: moved = [...new Set((adjusted || []).map((a) => a.role))];
</script>

<div
  bind:this={el}
  class="ewe-scheme"
  class:is-selected={scheme.current}
  class:is-busy={disabled}
  role="radio"
  aria-checked={scheme.current}
  aria-label="{scheme.name}, {scheme.variant}"
  {tabindex}
  style={vars}
  on:click={() => !disabled && !scheme.current && onApply()}
  on:keydown={(e) => {
    if ((e.key === "Enter" || e.key === " ") && e.target === el) {
      e.preventDefault();
      if (!disabled && !scheme.current) onApply();
    } else onKey(e);
  }}
>
  <div class="ewe-scheme__preview" aria-hidden="true">
    <div class="ewe-scheme__bar"><i></i><i></i><i></i></div>
    <div class="ewe-scheme__win"><b></b><s></s><s style="width: 55%"></s><u></u></div>
    <div class="ewe-scheme__pop"></div>
  </div>
  <div class="ewe-scheme__meta">
    <span class="ewe-scheme__name">{scheme.name}</span>
    <span class="ewe-scheme__dots" aria-hidden="true">
      <i style="--c: {r.accent || scheme.accent}"></i><i style="--c: {r.success || sw[3]}"></i><i style="--c: {r.danger || sw[4]}"></i>
    </span>
    {#if scheme.current}
      <Icon name="check" label="Selected" />
    {:else if scheme.builtin}
      <Icon name="lock" tone="muted" label="Built in" />
    {/if}
    <!-- the overflow menu; its clicks never reach the card -->
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <span class="contents" on:click|stopPropagation on:keydown|stopPropagation>
      <Menu.Root>
        <Menu.Trigger class="ewe-iconbtn ewe-iconbtn--ghost ewe-iconbtn--sm" aria-label="More for {scheme.name}" title="More for {scheme.name}" {disabled}>
          <Icon name="ellipsis" />
        </Menu.Trigger>
        <Menu.Content align="end">
          <Menu.Item disabled={scheme.current} onSelect={onApply}><Icon name="check" /><span class="ewe-menu__label">Apply</span></Menu.Item>
          <Menu.Item onSelect={onExport}><Icon name="copy" /><span class="ewe-menu__label">Export as YAML</span></Menu.Item>
          <Menu.Item onSelect={onDuplicate}><Icon name="plus" /><span class="ewe-menu__label">Duplicate</span></Menu.Item>
          {#if onEdit}
            <Menu.Item onSelect={onEdit}><Icon name="palette" /><span class="ewe-menu__label">Edit colors…</span></Menu.Item>
          {/if}
          {#if onLight}
            <Menu.Item onSelect={onLight}><Icon name="image" /><span class="ewe-menu__label">Make a light version</span></Menu.Item>
          {/if}
          {#if !scheme.builtin}
            <Menu.Separator />
            <Menu.Item variant="destructive" onSelect={onRemove}><Icon name="trash" /><span class="ewe-menu__label">Remove…</span></Menu.Item>
          {/if}
        </Menu.Content>
      </Menu.Root>
    </span>
  </div>
  {#if moved.length}
    <div class="ewe-alert ewe-alert--warning scheme-alert" role="note">
      <Icon name="info" />
      <div class="ewe-alert__body">
        <div class="ewe-alert__desc">Adjusted for contrast: {moved.join(", ")}.</div>
      </div>
    </div>
  {/if}
</div>
