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
    ? 'text-dim hover:bg-[color-mix(in_srgb,var(--danger)_14%,transparent)]0/15 hover:text-danger'
    : 'text-dim hover:bg-white/10 hover:text-fg'}"
  on:click={() => go()}
>
  <svg viewBox="0 0 24 24" class="h-3.5 w-3.5" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
    {@html icon}
  </svg>
</button>
