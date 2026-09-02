<script>
  import { onMount } from "svelte";
  import * as api from "./lib/api.js";
  import {
    prefs, pane, version, shellUp, effectiveAccent, appliedMsg, errorMsg
  } from "./lib/stores.js";
  import Toasts from "./lib/components/Toasts.svelte";
  import Appearance from "./lib/components/Appearance.svelte";
  import Animations from "./lib/components/Animations.svelte";
  import Layout from "./lib/components/Layout.svelte";
  import Displays from "./lib/components/Displays.svelte";
  import WallpaperPane from "./lib/components/WallpaperPane.svelte";
  import Input from "./lib/components/Input.svelte";
  import WindowRules from "./lib/components/WindowRules.svelte";
  import Saver from "./lib/components/Saver.svelte";
  import PowerPane from "./lib/components/PowerPane.svelte";
  import DefaultApps from "./lib/components/DefaultApps.svelte";
  import Startup from "./lib/components/Startup.svelte";
  import Shortcuts from "./lib/components/Shortcuts.svelte";
  import SystemInfo from "./lib/components/SystemInfo.svelte";
  import Network from "./lib/components/Network.svelte";
  import UserPane from "./lib/components/UserPane.svelte";
  import TimePlace from "./lib/components/TimePlace.svelte";

  // No window controls. Settings is part of ewe, and the shell's bar
  // already provides New / Float / Move / Close for every window.
  // The shell is always dark; Settings matches it rather than the GTK scheme.
  document.documentElement.classList.add("dark");

  // Phosphor Fill codepoints — the same glyphs the shell's own sidebar uses,
  // so DE and app are one icon language (font in assets/, MIT).
  const panes = [
    ["appearance", "Appearance", 0xE6C8],
    ["animations", "Animations", 0xE628],
    ["layout", "Layout & Dock", 0xE6D6],
    ["windowrules", "Window Rules", 0xE464],
    ["displays", "Displays", 0xE32E],
    ["network", "Networking", 0xE4EA],
    ["wallpaper", "Wallpaper", 0xE2CA],
    ["input", "Keyboard & Mouse", 0xE2D8],
    ["saver", "Screensaver", 0xE58E],
    ["power", "Power", 0xE3DA],
    ["defaults", "Default Apps", 0xE5DA],
    ["startup", "Startup", 0xE3FE],
    ["shortcuts", "Shortcuts", 0xE1C4],
    ["time", "Time & Place", 0xE28C],
    ["user", "User", 0xE4C2],
    ["system", "System", 0xE610]
  ];

  $: document.documentElement.style.setProperty("--accent", $effectiveAccent);
  $: document.documentElement.classList.toggle("blacksheep", ($prefs.themeName || "flock") === "blacksheep");

  onMount(async () => {
    try { prefs.set(await api.readPrefs()); } catch (e) { console.error(e); }
    try { version.set(await api.shellVersion()); } catch { version.set("unknown"); }
    try { shellUp.set(await api.shellRunning()); } catch { shellUp.set(false); }
  });
</script>

<div class="surface flex h-full">
  <!-- Collapses to an icon rail on narrow (tiled) windows. -->
  <!-- .rail / .rail-* — the chrome shared with Komble and ewe-sync (app.css) -->
  <aside class="rail">
    <div class="rail-brand">
      <!-- the gear mark, in the same 32 px tile the other apps use -->
      <div class="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg text-white" style="background: linear-gradient(135deg, var(--accent), color-mix(in srgb, var(--accent) 55%, #1c1c1e))">
        <span class="ph-i text-[18px]">{String.fromCodePoint(0xE228)}</span>
      </div>
      <div class="rail-brand-name">Settings</div>
    </div>
    <nav class="rail-nav">
      {#each panes as [id, label, icon] (id)}
        <button
          title={label}
          class="rail-item {$pane === id ? 'is-active' : ''}"
          on:click={() => pane.set(id)}
        >
          <span class="ph-i">{String.fromCodePoint(icon)}</span>
          <span class="rail-label">{label}</span>
        </button>
      {/each}
    </nav>

    <!-- ewe's version, not this app's: Settings is part of the desktop
         rather than a separate product. -->
    <div class="rail-foot">
      ewe {$version}
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
    {:else if $pane === "animations"}
      <Animations />
    {:else if $pane === "layout"}
      <Layout />
    {:else if $pane === "windowrules"}
      <WindowRules />
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
    {:else if $pane === "time"}
      <TimePlace />
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
