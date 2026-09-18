<script>
  import { onMount } from "svelte";
  import * as api from "../api.js";
  import { prefs, errorMsg } from "../stores.js";
  import { theme, refreshTheme } from "../theme.js";
  import { layout, loadLayout, applyGaps, setTiling, setPrefs, setLayoutMode, setColumnWidth } from "../overrides.js";
  import ToggleRow from "./ui/ToggleRow.svelte";
  import SliderRow from "./ui/SliderRow.svelte";

  onMount(loadLayout);

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
  // v3: the bar is normal (48px) or large (64px), [desktop.bar] size in
  // ewe.conf — it replaces the old status-glyph icon size. The live value is
  // what ewe-theme last built from.
  const barSizes = [
    ["normal", "Normal"],
    ["large", "Large"]
  ];
  async function setBarSize(v) {
    try {
      await api.setConf("desktop.bar.size", v);
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
  // The camera and the scissors are plugins since ewe 0.21 (ewe.screenshot,
  // ewe.clipboard) — Komble → Plugins turns them off, not this list.
  const barShows = (key) => !($prefs.barShow && $prefs.barShow[key] === false);
  const setBarShow = (key, on) => setPrefs({ barShow: { ...($prefs.barShow || {}), [key]: on } });
  import Page from "./ui/Page.svelte";
  import Group from "./ui/Group.svelte";
  import Row from "./ui/Row.svelte";
  import Seg from "./ui/Seg.svelte";
</script>

<Page title="Layout and dock" desc="How windows tile, the space around them, the top bar and the dock.">
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
    <Row title="Bar size" sub="Normal is 48px tall; large is 64px, with bigger modules and icons." dim={$prefs.barEnabled === false}>
      <Seg
        label="Bar size"
        options={barSizes}
        value={($theme && $theme.input && $theme.input.bar_size) || "normal"}
        disabled={$prefs.barEnabled === false}
        picked={setBarSize}
      />
    </Row>
  </Group>

  <Group title="What the status row shows" class={$prefs.barEnabled === false ? "pointer-events-none" : ""}>
    {#each barItems as [key, title, sub] (key)}
      <ToggleRow {title} {sub} dim={$prefs.barEnabled === false} on={barShows(key)} toggled={() => setBarShow(key, !barShows(key))} />
    {/each}
  </Group>

  <Group title="Dock">
    <ToggleRow
      title="Dock"
      sub="The dock at the bottom, with pinned apps, the launcher and Places."
      on={$prefs.dockEnabled !== false}
      toggled={() => setPrefs({ dockEnabled: !($prefs.dockEnabled !== false) })}
    />
    <ToggleRow
      title="Intelligent auto-hide"
      sub="Slides away when a window needs the space; comes back when you point at the bottom edge."
      dim={$prefs.dockEnabled === false}
      on={!!$prefs.dockAutohide}
      toggled={() => setPrefs({ dockAutohide: !$prefs.dockAutohide })}
    />
    <Row title="Icon size" sub="How big the dock buttons and workspace groups are." dim={$prefs.dockEnabled === false}>
      <Seg
        label="Dock icon size"
        options={iconSizes}
        value={$prefs.dockIconSize || "normal"}
        disabled={$prefs.dockEnabled === false}
        picked={(id) => setPrefs({ dockIconSize: id })}
      />
    </Row>
  </Group>
</Page>
