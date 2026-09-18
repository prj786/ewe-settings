<script>
  import { onMount, onDestroy } from "svelte";
  import { Tooltip } from "bits-ui";
  import * as api from "./lib/api.js";
  import { prefs, pane, version, shellUp, themeKey, errorMsg } from "./lib/stores.js";
  import { refreshTheme, watchTheme } from "./lib/theme.js";
  import sheep from "./assets/sheep.svg?raw";
  import Icon from "./lib/components/ui/Icon.svelte";
  import Alert from "./lib/components/ui/Alert.svelte";
  import Toasts from "./lib/components/Toasts.svelte";
  import Appearance from "./lib/components/Appearance.svelte";
  import Accessibility from "./lib/components/Accessibility.svelte";
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
  import Bluetooth from "./lib/components/Bluetooth.svelte";
  import UserPane from "./lib/components/UserPane.svelte";
  import TimePlace from "./lib/components/TimePlace.svelte";

  // No window controls. Settings is part of ewe, and the shell's bar
  // already provides New / Float / Move / Close for every window.

  // The side navigation (Side navigation card). Icons are Lucide glyphs from
  // the same font the shell uses. Sentence case, the words people use.
  const panes = [
    ["appearance", "Appearance", "palette", Appearance],
    ["accessibility", "Accessibility", "accessibility", Accessibility],
    ["animations", "Animations", "gauge", Animations],
    ["layout", "Layout and dock", "layout", Layout],
    ["windowrules", "Window rules", "windowRules", WindowRules],
    ["displays", "Displays", "monitor", Displays],
    ["network", "Networking", "wifi", Network],
    ["bluetooth", "Bluetooth", "bluetooth", Bluetooth],
    ["wallpaper", "Wallpaper", "image", WallpaperPane],
    ["input", "Keyboard and mouse", "keyboard", Input],
    ["saver", "Screensaver", "saver", Saver],
    ["power", "Power", "power", PowerPane],
    ["defaults", "Default apps", "apps", DefaultApps],
    ["startup", "Startup", "rocket", Startup],
    ["shortcuts", "Shortcuts", "command", Shortcuts],
    ["time", "Time and place", "globe", TimePlace],
    ["user", "User", "user", UserPane],
    ["system", "System", "cpu", SystemInfo]
  ];
  $: current = panes.find(([id]) => id === $pane) || panes[panes.length - 1];

  // The look, live (lib/theme.js): at start, on focus, and whenever the
  // theme input in prefs moves (an accent pick from any pane).
  $: $themeKey, refreshTheme();

  // Up and Down move between sections; Ctrl+1 … Ctrl+9 jump to the first nine.
  let navItems = [];
  function navKey(e, i) {
    const d = e.key === "ArrowDown" ? 1 : e.key === "ArrowUp" ? -1 : 0;
    if (!d) return;
    e.preventDefault();
    const n = (i + d + panes.length) % panes.length;
    navItems[n]?.focus();
    pane.set(panes[n][0]);
  }
  function globalKey(e) {
    if (e.ctrlKey && !e.altKey && !e.shiftKey && /^[1-9]$/.test(e.key)) {
      const p = panes[Number(e.key) - 1];
      if (p) {
        e.preventDefault();
        pane.set(p[0]);
      }
    }
  }

  let restarting = false;
  async function restartShell() {
    restarting = true;
    try { await api.restartShell(); } catch (e) { errorMsg.set(String(e)); }
    setTimeout(async () => {
      try { shellUp.set(await api.shellRunning()); } catch { shellUp.set(false); }
      restarting = false;
    }, 2500);
  }

  let unwatch = () => {};
  onMount(async () => {
    unwatch = watchTheme();
    try { prefs.set(await api.readPrefs()); } catch (e) { console.error(e); }
    try { version.set(await api.shellVersion()); } catch { version.set("unknown"); }
    try { shellUp.set(await api.shellRunning()); } catch { shellUp.set(false); }
  });
  onDestroy(() => unwatch());
</script>

<svelte:window on:keydown={globalKey} />

<Tooltip.Provider>
  <div class="ewe-appwin">
    <!-- the rail: the navigation landmark (App shell, Side navigation) -->
    <nav class="ewe-sidenav" aria-label="Settings sections">
      <div class="ewe-sidenav__brand">
        <span class="ewe-sidenav__logo" aria-hidden="true">{@html sheep}</span>
        <span class="ewe-sidenav__name">Settings</span>
      </div>
      <div class="ewe-sidenav__group">
        {#each panes as [id, label, icon], i (id)}
          <button
            bind:this={navItems[i]}
            class="ewe-navitem"
            class:is-selected={$pane === id}
            aria-current={$pane === id ? "page" : undefined}
            title={label}
            on:click={() => pane.set(id)}
            on:keydown={(e) => navKey(e, i)}
          >
            <Icon name={icon} />
            <span class="ewe-navitem__label">{label}</span>
          </button>
        {/each}
      </div>

      <!-- ewe's version, not this app's: Settings is part of the desktop
           rather than a separate product. -->
      <div class="ewe-sidenav__foot">
        {#if !$shellUp}
          <p class="ewe-sidenav__note">The shell isn't running. Changes apply at the next start.</p>
        {/if}
        <!-- Every change applies live through `settings reload`; this is the
             honest fallback for the rare one that does not, and after an
             update that replaced the shell's files under a running qs. -->
        <button
          class="ewe-navitem"
          title="Restart the shell (bar, dock, panels). Windows stay open."
          disabled={restarting}
          on:click={restartShell}
        >
          <Icon name="refresh" />
          <span class="ewe-navitem__label">{restarting ? "Restarting…" : "Restart shell"}</span>
        </button>
        <div class="ewe-sidenav__version">ewe {$version}</div>
      </div>
    </nav>

    <!-- the pane: the main landmark, inset from the window's edges -->
    <main class="ewe-appwin__pane">
      {#if $errorMsg}
        <Alert tone="danger" banner dismiss={() => errorMsg.set("")}>
          <span class="error-text" title={$errorMsg}>{$errorMsg}</span>
        </Alert>
      {/if}
      <div class="ewe-appwin__scroll">
        <svelte:component this={current[3]} />
      </div>
    </main>
  </div>
  <Toasts />
</Tooltip.Provider>
