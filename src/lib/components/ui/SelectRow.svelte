<script>
  import { afterUpdate } from "svelte";
  export let label = "";
  export let sub = "";
  export let options = []; // [{label, value}]
  export let value;
  export let dim = false;
  export let width = "w-48";
  export let picked = () => {};

  let el;
  // Setting <select value> before its <option>s exist silently resets it to "",
  // which is exactly what happens with a plain value attribute while the each
  // block renders. afterUpdate runs once the options are in the DOM, so the
  // current value is always visible.
  afterUpdate(() => {
    if (el) el.value = String(value);
  });
</script>

<div class="flex flex-wrap items-center justify-between gap-x-3 gap-y-1.5 px-4 py-3 {dim ? 'pointer-events-none opacity-50' : ''}">
  <div class="min-w-0">
    <div class="text-sm font-medium">{label}</div>
    {#if sub}<div class="text-xs text-zinc-400 dark:text-zinc-500">{sub}</div>{/if}
  </div>
  <select
    bind:this={el}
    class="input {width} !w-auto min-w-32"
    on:change={(e) => {
      const raw = e.currentTarget.value;
      const opt = options.find((o) => String(o.value) === raw);
      picked(opt ? opt.value : raw);
    }}
  >
    {#each options as o (String(o.value))}
      <option value={String(o.value)} selected={String(o.value) === String(value)}>{o.label}</option>
    {/each}
  </select>
</div>
