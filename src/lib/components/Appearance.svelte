<script>
  import * as api from "../api.js";
  import { prefs, themeName, accentApplies } from "../stores.js";
  import { toast } from "../stores.js";

  const looks = [
    ["graphite", "Graphite", "Neutral dark greys, rounded corners, your accent colour."],
    ["ambiance", "Ambiance (Unity)", "Warm gradient panel, full-height square highlights, Ubuntu type, aubergine menus. The accent is fixed to Ubuntu orange."]
  ];

  let busy = false;
  async function set(patch) {
    busy = true;
    try {
      prefs.set(await api.writePrefs(patch));
    } catch (e) {
      console.error(e);
    }
    busy = false;
  }
</script>

<div class="space-y-6 p-8">
  <h1 class="text-lg font-semibold">Appearance</h1>

  <section>
    <div class="section-title">Shell style</div>
    <div class="card divide-y divide-zinc-800/60">
      {#each looks as [id, label, blurb] (id)}
        <button
          class="flex w-full items-start gap-3 p-4 text-left hover:bg-white/5 disabled:opacity-50"
          disabled={busy}
          on:click={() => set({ themeName: id })}
        >
          <span class="mt-1 h-3 w-3 shrink-0 rounded-full border
                       {$themeName === id ? 'border-accent bg-accent' : 'border-zinc-600'}"></span>
          <span>
            <span class="block text-sm font-medium">{label}</span>
            <span class="block text-xs text-zinc-400">{blurb}</span>
          </span>
        </button>
      {/each}
    </div>
  </section>

  <section>
    <div class="section-title">Accent colour</div>
    <div class="card p-4">
      {#if $accentApplies}
        <p class="text-xs text-zinc-400">Applies to the shell, window borders and GTK/Qt apps.</p>
      {:else}
        <p class="text-xs text-amber-500">
          Ambiance fixes the accent to Ubuntu orange, so this has no effect while it is selected.
        </p>
      {/if}
    </div>
  </section>
</div>
