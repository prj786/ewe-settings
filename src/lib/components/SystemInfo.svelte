<script>
  import { onMount } from "svelte";
  import * as api from "../api.js";
  import { version, errorMsg, shellRestarting, restartShellNow } from "../stores.js";
  import { addons, loadAddons, openAddons } from "../addons.js";
  import KV from "./ui/KV.svelte";

  let d = null;
  let browserName = "";
  onMount(async () => {
    loadAddons();
    try {
      d = await api.diagnostics();
    } catch {}
    if (d && d.browser) {
      try {
        browserName = (await api.appNames([d.browser]))[d.browser] || "";
      } catch {}
    }
  });

  // the session's services, in words: what each one is for, and whether
  // it runs (a word and an icon, never color alone)
  const SERVICES = [
    ["Graphical session", "gsession", "The desktop session itself"],
    ["Desktop portal", "portal", "Lets apps open files and share things"],
    ["Screen sharing", "portal_hypr", "Screenshots and screen sharing"],
    ["File pickers", "portal_gtk", "The Open and Save windows"]
  ];
  const running = (s) => s === "active";
  const kernelText = (k) => String(k || "").replace(/-arch\d+-\d+$/, "") || "—";

  let confirmRestart = false;

  // Plugins (ewe 0.25): Komble installs them and holds their settings
  // (Options); this is the way there from Settings, plus what is on right
  // now. On an older ewe the list is unknown.
  $: installedAddons = ($addons.plugins || []).filter((p) => p.enabled && p.valid !== false);
  const browseAddons = () => openAddons().catch((e) => errorMsg.set(String(e)));
  import Page from "./ui/Page.svelte";
  import Group from "./ui/Group.svelte";
  import Row from "./ui/Row.svelte";
  import Icon from "./ui/Icon.svelte";
  import { Dialog } from "bits-ui";
</script>

<Page title="About this computer" desc="What this computer runs, and whether the desktop's services are up.">
  <Group title="This computer">
    <KV k="ewe" v={$version || "unknown"} />
    {#if d}
      <KV k="Hyprland" v={d.hypr || "—"} hint={d.hypr_raw || d.hypr} />
      <KV k="Linux" v={kernelText(d.kernel)} hint={d.kernel} />
      <KV k="Graphics driver" v={d.gpu || "—"} />
      <KV k="Memory" v={d.mem || "—"} />
      <KV k="Disk" v={d.disk || "—"} />
    {:else}
      <Row sub="Checking…" />
    {/if}
  </Group>

  <Group title="Plugins">
    <Row title="Plugins" sub="Extras like the dock, music and mail. You add them, and set them up, in Komble.">
      <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" on:click={browseAddons}>Open Komble</button>
    </Row>
    {#if $addons.loaded && !$addons.legacy}
      <KV k="On right now" v={installedAddons.length ? installedAddons.map((p) => p.name || p.id).join(", ") : "None"} />
    {/if}
  </Group>

  {#if d}
    <Group title="Session health">
      {#each SERVICES as [label, key, sub] (key)}
        <Row title={label} {sub}>
          <span class="kv-value">{running(d[key]) ? "Running" : d[key] ? "Not running" : "—"}</span>
          <Icon name={running(d[key]) ? "success" : "alert"} tone={running(d[key]) ? "success" : "danger"} />
        </Row>
      {/each}
      <KV k="Default browser" v={browserName || (d.browser ? d.browser.replace(/\.desktop$/, "") : "—")} />
    </Group>
  {/if}

  <Group title="Shell">
    <Row title="Restart the shell" sub="The bar, the dock and the panels start again. Your windows stay open.">
      <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" disabled={$shellRestarting} on:click={() => (confirmRestart = true)}>
        {$shellRestarting ? "Restarting…" : "Restart…"}
      </button>
    </Row>
  </Group>
</Page>

<!-- Restarting blinks the whole desktop, so it asks first -->
<Dialog.Root open={confirmRestart} onOpenChange={(v) => !v && (confirmRestart = false)}>
  <Dialog.Portal>
    <Dialog.Overlay class="scrim" />
    <Dialog.Content class="ewe-dialog is-floating">
      <div class="ewe-dialog__head">
        <span class="ewe-dialog__icon"><Icon name="refresh" /></span>
        <div class="ewe-dialog__titles">
          <Dialog.Title class="ewe-dialog__title">Restart the shell?</Dialog.Title>
          <Dialog.Description class="ewe-dialog__desc">
            The bar, the dock and the panels go away for a moment and come back. Your windows stay open.
          </Dialog.Description>
        </div>
      </div>
      <div class="ewe-dialog__foot">
        <Dialog.Close class="ewe-btn ewe-btn--secondary">Cancel</Dialog.Close>
        <button class="ewe-btn ewe-btn--primary" on:click={() => ((confirmRestart = false), restartShellNow())}>Restart</button>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
