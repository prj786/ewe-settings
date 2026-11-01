<script>
  import { onMount, onDestroy, tick } from "svelte";
  import { Tooltip } from "bits-ui";
  import * as api from "./lib/api.js";
  import { prefs, pane, version, shellUp, themeKey, errorMsg } from "./lib/stores.js";
  import { refreshTheme, watchTheme } from "./lib/theme.js";
  import { loadAddons } from "./lib/addons.js";
  import { NAV, PAGES, pageById } from "./lib/pages.js";
  import { findSettings, found, reveal } from "./lib/search.js";
  import sheep from "./assets/ewe-mark.svg?raw";
  import Icon from "./lib/components/ui/Icon.svelte";
  import Alert from "./lib/components/ui/Alert.svelte";
  import SearchField from "./lib/components/ui/SearchField.svelte";
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

  // No window controls. ewe Settings is part of ewe, and the shell's bar
  // already provides New / Float / Move / Close for every window.

  // pane id → page (the labels and groups are lib/pages.js)
  const VIEWS = {
    appearance: Appearance,
    wallpaper: WallpaperPane,
    animations: Animations,
    accessibility: Accessibility,
    layout: Layout,
    windowrules: WindowRules,
    displays: Displays,
    startup: Startup,
    shortcuts: Shortcuts,
    network: Network,
    bluetooth: Bluetooth,
    input: Input,
    power: PowerPane,
    saver: Saver,
    defaults: DefaultApps,
    time: TimePlace,
    user: UserPane,
    system: SystemInfo
  };
  $: current = pageById($pane) || PAGES[0];

  // The look, live (lib/theme.js): at start, on focus, and whenever the
  // theme input in prefs moves (an accent pick from any pane).
  $: $themeKey, refreshTheme();

  // ── navigation: Up and Down walk every page across the groups; Ctrl+1 …
  //    Ctrl+9 jump to the first nine; Ctrl+F finds a setting ──────────────
  let navItems = {};
  function navKey(e, id) {
    const d = e.key === "ArrowDown" ? 1 : e.key === "ArrowUp" ? -1 : 0;
    if (!d) return;
    e.preventDefault();
    const i = PAGES.findIndex((p) => p.id === id);
    const n = PAGES[(i + d + PAGES.length) % PAGES.length];
    navItems[n.id]?.focus();
    pane.set(n.id);
  }
  function globalKey(e) {
    if (e.ctrlKey && !e.altKey && !e.shiftKey && /^[1-9]$/.test(e.key)) {
      const p = PAGES[Number(e.key) - 1];
      if (p) {
        e.preventDefault();
        go(p.id);
      }
    } else if (e.ctrlKey && !e.altKey && (e.key === "f" || e.key === "F")) {
      e.preventDefault();
      openSearch();
    }
  }
  function go(id) {
    pane.set(id);
    found.set(null);
  }

  // ── Find a setting: results replace the groups while there is a query ──
  let query = "";
  let searchEl;
  let searchOpen = false; // the narrow rail widens while searching
  let resultEls = [];
  let picked = -1; // the result last opened, marked like the current page
  $: results = findSettings(query);
  $: query, (picked = -1);
  async function openSearch() {
    searchOpen = true;
    await tick();
    searchEl?.focus();
    searchEl?.select();
  }
  function closeSearch() {
    query = "";
    searchOpen = false;
  }
  function openResult(r, i) {
    picked = i;
    pane.set(r.page);
    found.set({ ...r, seq: Date.now() });
  }
  function searchKey(e) {
    if (e.key === "ArrowDown" && results.length) {
      e.preventDefault();
      resultEls[0]?.focus();
    } else if (e.key === "Enter" && results.length) {
      e.preventDefault();
      openResult(results[0], 0);
    } else if (e.key === "Escape" && !query) {
      // the field's own Esc clears the query first; a second one leaves
      closeSearch();
      navItems[$pane]?.focus();
    }
  }
  function resultKey(e, i) {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const n = i + (e.key === "ArrowDown" ? 1 : -1);
      if (n < 0) searchEl?.focus();
      else resultEls[Math.min(n, results.length - 1)]?.focus();
    } else if (e.key === "Escape") {
      e.preventDefault();
      searchEl?.focus();
    }
  }

  // A result opens its page, then the page's row is found, scrolled to,
  // marked for a moment and given focus (lib/search.js).
  let paneEl;
  $: if ($found && paneEl) tick().then(() => reveal(paneEl, $found));

  // Is the shell answering? Asked at start, on focus, and every 10 s while
  // it is not. It used to be asked ONCE: a shell restarting at that moment
  // (login, an update, a plugin install) left "The shell isn't running" up
  // for the life of the window. Two misses in a row before saying so — one
  // restart gap is not "down".
  let misses = 0;
  async function probeShell() {
    let up = false;
    try { up = await api.shellRunning(); } catch { up = false; }
    misses = up ? 0 : misses + 1;
    shellUp.set(up || misses < 2);
  }

  let unwatch = () => {};
  let probeTimer;
  // Plugins are installed in Komble, next door: re-read the list whenever
  // this window comes back into focus, so their rows appear without a
  // relaunch.
  const onFocus = () => {
    loadAddons(true);
    probeShell();
  };
  onMount(async () => {
    unwatch = watchTheme();
    loadAddons();
    window.addEventListener("focus", onFocus);
    probeTimer = setInterval(() => { if (!$shellUp || misses) probeShell(); }, 10000);
    try { prefs.set(await api.readPrefs()); } catch (e) { console.error(e); }
    try { version.set(await api.shellVersion()); } catch { version.set("unknown"); }
    await probeShell();
    if (misses) setTimeout(probeShell, 1500);
  });
  onDestroy(() => {
    unwatch();
    clearInterval(probeTimer);
    window.removeEventListener("focus", onFocus);
  });
