<script>
  import { onMount } from "svelte";
  import * as api from "../api.js";
  import { shortcutsModel } from "../hypr.js";

  let rows = [];
  onMount(async () => {
    try {
      rows = shortcutsModel(await api.readConfig("hypr/SHORTCUTS.md"));
    } catch {}
  });
</script>

<div class="mx-auto max-w-3xl space-y-1 p-5 sm:p-8">
  <h1 class="mb-4 text-lg font-semibold">Keyboard shortcuts</h1>

  {#if rows.length === 0}
    <p class="text-sm text-zinc-400">SHORTCUTS.md not found.</p>
  {/if}

  {#each rows as r, i (i)}
    {#if r.h}
      <div class="section-title !mt-6">{r.a}</div>
    {:else}
      <div class="flex items-baseline justify-between gap-4 border-b border-zinc-200/60 py-1.5 dark:border-zinc-700/40">
        <kbd class="shrink-0 rounded bg-zinc-200/70 px-2 py-0.5 font-mono text-[11px] dark:bg-zinc-700/60">{r.a}</kbd>
        <span class="min-w-0 text-right text-sm text-zinc-500 dark:text-zinc-400">{r.b}</span>
      </div>
    {/if}
  {/each}
</div>
