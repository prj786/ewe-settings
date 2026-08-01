<script>
  import { onMount } from "svelte";
  import * as api from "./lib/api.js";
  import {
    prefs, pane, version, shellUp, effectiveAccent, appliedMsg, errorMsg
  } from "./lib/stores.js";
  import Toasts from "./lib/components/Toasts.svelte";
  import Appearance from "./lib/components/Appearance.svelte";
  import Layout from "./lib/components/Layout.svelte";
  import Displays from "./lib/components/Displays.svelte";
  import WallpaperPane from "./lib/components/WallpaperPane.svelte";
  import Input from "./lib/components/Input.svelte";
  import Saver from "./lib/components/Saver.svelte";
  import PowerPane from "./lib/components/PowerPane.svelte";
  import DefaultApps from "./lib/components/DefaultApps.svelte";
  import Startup from "./lib/components/Startup.svelte";
  import Shortcuts from "./lib/components/Shortcuts.svelte";
  import SystemInfo from "./lib/components/SystemInfo.svelte";
  import Network from "./lib/components/Network.svelte";
  import UserPane from "./lib/components/UserPane.svelte";

  // No window controls. Settings is part of hypr-shell, and the shell's bar
  // already provides New / Float / Move / Close for every window.
  // The shell is always dark; Settings matches it rather than the GTK scheme.
  document.documentElement.classList.add("dark");

  const panes = [
    ["appearance", "Appearance", "M12 3a9 9 0 1 0 9 9c0-.46-.04-.92-.1-1.36a5.4 5.4 0 0 1-4.4 2.26 5.4 5.4 0 0 1-5.4-5.4 5.4 5.4 0 0 1 2.26-4.4A9.1 9.1 0 0 0 12 3z"],
    ["layout", "Layout & Dock", "M3 5h18v6H3zM3 15h8v4H3zM15 15h6v4h-6z"],
    ["displays", "Displays", "M2 4h20v12H2zM8 20h8M12 16v4"],
    ["network", "Networking", "M5 12.55a11 11 0 0 1 14.08 0M8.53 16.11a6 6 0 0 1 6.95 0M12 20h.01"],
    ["wallpaper", "Wallpaper", "M3 5h18v14H3zM3 15l5-5 4 4 3-3 6 6"],
    ["input", "Keyboard & Mouse", "M2 7h20v10H2zM6 11h1M10 11h1M14 11h1M18 11h1M7 14h10"],
    ["saver", "Screensaver", "M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9z"],
    ["power", "Power", "M12 2v10M6.3 6.3a8 8 0 1 0 11.4 0"],
    ["defaults", "Default Apps", "M12 2l9 5-9 5-9-5 9-5zM3 12l9 5 9-5M3 17l9 5 9-5"],
    ["startup", "Startup", "M5 3l14 9-14 9V3z"],
    ["shortcuts", "Shortcuts", "M4 6h16M4 12h16M4 18h10"],
    ["user", "User", "M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2M12 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8z"],
    ["system", "System", "M4 4h16v16H4zM9 9h6v6H9z"]
  ];

  $: document.documentElement.style.setProperty("--accent", $effectiveAccent);

  onMount(async () => {
    try { prefs.set(await api.readPrefs()); } catch (e) { console.error(e); }
    try { version.set(await api.shellVersion()); } catch { version.set("unknown"); }
    try { shellUp.set(await api.shellRunning()); } catch { shellUp.set(false); }
  });
</script>

<div class="flex h-full">
  <!-- Collapses to an icon rail on narrow (tiled) windows. -->
  <aside class="flex w-14 shrink-0 flex-col border-r border-zinc-800/60 bg-black/20 md:w-56">
    <div class="hidden px-4 pb-2 pt-5 text-sm font-semibold md:block">Settings</div>
    <div class="h-3 md:hidden"></div>
    <nav class="flex-1 space-y-0.5 overflow-y-auto px-1.5 py-2 md:px-2">
      {#each panes as [id, label, icon] (id)}
        <button
          title={label}
          class="flex w-full items-center justify-center gap-2.5 rounded-md px-2 py-2 text-left text-sm transition-colors md:justify-start md:px-3
                 {$pane === id ? 'text-white' : 'hover:bg-white/5'}"
          style={$pane === id ? "background: var(--accent)" : ""}
          on:click={() => pane.set(id)}
        >
          <svg viewBox="0 0 24 24" class="h-4 w-4 shrink-0" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
            <path d={icon} />
          </svg>
          <span class="hidden min-w-0 flex-1 truncate md:block">{label}</span>
        </button>
      {/each}
    </nav>

    <!-- hypr-shell's version, not this app's: Settings is part of the desktop
         rather than a separate product. -->
    <div class="hidden px-4 py-3 text-[11px] text-zinc-500 md:block">
      hypr-shell {$version}
      {#if !$shellUp}<div class="mt-1 text-amber-500">shell not running — changes apply at next start</div>{/if}
    </div>
  </aside>

  <main class="relative min-w-0 flex-1 overflow-y-auto">
    {#if $errorMsg}
      <div class="sticky top-0 z-30 flex items-center justify-between gap-3 border-b border-red-500/30 bg-red-950/90 px-4 py-2 text-sm text-red-200 backdrop-blur">
        <span class="min-w-0 truncate" title={$errorMsg}>{$errorMsg}</span>
        <button class="shrink-0 text-xs underline" on:click={() => errorMsg.set("")}>Dismiss</button>
      </div>
    {/if}

    {#if $pane === "appearance"}
      <Appearance />
    {:else if $pane === "layout"}
      <Layout />
    {:else if $pane === "displays"}
      <Displays />
    {:else if $pane === "network"}
      <Network />
    {:else if $pane === "wallpaper"}
      <WallpaperPane />
    {:else if $pane === "input"}
      <Input />
    {:else if $pane === "saver"}
      <Saver />
    {:else if $pane === "power"}
      <PowerPane />
    {:else if $pane === "defaults"}
      <DefaultApps />
    {:else if $pane === "startup"}
      <Startup />
    {:else if $pane === "shortcuts"}
      <Shortcuts />
    {:else if $pane === "user"}
      <UserPane />
    {:else}
      <SystemInfo />
    {/if}

    {#if $appliedMsg}
      <div class="pointer-events-none fixed bottom-4 left-1/2 z-40 -translate-x-1/2 rounded-full px-4 py-1.5 text-xs font-medium text-white shadow-lg" style="background: var(--accent)">
        {$appliedMsg}
      </div>
    {/if}
  </main>
</div>
<Toasts />
