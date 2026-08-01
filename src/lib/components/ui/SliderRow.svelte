<script>
  export let label = "";
  export let value = 0;
  export let from = 0;
  export let to = 100;
  export let step = 1;
  export let unit = "";
  export let dim = false;
  /** Called on release (change), not every pixel — writes are not free. */
  export let moved = () => {};
  let live = null; // value while dragging

  $: shown = live ?? value;
</script>

<div class="px-4 py-3 {dim ? 'pointer-events-none opacity-50' : ''}">
  <div class="mb-1.5 flex items-center justify-between">
    <span class="text-sm font-medium">{label}</span>
    <span class="text-xs tabular-nums text-zinc-400">{shown}{unit}</span>
  </div>
  <input
    type="range"
    min={from}
    max={to}
    {step}
    value={shown}
    class="w-full accent-[var(--accent)]"
    on:input={(e) => (live = Number(e.currentTarget.value))}
    on:change={(e) => {
      live = null;
      moved(Number(e.currentTarget.value));
    }}
  />
</div>
