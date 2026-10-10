<script>
  import { onMount } from "svelte";
  import * as api from "../api.js";
  import { version, errorMsg } from "../stores.js";
  import { addons, loadAddons, openAddons } from "../addons.js";
  import KV from "./ui/KV.svelte";

  let d = null;
  onMount(async () => {
    loadAddons();
    try {
      d = await api.diagnostics();
    } catch {}
  });

  // Plugins (ewe 0.25): Komble installs them and holds their settings
  // (Options); this is the way there from Settings, plus what is on right
  // now. On an older ewe the list is unknown.
  $: installedAddons = ($addons.plugins || []).filter((p) => p.enabled && p.valid !== false);
  const browseAddons = () => openAddons().catch((e) => errorMsg.set(String(e)));
  import Page from "./ui/Page.svelte";
  import Group from "./ui/Group.svelte";
  import Row from "./ui/Row.svelte";
  import Icon from "./ui/Icon.svelte";
</script>

<Page title="System" desc="What this computer runs, and whether the session's services are up.">
  <Group>
    <KV k="ewe" v={$version || "unknown"} />
    {#if d}
      <KV k="Hyprland" v={d.hypr || "—"} />
      <KV k="Kernel" v={d.kernel || "—"} />
      <KV k="GPU driver" v={d.gpu || "—"} />
      <KV k="Memory" v={d.mem || "—"} />
      <KV k="Disk (/)" v={d.disk || "—"} />
    {:else}
      <Row sub="Checking…" />
    {/if}
  </Group>

  <Group title="Plugins">
    <Row title="Plugins" sub="Extra features like the dock, music, the phone and mail. Komble installs and removes them, and each one's settings are in its Options there.">
      <button class="ewe-btn ewe-btn--secondary" on:click={browseAddons}>Open Plugins</button>
    </Row>
    {#if $addons.loaded && !$addons.legacy}
      <KV k="On right now" v={installedAddons.length ? installedAddons.map((p) => p.name || p.id).join(", ") : "None"} />
    {/if}
  </Group>

  {#if d}
    <Group title="Session health">
      {#each [
        ["Graphical session", d.gsession],
        ["Desktop portal", d.portal],
        ["Portal (Hyprland)", d.portal_hypr],
        ["Portal (GTK)", d.portal_gtk]
      ] as [label, state] (label)}
        <Row title={label}>
          {state || "—"}
          <!-- status is a word and an icon, never color alone -->
          <Icon name={state === "active" ? "success" : "alert"} tone={state === "active" ? "success" : "danger"} />
        </Row>
      {/each}
      <KV k="Default browser" v={d.browser || "—"} />
    </Group>
  {/if}
</Page>
