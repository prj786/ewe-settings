<script>
  import * as Select from "./select/index.js";
  import Row from "./Row.svelte";
  export let label = "";
  export let sub = "";
  export let icon = "";
  export let options = []; // [{label, value}]
  export let value;
  export let dim = false;
  export let indent = false;
  export let width = ""; // kept for older call sites; the trigger sizes to its value
  export let picked = () => {};

  // bits-ui speaks strings, while option values here are freely numbers,
  // booleans or strings — so map in and out at the boundary. It also reads ""
  // as "nothing selected", so an option that legitimately *means* empty (the
  // "Default" entries) travels under a sentinel and sheds it on the way out.
  const EMPTY = " empty";
  const enc = (v) => (String(v ?? "") === "" ? EMPTY : String(v));

  $: current = enc(value);
  $: shown = (options.find((o) => enc(o.value) === current) || {}).label;
</script>

<Row title={label} {sub} {icon} {dim} {indent}>
  <Select.Root
    type="single"
    value={current}
    disabled={dim}
    onValueChange={(raw) => {
      const opt = options.find((o) => enc(o.value) === raw);
      picked(opt ? opt.value : raw);
    }}
  >
    <Select.Trigger class="select-trigger {width}" aria-label={label}>
      {shown ?? "—"}
    </Select.Trigger>
    <Select.Content>
      {#each options as o (enc(o.value))}
        <Select.Item value={enc(o.value)} label={o.label} />
      {/each}
    </Select.Content>
  </Select.Root>
</Row>
