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
  // The released value, held until `value` comes back from the write. The
  // knob used to jump to the OLD value on release and only then to the new
  // one, once ewe-conf, the reloads and `ewe-theme show` had all answered.
  // A write that never lands (refused, failed) gives up after a few seconds.
  let committed = null;
  let giveUp;
  $: value, (committed = committed !== null && Number(value) === Number(committed) ? null : committed);
  function commit(v) {
    live = null;
    committed = v;
    clearTimeout(giveUp);
    giveUp = setTimeout(() => (committed = null), 6000);
    moved(v);
  }

  // Fractional steps (0.05, 0.1) accumulate float dust, so the readout is
  // snapped back to however many decimals the step itself carries.
  const decimals = (n) => {
    const s = String(n);
    const i = s.indexOf(".");
    return i < 0 ? 0 : s.length - i - 1;
  };
  $: shown = Number(Number(live ?? committed ?? value).toFixed(decimals(step)));
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
      onValueCommit={commit}
    />
    <!-- Geist Mono, right-aligned in its own column so it never shifts -->
    <span class="ewe-slider__value slider-value">{shown}{unit}</span>
  </div>
</Row>
