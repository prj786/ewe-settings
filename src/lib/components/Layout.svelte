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
    ["scrolling", "Scrolling", "Windows sit on an endless horizontal strip, like PaperWM. Scroll with Super+Alt+[ and ]."]
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

  // Top bar — what the status row shows. Absent = shown, so a fresh install
  // and an old user-theme.json both mean "everything"; the shell reads the
  // same object (Globals.barShow). Identity (workspace, window title) and
  // Komble's update state are not optional.
  const barItems = [
    ["sound", "Sound", "The volume, or the headset or headphones when sound goes there."],
    ["mic", "Microphone in use", "An accent microphone while an app has the microphone open."],
    ["wifi", "Network", "Wi-Fi, or the wired connection, while connected."],
    ["bluetooth", "Bluetooth", "While Bluetooth is on; filled when a device is connected."],
    ["battery", "Battery", "Icon and percentage, on laptops."],
    ["power", "Power profile", "A leaf, a balance or a speedometer."],
    ["keyboard", "Keyboard layout", "US or GE. Click to switch."],
    ["tray", "System tray", "Icons from apps that ask for one."],
    ["tiling", "Tiling or floating", "The layout switch."]
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

<Page title="Layout" desc="How windows tile, the space around them, and the top bar.">
  <Group title="Window behavior">
    <ToggleRow
      title="Tiling"
      sub="Off: every new window opens floating, like a stacking desktop. Applies after a config reload."
      on={$prefs.tilingEnabled !== false}
      toggled={() => setTiling(!($prefs.tilingEnabled !== false))}
    />
  </Group>

  <Group title="Window layout">
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
      <SliderRow label="Column width" value={Math.round(($layout.columnWidth ?? 0.5) * 100)} from={20} to={100} unit="%" moved={(v) => setColumnWidth(Math.round(v) / 100)} />
    {/if}
  </Group>

  <Group title="Gaps and borders">
    <SliderRow label="Inner gaps" value={$layout.gapsIn} from={0} to={40} unit=" px" moved={(v) => setLayout({ gapsIn: Math.round(v) })} />
    <SliderRow label="Outer gaps" value={$layout.gapsOut} from={0} to={60} unit=" px" moved={(v) => setLayout({ gapsOut: Math.round(v) })} />
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
      title="Bar icons"
      sub={textScale >= 130
        ? "The bar grows with its icons. At text size 130%, they’re one size larger."
        : "The bar grows with its icons: 36, 40 or 48\u00a0px tall."}
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
  </Group>

  <Group title="What the status row shows" class={$prefs.barEnabled === false ? "pointer-events-none" : ""}>
    {#each barItems as [key, title, sub] (key)}
      <ToggleRow {title} {sub} dim={$prefs.barEnabled === false} on={barShows(key)} toggled={() => setBarShow(key, !barShows(key))} />
    {/each}
    <Row title="Plugins in the bar" sub="Each plugin's Show in bar switch is in its Options, in Komble → Plugins.">
      <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" on:click={getPlugins}>Open Plugins</button>
    </Row>
  </Group>

  <!-- The dock's settings are the Dock plugin's own (Komble → Options);
       one row says where, and opens that dialog. -->
  <Group title="Dock">
    {#if !$addons.loaded}
      <Row sub="Checking…" />
    {:else if dockInstalled}
      <Row
        title="Dock options"
        sub={dockOn
          ? "Auto-hide and icon size are the Dock plugin's own settings, in Komble."
          : "The Dock plugin is installed but turned off. Turn it on in Komble; its settings are there too."}
      >
        <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" on:click={dockOptions}>Dock options</button>
      </Row>
    {:else}
      <Row
        title="The dock is a plugin."
        sub="Pinned apps, the launcher and your workspaces at the bottom of the screen. Install it in Komble → Plugins; its settings come with it."
      >
        <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" on:click={getPlugins}>Get plugins</button>
      </Row>
    {/if}
  </Group>
</Page>
