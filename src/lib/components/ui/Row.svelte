<script>
  /**
   * A settings row (Settings page, List row): the setting's name and what it
   * does on the left, its control or current value in the fixed column on
   * the right. 40px tall, 48 with a description; rows in a well are divided
   * by a 1px line (app.css). `icon` adds the 32px icon tile; `dim` marks the
   * row unavailable; `block` stacks the control under the text; `sub` is the
   * description; `indent` nests the row under the one above (Glass →
   * Opacity). `setting` (default: the title) is what Find a setting looks
   * for. `onRow` makes a click anywhere on the row act like its control.
   */
  import Icon from "./Icon.svelte";
  export let title = "";
  export let sub = "";
  export let icon = "";
  export let dim = false;
  export let block = false;
  export let dense = false;
  export let indent = false;
  export let setting = "";
  export let onRow = null;
  let className = "";
  export { className as class };
  let trail;
  function rowClick(e) {
    if (!onRow || dim) return;
    // the control handles its own clicks; the rest of the row forwards
    if (trail && trail.contains(e.target)) return;
    if (e.target.closest("a, button, input, select, textarea, [role='switch']")) return;
    onRow();
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div
  class="ewe-row {block ? 'ewe-row--block' : ''} {dense ? 'ewe-row--dense' : ''} {indent ? 'ewe-row--indent' : ''} {onRow && !dim ? 'row-clickable' : ''} {dim ? 'is-dim' : ''} {className}"
  aria-disabled={dim || undefined}
  data-setting={setting || title || undefined}
  on:click={rowClick}
>
  {#if icon}
    <span class="ewe-row__lead ewe-row__lead--tile"><Icon name={icon} /></span>
  {/if}
  {#if title || sub || $$slots.text}
    <div class="ewe-row__text">
      {#if title}<div class="ewe-row__title">{title}</div>{/if}
      {#if sub}<div class="ewe-row__desc">{sub}</div>{/if}
      <slot name="text" />
    </div>
  {/if}
  {#if $$slots.default}
    <div class="ewe-row__trail" class:pointer-events-none={dim} bind:this={trail}><slot /></div>
  {/if}
</div>
