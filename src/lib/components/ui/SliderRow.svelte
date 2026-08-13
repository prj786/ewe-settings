<script>
  import { Slider } from "./slider/index.js";
  export let label = "";
  export let value = 0;
  export let from = 0;
  export let to = 100;
  export let step = 1;
  export let unit = "";
  export let dim = false;
  /** Called on release (commit), not every pixel — writes are not free. */
  export let moved = () => {};
  let live = null; // value while dragging

  // Fractional steps (0.05, 0.1) accumulate float dust, so the readout is
  // snapped back to however many decimals the step itself carries.
  const decimals = (n) => {
    const s = String(n);
    const i = s.indexOf(".");
    return i < 0 ? 0 : s.length - i - 1;
  };
  $: shown = Number(Number(live ?? value).toFixed(decimals(step)));
</script>

<div class="px-4 py-3 {dim ? 'pointer-events-none opacity-50' : ''}">
  <div class="mb-2 flex items-center justify-between">
    <span class="text-sm font-medium">{label}</span>
    <span class="text-xs tabular-nums text-zinc-400">{shown}{unit}</span>
  </div>
  <Slider
    type="single"
    value={shown}
    min={from}
    max={to}
    {step}
    aria-label={label}
    onValueChange={(v) => (live = v)}
    onValueCommit={(v) => {
      live = null;
      moved(v);
    }}
  />
</div>
