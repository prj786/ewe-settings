<script>
  /**
   * A settings page (App shell, Settings page): the page-title Header bar
   * with a one-line description and at most one page action, then groups.
   */
  export let title = "";
  export let desc = "";
</script>

<div class="page">
  <header class="ewe-headerbar ewe-headerbar--page">
    <div class="ewe-headerbar__titles">
      <h1 class="ewe-headerbar__title">{title}</h1>
      {#if desc}<p class="ewe-headerbar__subtitle">{desc}</p>{/if}
    </div>
    {#if $$slots.actions}
      <div class="ewe-headerbar__actions"><slot name="actions" /></div>
    {/if}
  </header>
  <slot />
</div>
