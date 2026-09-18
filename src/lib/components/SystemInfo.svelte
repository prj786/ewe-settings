<script>
  import { onMount } from "svelte";
  import * as api from "../api.js";
  import { version } from "../stores.js";
  import KV from "./ui/KV.svelte";

  let d = null;
  onMount(async () => {
    try {
      d = await api.diagnostics();
    } catch {}
  });
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
