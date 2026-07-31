<script>
  import { onMount } from "svelte";
  import * as api from "./lib/api.js";
  import { prefs, pane, version, shellUp, themeName, accentApplies } from "./lib/stores.js";
  import Toasts from "./lib/components/Toasts.svelte";
  import Appearance from "./lib/components/Appearance.svelte";

  // No window controls. Settings is part of hypr-shell, and the shell's bar
  // already provides New / Float / Move / Close for every window — drawing our
  // own title bar and an X would be a second set of controls for the same job.
  const panes = [
    ["appearance", "Appearance"],
    ["displays", "Displays"],
    ["input", "Keyboard & Mouse"],
    ["power", "Power"],
    ["startup", "Startup"],
    ["rules", "Window rules"]
  ];

  onMount(async () => {
    try { prefs.set(await api.readPrefs()); } catch (e) { console.error(e); }
    try { version.set(await api.shellVersion()); } catch { version.set("unknown"); }
    try { shellUp.set(await api.shellRunning()); } catch { shellUp.set(false); }
  });
</script>

<div class="flex h-full">
  <aside class="flex w-56 shrink-0 flex-col border-r border-zinc-800/60 bg-black/20">
    <div class="px-4 pb-2 pt-5 text-sm font-semibold">Settings</div>
    <nav class="flex-1 space-y-0.5 px-2 py-2">
      {#each panes as [id, label] (id)}
        <button
          class="w-full rounded-md px-3 py-2 text-left text-sm transition-colors
                 {$pane === id ? 'bg-accent text-white' : 'hover:bg-white/5'}"
          on:click={() => pane.set(id)}
        >{label}</button>
      {/each}
    </nav>

    <!-- hypr-shell's version, not this app's: Settings is part of the desktop
         rather than a separate product, and a second number would only raise
         the question of which one is real. -->
    <div class="px-4 py-3 text-[11px] text-zinc-500">
      hypr-shell {$version}
      {#if !$shellUp}<div class="mt-1 text-amber-500">shell not running — changes apply at next start</div>{/if}
    </div>
  </aside>

  <main class="flex-1 overflow-y-auto">
    {#if $pane === "appearance"}
      <Appearance />
    {:else}
      <div class="p-8 text-sm text-zinc-400">
        <h1 class="mb-2 text-lg font-semibold text-zinc-200">
          {panes.find(([id]) => id === $pane)?.[1]}
        </h1>
        Not ported yet — still in the shell's own Settings for now.
      </div>
    {/if}
  </main>
</div>
<Toasts />
