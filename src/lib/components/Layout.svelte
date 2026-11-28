<script>
  import { onMount } from "svelte";
  import * as api from "../api.js";
  import { prefs, errorMsg } from "../stores.js";
  import { theme, refreshTheme } from "../theme.js";
  import { layout, loadLayout, applyGaps, setTiling, setPrefs, setLayoutMode, setColumnWidth } from "../overrides.js";
  import { addons, loadAddons, hasAddon, addonInstalled, openAddons, openPluginOptions } from "../addons.js";
  import ToggleRow from "./ui/ToggleRow.svelte";
  import SliderRow from "./ui/SliderRow.svelte";

  onMount(() => {
    loadLayout();
    loadAddons();
  });

  // The dock is a plugin (ewe.dock, installed from Komble) and its settings
  // are its own since Dock 1.1.0 — Komble → Plugins → Dock → Options. This
  // page keeps one row that says so and opens that dialog.
  $: dockOn = hasAddon($addons, "ewe.dock");
  $: dockInstalled = addonInstalled($addons, "ewe.dock");
  const getPlugins = () => openAddons().catch((e) => errorMsg.set(String(e)));
  const dockOptions = () => openPluginOptions("ewe.dock").catch((e) => errorMsg.set(String(e)));

  function setLayout(patch) {
    layout.update((l) => ({ ...l, ...patch }));
    applyGaps();
  }

  const layoutModes = [
    ["dwindle", "Dwindle", "Each new window splits the focused one in half. The classic tiling feel."],
    ["master", "Master", "One large window on the left, a stack on the right."],
    ["scrolling", "Scrolling", "Windows sit on an endless strip that scrolls sideways. Super+Alt+[ and ] move along it."],
    ["monocle", "Monocle", "One window at a time, filling the screen. Super+Tab turns the stack."]
  ];

  const iconSizes = [
    ["small", "Small"],
    ["normal", "Normal"],
    ["large", "Large"]
  ];
  // [desktop.bar] icon_size in ewe.conf: the bar has no height of its own,
  // it is its icons plus padding (44 / 48 / 56). The live value is what
  // ewe-theme last built from; text size 130% moves the icons a size up.
  $: textScale = Number(($theme && $theme.input && $theme.input.text_scale) ?? 100);
  async function setBarIcons(v) {
    try {
      await api.setConf("desktop.bar.icon_size", v);
      await refreshTheme();
    } catch (e) {
      errorMsg.set(String(e));
    }
  }

  // [desktop.bar] layout (ewe 0.52): split = the top bar and the dock apart;
  // merged = one full-width bar at the bottom, like a Windows taskbar — the
  // dock's buttons on its left, status and clock on its right. The shell
  // reads it as barLayout (user-theme.json, through ewe-conf absorb).
  const barLayouts = [
    ["split", "Separate"],
    ["merged", "Merged"]
  ];
  $: merged = $prefs.barLayout === "merged";
  const setBarLayout = (v) => setPrefs({ barLayout: v });

  // Top bar — what the status row shows. Absent = shown, so a fresh install
  // and an old user-theme.json both mean "everything"; the shell reads the
  // same object (Globals.barShow). Identity (workspace, window title) and
  // Komble's update state are not optional.
  const barItems = [
    ["sound", "Sound", "The volume, or the headset or headphones when sound goes there"],
    ["mic", "Microphone in use", "An accent microphone while an app is listening"],
    ["wifi", "Network", "Wi-Fi, or the wired connection, while connected"],
    ["bluetooth", "Bluetooth", "While Bluetooth is on; filled when a device is connected"],
    ["battery", "Battery", "Icon and percentage, on laptops"],
    ["power", "Power profile", "A leaf, a balance or a speedometer"],
    ["keyboard", "Keyboard layout", "US or GE. Click it to switch."],
    ["tray", "System tray", "Icons from apps that ask for one"],
    ["tiling", "Tiling or floating", "The layout switch"]
  ];
  // A plugin's place in the bar (the camera, the scissors, the system
  // monitor…) is that plugin's Show in bar switch in Komble's Options
  // (`ewe-plugin bar`) — one switch, where the rest of its settings are.
  const barShows = (key) => !($prefs.barShow && $prefs.barShow[key] === false);
  const setBarShow = (key, on) => setPrefs({ barShow: { ...($prefs.barShow || {}), [key]: on } });
  import Page from "./ui/Page.svelte";
  import Group from "./ui/Group.svelte";
  import Row from "./ui/Row.svelte";
  import Seg from "./ui/Seg.svelte";
</script>