</script>

<svelte:window on:keydown={globalKey} />

<Tooltip.Provider>
  <div class="ewe-appwin">
    <!-- the rail: the navigation landmark (App shell, Side navigation) -->
    <nav class="ewe-sidenav settings-nav" class:is-searching={searchOpen || query} aria-label="ewe Settings sections">
      <div class="ewe-sidenav__brand">
        <span class="ewe-sidenav__logo" aria-hidden="true">{@html sheep}</span>
        <span class="ewe-sidenav__name">ewe Settings</span>
      </div>

      <div class="nav-search">
        <SearchField
          bind:value={query}
          bind:el={searchEl}
          size="sm"
          placeholder="Find a setting"
          onKey={searchKey}
          onFocus={() => (searchOpen = true)}
          onBlur={() => !query && (searchOpen = false)}
        />
      </div>
      <!-- the narrow rail has no room for the field: a button opens it -->
      <button class="ewe-navitem nav-search-btn" title="Find a setting (Ctrl+F)" aria-label="Find a setting" on:click={openSearch}>
        <Icon name="search" />
      </button>

      {#if query.trim()}
        <div class="nav-results" role="region" aria-label="Settings found">
          <p class="sr-only" aria-live="polite">
            {results.length === 0 ? "No settings found" : results.length === 1 ? "1 setting found" : `${results.length} settings found`}
          </p>
          {#each results as r, i (r.key)}
            <button
              bind:this={resultEls[i]}
              class="ewe-navitem nav-result"
              class:is-selected={picked === i}
              aria-current={picked === i ? "true" : undefined}
              on:click={() => openResult(r, i)}
              on:keydown={(e) => resultKey(e, i)}
            >
              <span class="nav-result__label">{r.label}</span>
              <span class="nav-result__where">{r.where}</span>
            </button>
          {:else}
            <p class="nav-empty">Nothing matches “{query.trim()}”. Try another word, like “dark” or “wifi”.</p>
          {/each}
        </div>
      {:else}
        {#each NAV as g (g.title)}
          <div class="ewe-sidenav__group" role="group" aria-labelledby="nav-{g.title}">
            <div class="ewe-sidenav__heading" id="nav-{g.title}">{g.title}</div>
            {#each g.pages as p (p.id)}
              <button
                bind:this={navItems[p.id]}
                class="ewe-navitem"
                class:is-selected={$pane === p.id}
                aria-current={$pane === p.id ? "page" : undefined}
                title={p.label}
                on:click={() => go(p.id)}
                on:keydown={(e) => navKey(e, p.id)}
              >
                <Icon name={p.icon} />
                <span class="ewe-navitem__label">{p.label}</span>
              </button>
            {/each}
          </div>
        {/each}
      {/if}

      <!-- ewe's version, not this app's: Settings is part of the desktop
           rather than a separate product. -->
      <div class="ewe-sidenav__foot">
        <div class="ewe-sidenav__version">ewe {$version}</div>
      </div>
    </nav>

    <!-- the pane: the main landmark, inset from the window's edges. Each
         page draws its own header and scroll area (ui/Page.svelte). -->
    <main class="ewe-appwin__pane" bind:this={paneEl}>
      {#if $errorMsg}
        <Alert tone="danger" banner dismiss={() => errorMsg.set("")}>
          <span class="error-text" title={$errorMsg}>{$errorMsg}</span>
        </Alert>
      {/if}
      <svelte:component this={VIEWS[current.id]} />
    </main>
  </div>
  <Toasts />
</Tooltip.Provider>
