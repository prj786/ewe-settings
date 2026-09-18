<script>
  /**
   * A search field over a list of rows to pick one from (adding a startup
   * app, a window rule, a keyboard layout, a language). Rows are buttons;
   * Up and Down move between them.
   * items: [{ id, title, desc?, trail? }]
   */
  import SearchField from "./SearchField.svelte";
  export let items = [];
  export let query = "";
  export let placeholder = "Search";
  export let empty = "No match. Check the spelling.";
  export let pick = () => {};
  let rows = [];
  function key(e, i) {
    const d = e.key === "ArrowDown" ? 1 : e.key === "ArrowUp" ? -1 : 0;
    if (!d) return;
    e.preventDefault();
    rows[Math.max(0, Math.min(items.length - 1, i + d))]?.focus();
  }
</script>

<div class="picklist">
  <SearchField bind:value={query} {placeholder} autofocus />
  <div class="ewe-list picklist__rows">
    {#each items as it, i (it.id)}
      <button
        bind:this={rows[i]}
        type="button"
        class="ewe-row ewe-row--dense ewe-row--interactive"
        class:is-selected={it.selected}
        on:click={() => pick(it)}
        on:keydown={(e) => key(e, i)}
      >
        <span class="ewe-row__text"><span class="ewe-row__title">{it.title}</span></span>
        {#if it.trail}<span class="ewe-row__trail picklist__trail">{it.trail}</span>{/if}
      </button>
    {:else}
      <div class="ewe-row ewe-row--dense"><span class="ewe-row__desc">{empty}</span></div>
    {/each}
  </div>
</div>