<Page title="Layout" desc="How windows tile, the space around them, the top bar and the dock.">
  <Group title="Windows">
    <ToggleRow
      title="Tiling"
      sub="Off: every new window opens floating, like a stacking desktop"
      on={$prefs.tilingEnabled !== false}
      toggled={() => setTiling(!($prefs.tilingEnabled !== false))}
    />
    <Row
      title="Tiling style"
      sub={(layoutModes.find(([id]) => id === ($layout.mode || "dwindle")) || layoutModes[0])[2]}
    >
      <Seg
        label="Tiling style"
        options={layoutModes.map(([id, label]) => [id, label])}
        value={$layout.mode || "dwindle"}
        picked={setLayoutMode}
      />
    </Row>
    {#if $layout.mode === "scrolling"}
      <SliderRow label="Column width" value={Math.round(($layout.columnWidth ?? 0.5) * 100)} from={20} to={100} unit="%" indent moved={(v) => setColumnWidth(Math.round(v) / 100)} />
    {/if}
  </Group>

  <Group title="Gaps and borders">
    <SliderRow label="Inner gaps" sub="Between windows" value={$layout.gapsIn} from={0} to={40} unit=" px" moved={(v) => setLayout({ gapsIn: Math.round(v) })} />
    <SliderRow label="Outer gaps" sub="Between windows and the screen's edge" value={$layout.gapsOut} from={0} to={60} unit=" px" moved={(v) => setLayout({ gapsOut: Math.round(v) })} />
    <SliderRow label="Border width" value={$layout.borderSize} from={0} to={8} unit=" px" moved={(v) => setLayout({ borderSize: Math.round(v) })} />
    <SliderRow label="Corner radius" value={$layout.rounding} from={0} to={24} unit=" px" moved={(v) => setLayout({ rounding: Math.round(v) })} />
  </Group>

  <Group title="Top bar">
    <ToggleRow
      title="Top bar"
      sub="Workspace, window title, status and clock. Super+Shift+B hides it until you sign out."
      on={$prefs.barEnabled !== false}
      toggled={() => setPrefs({ barEnabled: !($prefs.barEnabled !== false) })}
    />
    <Row
      title="Bar and dock"
      sub={merged
        ? "One full-width bar at the bottom, like Windows: the dock's apps and workspaces on the left, status and clock on the right"
        : "The bar at the top, the dock at the bottom"}
      dim={$prefs.barEnabled === false}
    >
      <Seg
        label="Bar and dock"
        options={barLayouts}
        value={merged ? "merged" : "split"}
        disabled={$prefs.barEnabled === false}
        picked={setBarLayout}
      />
    </Row>
    <Row
      title="Bar icons"
      sub={textScale >= 130
        ? "The bar grows with its icons. At text size 130% they're a size larger."
        : merged
          ? "The merged bar is 48\u00a0px tall, 56\u00a0px with large icons"
          : "The bar grows with its icons: 36, 40 or 48\u00a0px tall"}
      dim={$prefs.barEnabled === false}
    >
      <Seg
        label="Bar icons"
        options={iconSizes}
        value={($theme && $theme.input && $theme.input.bar_icon_size) || "normal"}
        disabled={$prefs.barEnabled === false}
        picked={setBarIcons}
      />
    </Row>
    <ToggleRow
      title="Fold the extras"
      sub="Tray icons, plugin buttons and the tiling switch hide behind an arrow. Click the arrow to show them."
      dim={$prefs.barEnabled === false}
      on={$prefs.barCollapse === true}
      toggled={() => setPrefs({ barCollapse: !($prefs.barCollapse === true) })}
    />
  </Group>

  <Group title="Status row" desc="What the right side of the top bar shows.">
    {#each barItems as [key, title, sub] (key)}
      <ToggleRow {title} {sub} dim={$prefs.barEnabled === false} on={barShows(key)} toggled={() => setBarShow(key, !barShows(key))} />
    {/each}
    <Row title="Plugins in the bar" sub="Each plugin has its own Show in bar switch, with its options in Komble">
      <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" on:click={getPlugins}>Open Komble</button>
    </Row>
  </Group>

  <!-- The dock's settings are the Dock plugin's own (Komble → Options);
       one row says where, and opens that dialog. -->
  <Group title="Dock">
    {#if !$addons.loaded}
      <Row title="Dock" sub="Checking…" />
    {:else if dockInstalled}
      <Row
        title="Dock"
        sub={!dockOn
          ? "The Dock plugin is installed but turned off. Turn it on in Komble."
          : merged
            ? "It sits in the merged bar. Which displays show it is the Dock plugin's own setting"
            : "Auto-hide, icon size and which displays show it are the Dock plugin's own settings"}
      >
        <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" on:click={dockOptions}>Dock settings</button>
      </Row>
    {:else}
      <Row title="Dock" sub="Pinned apps, the launcher and your workspaces at the bottom of the screen. It's a plugin you add in Komble.">
        <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" on:click={getPlugins}>Open Komble</button>
      </Row>
    {/if}
  </Group>
</Page>
