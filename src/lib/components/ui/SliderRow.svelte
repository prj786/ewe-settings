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

<div class="px-6 py-2 {dim ? 'pointer-events-none opacity-50' : ''}">
  <div class="mb-2 flex items-center justify-between">
    <span class="row-title">{label}</span>
    <!-- the readout is a MARK, so it wears the raw accent -->
    <span class="text-sm font-bold tabular-nums text-[var(--accent)]">{shown}{unit}</span>
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
