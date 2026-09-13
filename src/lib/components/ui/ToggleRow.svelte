<script>
  import Toggle from "./Toggle.svelte";
  export let title = "";
  export let sub = "";
  export let on = false;
  export let dim = false;
  export let toggled = () => {};
</script>

<div class="row {dim ? 'opacity-50' : ''}">
  <div class="min-w-0">
    <div class="row-title">{title}</div>
    {#if sub}<div class="row-sub">{sub}</div>{/if}
  </div>
  <Toggle {on} disabled={dim} {toggled} />
</div>
