<script>
  export let k = "";
  export let v = "";
</script>

<div class="flex items-baseline justify-between gap-4 px-4 py-2.5">
  <span class="shrink-0 text-sm text-zinc-400 dark:text-zinc-500">{k}</span>
  <span class="min-w-0 truncate text-right text-sm" title={v}>{v}</span>
</div>
