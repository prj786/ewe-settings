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

  // Lucide-style inner SVG markup (24×24, stroked round) — single paths were
  // too crude for the keyboard/wallpaper glyphs.
  const panes = [
    ["appearance", "Appearance", '<circle cx="13.5" cy="6.5" r=".5" fill="currentColor"/><circle cx="17.5" cy="10.5" r=".5" fill="currentColor"/><circle cx="8.5" cy="7.5" r=".5" fill="currentColor"/><circle cx="6.5" cy="12.5" r=".5" fill="currentColor"/><path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 0 1 1.668-1.668h1.996c3.051 0 5.555-2.503 5.555-5.554C21.965 6.012 17.461 2 12 2z"/>'],
    ["layout", "Layout & Dock", '<rect width="7" height="9" x="3" y="3" rx="1"/><rect width="7" height="5" x="14" y="3" rx="1"/><rect width="7" height="9" x="14" y="12" rx="1"/><rect width="7" height="5" x="3" y="16" rx="1"/>'],
    ["displays", "Displays", '<rect width="20" height="14" x="2" y="3" rx="2"/><line x1="8" x2="16" y1="21" y2="21"/><line x1="12" x2="12" y1="17" y2="21"/>'],
    ["network", "Networking", '<path d="M12 20h.01"/><path d="M2 8.82a15 15 0 0 1 20 0"/><path d="M5 12.859a10 10 0 0 1 14 0"/><path d="M8.5 16.429a5 5 0 0 1 7 0"/>'],
    ["wallpaper", "Wallpaper", '<rect width="18" height="18" x="3" y="3" rx="2" ry="2"/><circle cx="9" cy="9" r="2"/><path d="m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21"/>'],
    ["input", "Keyboard & Mouse", '<rect width="20" height="16" x="2" y="4" rx="2"/><path d="M6 8h.01"/><path d="M10 8h.01"/><path d="M14 8h.01"/><path d="M18 8h.01"/><path d="M8 12h.01"/><path d="M12 12h.01"/><path d="M16 12h.01"/><path d="M7 16h10"/>'],
    ["saver", "Screensaver", '<path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9z"/>'],
    ["power", "Power", '<path d="M12 2v10"/><path d="M18.4 6.6a9 9 0 1 1-12.77.04"/>'],
    ["defaults", "Default Apps", '<rect width="7" height="7" x="3" y="3" rx="1"/><rect width="7" height="7" x="14" y="3" rx="1"/><rect width="7" height="7" x="14" y="14" rx="1"/><rect width="7" height="7" x="3" y="14" rx="1"/>'],
    ["startup", "Startup", '<circle cx="12" cy="12" r="10"/><polygon points="10 8 16 12 10 16 10 8"/>'],
    ["shortcuts", "Shortcuts", '<path d="M15 6v12a3 3 0 1 0 3-3H6a3 3 0 1 0 3 3V6a3 3 0 1 0-3 3h12a3 3 0 1 0-3-3"/>'],
    ["user", "User", '<path d="M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/>'],
    ["system", "System", '<rect width="16" height="16" x="4" y="4" rx="2"/><rect width="6" height="6" x="9" y="9" rx="1"/><path d="M15 2v2"/><path d="M15 20v2"/><path d="M2 15h2"/><path d="M2 9h2"/><path d="M20 15h2"/><path d="M20 9h2"/><path d="M9 2v2"/><path d="M9 20v2"/>']
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
            {@html icon}
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
