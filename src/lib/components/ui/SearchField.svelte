<script>
  /**
   * Search field (design/system/components/SearchField): the Text field box
   * with the search glyph first and a clear button once there is text. Esc
   * clears the query; with nothing to clear it goes on to `onKey`, so the
   * surface can close. `size` "sm" is the toolbar / side-navigation size.
   */
  import Icon from "./Icon.svelte";
  import { onMount } from "svelte";
  export let value = "";
  export let placeholder = "Search";
  export let autofocus = false;
  export let size = "";
  export let el = null; // the <input>, for focus from outside
  export let onKey = null;
  export let onFocus = null;
  export let onBlur = null;
  onMount(() => autofocus && el?.focus());
</script>

<label class="ewe-input {size ? `ewe-input--${size}` : ''} search-box">
  <Icon name="search" />
  <input
    bind:this={el}
    bind:value
    class="ewe-input__text search-text"
    type="search"
    {placeholder}
    aria-label={placeholder}
    autocomplete="off"
    spellcheck="false"
    on:focus={() => onFocus && onFocus()}
    on:blur={() => onBlur && onBlur()}
    on:keydown={(e) => {
      if (e.key === "Escape" && value) {
        e.stopPropagation();
        e.preventDefault();
        value = "";
        return;
      }
      onKey && onKey(e);
    }}
  />
  {#if value}
    <button type="button" class="ewe-iconbtn ewe-iconbtn--ghost ewe-iconbtn--sm" aria-label="Clear" on:click={() => ((value = ""), el?.focus())}>
      <Icon name="x" />
    </button>
  {/if}
</label>
