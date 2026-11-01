<script>
  import { onMount, onDestroy } from "svelte";
  import * as api from "../api.js";
  import { addons, loadAddons, hasAddon } from "../addons.js";
  loadAddons();
  // mention the ewe.vpn plugin only while it is installed and on
  $: vpnPlugin = hasAddon($addons, "ewe.vpn");
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { errorMsg, flashApplied } from "../stores.js";
  import Toggle from "./ui/Toggle.svelte";
  import Sheet from "./ui/Sheet.svelte";
  import KV from "./ui/KV.svelte";
  import ToggleRow from "./ui/ToggleRow.svelte";
  import NetDetails from "./NetDetails.svelte";

  let st = null;
  let pwTarget = ""; // SSID awaiting a password
  let pwText = "";
  let busy = "";
  let timer;

  // saved Wi-Fi / wired profiles, each with a details sheet (NetDetails)
  let conns = [];
  let detailsUuid = "";
  async function refreshConns() {
    try {
      conns = await api.netConnections();
    } catch (e) {
      /* the page still works without the list */
    }
  }

  async function refresh() {
    try {
      st = await api.netStatus();
    } catch (e) {
      errorMsg.set(String(e));
    }
    refreshConns();
  }

  // the machine's name on the network (hostnamectl; polkit may ask)
  let hostname = "";
  let hostDraft = "";
  let hostBusy = false;
  api
    .hostnameGet()
    .then((h) => (hostname = hostDraft = h))
    .catch(() => {});
  async function saveHostname() {
    hostBusy = true;
    try {
      await api.hostnameSet(hostDraft.trim());
      hostname = hostDraft = hostDraft.trim();
      flashApplied(`This computer is now ${hostname}`);
    } catch (e) {
      errorMsg.set(String(e));
    }
    hostBusy = false;
  }
  onMount(() => {
    refresh();
    timer = setInterval(refresh, 8000);
  });
  onDestroy(() => clearInterval(timer));

  async function toggleWifi() {
    try {
      await api.wifiSet(!st.wifiOn);
      setTimeout(refresh, 1500);
    } catch (e) {
      errorMsg.set(String(e));
    }
  }

  async function connect(w) {
    if (w.active) return;
    // a saved profile joins without a prompt — NetworkManager has the key.
    // Only a network with no profile, or one whose key sits in a secret agent
    // ewe does not have, opens the password row.
    const saved = (st.saved && st.saved[w.ssid]) || null;
    const needsKey = w.sec && w.sec !== "" && (!saved || saved.psk === "agent");
    if (needsKey && pwText === "" && pwTarget !== w.ssid) {
      pwTarget = w.ssid;
      pwText = "";
      return;
    }
    busy = w.ssid;
    try {
      await api.wifiConnect(w.ssid, pwText || null, saved ? saved.name : null);
      flashApplied(`Connected to ${w.ssid}`);
      pwTarget = "";
      pwText = "";
    } catch (e) {
      errorMsg.set(String(e));
    }
    busy = "";
    setTimeout(refresh, 1200);
  }

  // ── VPN credentials, inline ──
  // Nothing in ewe is a NetworkManager secret agent, so a profile without
  // stored secrets fails with "secrets were required … --ask". The row then
  // opens a credentials form; the secrets are stored IN the profile
  // (password-flags=0, a root-only file) and the toggle works from then on.
  let credTarget = ""; // VPN name whose credentials form is open
  let credUser = "";
  let credPass = "";
  let credPsk = "";
  let credNeedsPsk = false;
  let credError = "";

  async function askCredentials(v) {
    credTarget = v.name;
    credError = "";
    credPass = "";
    credPsk = "";
    credNeedsPsk = false;
    credUser = "";
    try {
      const info = await api.vpnInfo(v.name);
      credNeedsPsk = !!info.needsPsk;
      credUser = info.user || "";
    } catch (e) {
      /* the form still works without the hint */
    }
  }
  function closeCredentials() {
    credTarget = "";
    credPass = "";
    credPsk = "";
    credError = "";
  }
  async function saveCredentials(v) {
    if (!credUser || !credPass) {
      credError = "Enter the user name and password.";
      return;
    }
    busy = v.name;
    credError = "";
    try {
      await api.vpnSetSecrets(v.name, credUser, credPass, credPsk || null);
      await api.connectionSet(v.name, true);
      flashApplied(`${v.name} connected`);
      closeCredentials();
    } catch (e) {
      credError = String(e);
    }
    busy = "";
    setTimeout(refresh, 1200);
  }

  async function toggleWired() {
    if (!st.wired || st.wired.state === "unavailable") return;
    const on = !(wiredIs(st.wired.state, "connected") || wiredIs(st.wired.state, "connecting"));
    busy = "wired";
    try {
      await api.wiredSet(st.wired.dev, on);
      flashApplied(on ? "Wired connected" : "Wired off");
    } catch (e) {
      errorMsg.set(String(e));
    }
    busy = "";
    setTimeout(refresh, 1200);
  }

  // a DEVICE state (nmcli device) is "connecting (getting IP configuration)",
  // "connected (externally)" … — the bare word never matches on its own
  const wiredIs = (s, w) => typeof s === "string" && s.startsWith(w);
  // what NetworkManager says, in words a person uses
  const stateWord = (s) =>
    s === "activated" ? "Connected" : s === "activating" ? "Connecting…" : s === "deactivating" ? "Disconnecting…" : s;
  const typeWord = (t) =>
    t === "802-11-wireless" ? "Wi-Fi" : t === "802-3-ethernet" ? "Wired" : t === "wireguard" ? "WireGuard" : t === "vpn" ? "VPN" : t;

  async function vpnToggle(v) {
    busy = v.name;
    try {
      await api.connectionSet(v.name, !v.active);
      flashApplied(v.active ? `${v.name} disconnected` : `${v.name} connected`);
    } catch (e) {
      const msg = String(e);
      if (!v.active && /secrets|--ask|agent/i.test(msg)) {
        busy = "";
        await askCredentials(v);
        return;
      }
      errorMsg.set(msg);
    }
    busy = "";
    setTimeout(refresh, 1200);
  }

  // ── Add VPN ──
  let addKind = "l2tp"; // l2tp | openvpn | wireguard
  let addName = "";
  let addGateway = "";
  let addUser = "";
  let addPass = "";
  let addPsk = "";
  let addPath = "";
  let addBusy = false;
  let addError = "";

  async function pickFile() {
    try {
      const sel = await openDialog({
        multiple: false,
        filters:
          addKind === "openvpn"
            ? [{ name: "OpenVPN profile", extensions: ["ovpn", "conf"] }]
            : [{ name: "WireGuard config", extensions: ["conf"] }],
      });
      if (typeof sel === "string") addPath = sel;
    } catch (e) {
      addError = String(e);
    }
  }
  let addOpen = false;
  function openAddVpn() {
    addError = "";
    addOpen = true;
  }
  async function addVpn() {
    addError = "";
    addBusy = true;
    try {
      if (addKind === "l2tp") {
        await api.vpnAddL2tp(addName.trim(), addGateway.trim(), addUser, addPass, addPsk || null);
        flashApplied(`${addName.trim()} added`);
      } else {
        await api.vpnImport(addKind, addPath.trim());
        flashApplied("VPN added");
      }
      addName = "";
      addGateway = "";
      addUser = "";
      addPass = "";
      addPsk = "";
      addPath = "";
      addOpen = false;
    } catch (e) {
      addError = String(e);
    }
    addBusy = false;
    setTimeout(refresh, 800);
  }

  // signal strength as the Wi-Fi glyph's arcs (and its value in words for
  // screen readers), not a row of block characters
  const bars = (s) => (s >= 67 ? "wifi" : s >= 34 ? "wifiHigh" : s >= 15 ? "wifiLow" : "wifiZero");
  // nearby networks: the connected one and the strongest few, the rest on request
  const FEW = 6;
  let allWifi = false;
  $: wifiShown = st && st.wifi ? (allWifi ? st.wifi : st.wifi.slice(0, FEW)) : [];
  $: activeSsid = st && st.wifi ? st.wifi.find((w) => w.active)?.ssid : "";
  import Page from "./ui/Page.svelte";
  import Group from "./ui/Group.svelte";
  import Row from "./ui/Row.svelte";
  import Icon from "./ui/Icon.svelte";
  import * as Select from "./ui/select/index.js";
  const KINDS = [
    { value: "l2tp", label: "L2TP/IPsec" },
    { value: "openvpn", label: "OpenVPN, from a file" },
    { value: "wireguard", label: "WireGuard, from a file" }
  ];
