<script>
  export let on = false;
  export let disabled = false;
  export let toggled = () => {};
</script>

<button
  type="button"
  role="switch"
  aria-checked={on}
  aria-label="Toggle"
  {disabled}
  class="relative h-5 w-9 shrink-0 rounded-full transition-colors disabled:opacity-40
    {on ? '' : 'bg-zinc-300 dark:bg-zinc-600'}"
  style={on ? "background: var(--accent)" : ""}
  on:click={() => toggled()}
>
  <span
    class="absolute top-0.5 h-4 w-4 rounded-full bg-white shadow transition-all"
    style="left: {on ? '18px' : '2px'}"
  ></span>
</button>
