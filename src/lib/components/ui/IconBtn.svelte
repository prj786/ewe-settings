<script>
  /** Small round icon button — chevrons, close, etc. `icon` is inner SVG. */
  export let icon = "";
  export let title = "";
  export let disabled = false;
  export let danger = false;
  export let go = () => {};
</script>

<button
  {title}
  aria-label={title}
  {disabled}
  class="flex h-6 w-6 items-center justify-center rounded-full transition-colors disabled:opacity-30
    {danger
    ? 'text-zinc-400 hover:bg-red-500/15 hover:text-red-400'
    : 'text-zinc-400 hover:bg-white/10 hover:text-zinc-100'}"
  on:click={() => go()}
>
  <svg viewBox="0 0 24 24" class="h-3.5 w-3.5" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
    {@html icon}
  </svg>
</button>
