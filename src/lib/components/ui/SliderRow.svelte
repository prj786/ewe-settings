<script>
  import { Slider } from "./slider/index.js";
  import Row from "./Row.svelte";
  export let label = "";
  export let sub = "";
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

<Row title={label} {sub} {dim}>
  <div class="ewe-slider slider-trail">
    <Slider
      type="single"
      value={shown}
      min={from}
      max={to}
      {step}
      disabled={dim}
      aria-label={label}
      onValueChange={(v) => (live = v)}
      onValueCommit={(v) => {
        live = null;
        moved(v);
      }}
    />
    <!-- Geist Mono, right-aligned in its own column so it never shifts -->
    <span class="ewe-slider__value slider-value">{shown}{unit}</span>
  </div>
</Row>
