<script>
  /**
   * A group on a settings page: an overline Section header, at most one
   * line of description right under it, then a list well of rows separated
   * by dividers. `action` is the header's optional small button (right).
   * `well = false` leaves the content bare (cards, grids). The title is the
   * group's data-section, which Find a setting scrolls to.
   */
  export let title = "";
  export let desc = "";
  export let well = true;
  let className = "";
  export { className as class };
</script>

<section class="group {className}" data-section={title || undefined}>
  {#if title || desc || $$slots.action}
    <div class="ewe-section group-head">
      <div class="ewe-section__text">
        {#if title}<h2 class="ewe-section__title ewe-section__title--overline">{title}</h2>{/if}
        {#if desc}<p class="group-desc">{desc}</p>{/if}
      </div>
      <slot name="action" />
    </div>
  {/if}
  {#if well}
    <div class="ewe-list"><slot /></div>
  {:else}
    <slot />
  {/if}
  <slot name="after" />
</section>