</script>

<Page title="Network" desc="Wi-Fi, the wired connection, VPNs and this computer's name.">
  {#if !st}
    <Group title="Wi-Fi"><Row title="Wi-Fi" sub="Checking the network…" /></Group>
  {:else}
    {#if st.hasWifi}
      <Group title="Wi-Fi">
        <ToggleRow
          title="Wi-Fi"
          sub={st.wifiOn ? (activeSsid ? `Connected to ${activeSsid}` : "On, not connected") : "Off"}
          on={st.wifiOn}
          toggled={toggleWifi}
        />
        {#if st.wifiOn}
          {#each wifiShown as w (w.ssid)}
            <div>
              <button
                class="ewe-net net-row"
                class:is-active={w.active}
                aria-current={w.active || undefined}
                on:click={() => connect(w)}
              >
                <Icon name={bars(w.signal)} label="Signal {w.signal}%" />
                <span class="ewe-net__name">{w.ssid}</span>
                {#if w.active}
                  <span class="ewe-badge ewe-badge--success"><span class="ewe-badge__label">Connected</span></span>
                {:else if busy === w.ssid}
                  <span class="ewe-spinner ewe-spinner--sm" aria-hidden="true"></span>
                  <span class="text-xs text-dim">Connecting…</span>
                {/if}
                {#if w.sec}<Icon name="lock" class="ewe-net__lock" label="Needs a password" />{/if}
              </button>
              {#if pwTarget === w.ssid && !w.active}
                <form class="ewe-net__join net-join" on:submit|preventDefault={() => connect(w)}>
                  <Icon name="lock" size="sm" />
                  <!-- svelte-ignore a11y_autofocus -->
                  <input
                    class="ewe-input"
                    type="password"
                    placeholder="Password for {w.ssid}"
                    aria-label="Password for {w.ssid}"
                    autofocus
                    bind:value={pwText}
                  />
                  <button class="ewe-btn ewe-btn--primary ewe-btn--sm" type="submit">Connect</button>
                </form>
              {/if}
            </div>
          {:else}
            <Row sub="No networks found nearby." />
          {/each}
          {#if st.wifi.length > FEW}
            <div class="row-pad">
              <button class="ewe-btn ewe-btn--ghost ewe-btn--sm" on:click={() => (allWifi = !allWifi)}>
                <Icon name={allWifi ? "caretUp" : "caretDown"} />{allWifi ? "Show fewer" : `Show all ${st.wifi.length} networks`}
              </button>
            </div>
          {/if}
        {/if}
      </Group>
    {/if}

    {#if st.wired}
      <Group title="Wired">
        <Row
          title="Wired"
          sub={st.wired.state === "unavailable" ? "No cable plugged in"
            : wiredIs(st.wired.state, "connected") ? "Connected"
            : wiredIs(st.wired.state, "connecting") || busy === "wired" ? "Connecting…"
            : "Off"}
          dim={st.wired.state === "unavailable"}
          onRow={st.wired.state === "unavailable" ? null : toggleWired}
        >
          <Toggle
            label="Wired"
            on={wiredIs(st.wired.state, "connected") || wiredIs(st.wired.state, "connecting")}
            disabled={st.wired.state === "unavailable"}
            toggled={toggleWired}
          />
        </Row>
      </Group>
    {/if}

    <Group title="Active connections">
      {#each st.active.filter((c) => c.type !== "loopback") as c (c.name + c.dev)}
        <KV k={c.name} v={`${typeWord(c.type)} · ${stateWord(c.state)}`} />
      {:else}
        <Row sub="Nothing connected." />
      {/each}
      {#each st.ips as ip (ip)}
        <KV k="IP address" v={ip} mono />
      {/each}
    </Group>

    <!-- Saved Wi-Fi and wired profiles: each opens a sheet with its IP,
         DNS, MAC and MTU settings (NetDetails) -->
    <Group title="Saved networks" desc="IP addresses, DNS servers and more for each network.">
      {#each conns as c (c.uuid)}
        <button class="ewe-row ewe-row--interactive net-row" on:click={() => (detailsUuid = c.uuid)}>
          <span class="ewe-row__lead"><Icon name={c.type === "802-11-wireless" ? "wifi" : "cable"} /></span>
          <span class="ewe-row__text">
            <span class="ewe-row__title">{c.name}</span>
            <span class="ewe-row__desc">{typeWord(c.type)}{c.active ? " · Connected" : ""}</span>
          </span>
          <span class="ewe-row__trail"><Icon name="caretRight" /></span>
        </button>
      {:else}
        <Row sub="No saved networks yet. They appear here once you connect." />
      {/each}
    </Group>

    <Group title="VPN" desc={vpnPlugin ? "You can also switch them from Quick settings." : ""}>
      <svelte:fragment slot="action">
        <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" data-setting="Add VPN" on:click={openAddVpn}><Icon name="plus" />Add VPN…</button>
      </svelte:fragment>
      {#each st.vpn as v (v.name)}
        <Row title={v.name} sub={v.active ? "Connected" : ""} setting={v.name === st.vpn[0].name ? "VPN" : ""}>
          {#if busy === v.name}
            <span class="ewe-spinner ewe-spinner--sm" role="status" aria-label="Working…"></span>
          {:else}
            <Toggle label={v.name} on={v.active} toggled={() => vpnToggle(v)} />
          {/if}
        </Row>
        {#if credTarget === v.name && !v.active}
          <form class="net-form" on:submit|preventDefault={() => saveCredentials(v)}>
            <p class="ewe-field__helper">
              This VPN needs your user name and password once. They're kept with it, so the switch works from
              then on.
            </p>
            <label class="ewe-field">
              <span class="ewe-field__label">User name</span>
              <!-- svelte-ignore a11y_autofocus -->
              <input class="ewe-input" autofocus bind:value={credUser} />
            </label>
            <label class="ewe-field">
              <span class="ewe-field__label">Password</span>
              <input class="ewe-input" type="password" bind:value={credPass} />
            </label>
            {#if credNeedsPsk}
              <label class="ewe-field">
                <span class="ewe-field__label">Pre-shared key <span class="ewe-field__optional">optional</span></span>
                <input class="ewe-input" type="password" bind:value={credPsk} />
              </label>
            {/if}
            {#if credError}
              <p class="ewe-field__helper ewe-field__helper--error"><Icon name="alert" />{credError}</p>
            {/if}
            <div class="flex justify-end gap-2">
              <button class="ewe-btn ewe-btn--secondary" type="button" on:click={closeCredentials}>Cancel</button>
              <button class="ewe-btn ewe-btn--primary" type="submit" disabled={busy === v.name}>
                {busy === v.name ? "Connecting…" : "Connect"}
              </button>
            </div>
          </form>
        {/if}
      {:else}
        <Row title="VPN" sub="No VPNs yet." />
      {/each}
    </Group>

    <Group title="This computer">
      <Row title="Name on the network" sub="What other devices see. Letters, digits and dashes.">
        <form class="flex items-center gap-2" on:submit|preventDefault={saveHostname}>
          <input class="ewe-input ewe-input--sm font-mono host-input" aria-label="Name on the network" bind:value={hostDraft} />
          <button
            class="ewe-btn ewe-btn--secondary ewe-btn--sm"
            type="submit"
            disabled={hostBusy || !hostDraft.trim() || hostDraft.trim() === hostname}
          >
            {hostBusy ? "Renaming…" : "Rename"}
          </button>
        </form>
      </Row>
    </Group>

    {#if st.sshHosts.length}
      <Group title="SSH hosts" desc="Computers you've set up to reach over SSH.">
        {#each st.sshHosts as h (h)}
          <div class="ewe-row" data-setting={h === st.sshHosts[0] ? "SSH hosts" : undefined}>
            <span class="ewe-row__lead"><Icon name="terminal" /></span>
            <div class="ewe-row__text"><div class="ewe-row__title font-mono">{h}</div></div>
          </div>
        {/each}
      </Group>
    {/if}
  {/if}
</Page>

<!-- Add a VPN: an L2TP/IPsec profile from its four facts (no file), or an
     OpenVPN / WireGuard file. Definitions sync through the one file;
     secrets stay in the profile. -->
<Sheet open={addOpen} title="Add a VPN" onClose={() => (addOpen = false)}>
  <form id="add-vpn" class="net-form" on:submit|preventDefault={addVpn}>
    <div class="ewe-field">
      <span class="ewe-field__label">Type</span>
      <Select.Root type="single" value={addKind} onValueChange={(v) => (addKind = v)}>
        <Select.Trigger class="self-start select-trigger" aria-label="VPN type">
          {KINDS.find((k) => k.value === addKind)?.label}
        </Select.Trigger>
        <Select.Content>
          {#each KINDS as k (k.value)}
            <Select.Item value={k.value} label={k.label} />
          {/each}
        </Select.Content>
      </Select.Root>
    </div>
    {#if addKind === "l2tp"}
      <label class="ewe-field">
        <span class="ewe-field__label">Name</span>
        <input class="ewe-input" placeholder="Work" bind:value={addName} />
      </label>
      <label class="ewe-field">
        <span class="ewe-field__label">Server</span>
        <input class="ewe-input" placeholder="vpn.example.com" bind:value={addGateway} />
      </label>
      <label class="ewe-field">
        <span class="ewe-field__label">User name</span>
        <input class="ewe-input" bind:value={addUser} />
      </label>
      <label class="ewe-field">
        <span class="ewe-field__label">Password</span>
        <input class="ewe-input" type="password" bind:value={addPass} />
      </label>
      <label class="ewe-field">
        <span class="ewe-field__label">Pre-shared key <span class="ewe-field__optional">optional</span></span>
        <input class="ewe-input" type="password" bind:value={addPsk} />
        <span class="ewe-field__helper">Leave it empty if your VPN has none.</span>
      </label>
    {:else}
      <div class="ewe-field">
        <span class="ewe-field__label">File</span>
        <div class="flex gap-2">
          <input class="ewe-input flex-1" placeholder="work.{addKind === 'openvpn' ? 'ovpn' : 'conf'}" aria-label="The VPN's file" bind:value={addPath} />
          <button class="ewe-btn ewe-btn--secondary" type="button" on:click={pickFile}>Choose…</button>
        </div>
        <span class="ewe-field__helper">The file your VPN provider gave you.</span>
      </div>
    {/if}
    {#if addError}
      <p class="ewe-field__helper ewe-field__helper--error"><Icon name="alert" />{addError}</p>
    {/if}
    <p class="ewe-field__helper">
      {addKind === "l2tp"
        ? "The VPN syncs to your other computers. Its password and key never do."
        : "The first time you connect, it asks for your user name and password once."}
    </p>
  </form>
  <svelte:fragment slot="footer">
    <button class="ewe-btn ewe-btn--secondary" on:click={() => (addOpen = false)}>Cancel</button>
    <button
      class="ewe-btn ewe-btn--primary"
      type="submit"
      form="add-vpn"
      disabled={addBusy ||
        (addKind === "l2tp" ? !addName.trim() || !addGateway.trim() || !addUser || !addPass : !addPath.trim())}
    >
      {addBusy ? "Adding…" : "Add VPN"}
    </button>
  </svelte:fragment>
</Sheet>

<NetDetails uuid={detailsUuid} onClose={() => (detailsUuid = "")} changed={refresh} />
