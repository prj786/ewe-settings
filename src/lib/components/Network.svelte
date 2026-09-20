<script>
  import { onMount, onDestroy } from "svelte";
  import * as api from "../api.js";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { errorMsg, flashApplied } from "../stores.js";
  import Toggle from "./ui/Toggle.svelte";
  import KV from "./ui/KV.svelte";

  let st = null;
  let pwTarget = ""; // SSID awaiting a password
  let pwText = "";
  let busy = "";
  let timer;

  async function refresh() {
    try {
      st = await api.netStatus();
    } catch (e) {
      errorMsg.set(String(e));
    }
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
  async function addVpn() {
    addError = "";
    addBusy = true;
    try {
      if (addKind === "l2tp") {
        await api.vpnAddL2tp(addName.trim(), addGateway.trim(), addUser, addPass, addPsk || null);
        flashApplied(`${addName.trim()} added`);
      } else {
        await api.vpnImport(addKind, addPath.trim());
        flashApplied("VPN profile imported");
      }
      addName = "";
      addGateway = "";
      addUser = "";
      addPass = "";
      addPsk = "";
      addPath = "";
    } catch (e) {
      addError = String(e);
    }
    addBusy = false;
    setTimeout(refresh, 800);
  }

  // signal strength as the Wi-Fi glyph's arcs (and its value in words for
  // screen readers), not a row of block characters
  const bars = (s) => (s >= 67 ? "wifi" : s >= 34 ? "wifiHigh" : s >= 15 ? "wifiLow" : "wifiZero");
  import Page from "./ui/Page.svelte";
  import Group from "./ui/Group.svelte";
  import Row from "./ui/Row.svelte";
  import Icon from "./ui/Icon.svelte";
  import * as Select from "./ui/select/index.js";
  const KINDS = [
    { value: "l2tp", label: "L2TP/IPsec" },
    { value: "openvpn", label: "OpenVPN file (.ovpn)" },
    { value: "wireguard", label: "WireGuard file (.conf)" }
  ];
</script>

<Page title="Networking" desc="Wi-Fi, the wired connection and VPNs, through NetworkManager.">
  {#if !st}
    <p class="note">Checking the network…</p>
  {:else}
    {#if st.wired}
      <Group title="Wired">
        <Row
          icon="cable"
          title="Wired"
          sub={st.wired.state === "unavailable" ? "No cable"
            : wiredIs(st.wired.state, "connected") ? "Connected"
            : wiredIs(st.wired.state, "connecting") || busy === "wired" ? "Connecting…"
            : "Off"}
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

    {#if st.hasWifi}
      <Group title="Wi-Fi">
        <Row icon="wifi" title="Wi-Fi" sub={st.wifiOn ? (st.wifi.find((w) => w.active)?.ssid ? `Connected to ${st.wifi.find((w) => w.active).ssid}` : "On") : "Off"}>
          <Toggle label="Wi-Fi" on={st.wifiOn} toggled={toggleWifi} />
        </Row>
        {#if st.wifiOn}
          <div class="ewe-list__divider"></div>
          {#each st.wifi as w (w.ssid)}
            <button
              class="ewe-net net-row"
              class:is-active={w.active}
              aria-current={w.active || undefined}
              on:click={() => connect(w)}
            >
              <Icon name={bars(w.signal)} label="Signal {w.signal}%" />
              <span class="ewe-net__name">{w.ssid}</span>
              {#if w.active}
                <span class="text-xs">Connected</span>
              {:else if busy === w.ssid}
                <span class="ewe-spinner ewe-spinner--sm" aria-hidden="true"></span>
                <span class="text-xs text-dim">Connecting…</span>
              {/if}
              {#if w.sec}<Icon name="lock" class="ewe-net__lock" label="Secured" />{/if}
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
          {:else}
            <Row sub="No networks found." />
          {/each}
        {/if}
      </Group>
    {/if}

    <Group title="Active connections">
      {#each st.active.filter((c) => c.type !== "loopback") as c (c.name + c.dev)}
        <KV k={c.name} v={`${typeWord(c.type)} · ${c.dev} · ${stateWord(c.state)}`} />
      {:else}
        <Row sub="Nothing connected." />
      {/each}
      {#each st.ips as ip (ip)}
        <KV k="IP address" v={ip} mono />
      {/each}
    </Group>

    <Group title="VPN">
      {#each st.vpn as v (v.name)}
        <Row icon="shield" title={v.name} sub={v.active ? "Connected" : ""}>
          {#if busy === v.name}
            <span class="ewe-spinner ewe-spinner--sm" role="status" aria-label="Working…"></span>
          {:else}
            <Toggle label={v.name} on={v.active} toggled={() => vpnToggle(v)} />
          {/if}
        </Row>
        {#if credTarget === v.name && !v.active}
          <form class="net-form" on:submit|preventDefault={() => saveCredentials(v)}>
            <p class="ewe-field__helper">
              This VPN needs your credentials once. They're kept in the profile, so the switch works from
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
                <span class="ewe-field__label">Pre-shared key (IPsec) <span class="ewe-field__optional">optional</span></span>
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
        <Row sub="No VPNs yet. Add one below." />
      {/each}
    </Group>

    <!-- Add VPN: an L2TP/IPsec profile from its four facts (no file), or an
         OpenVPN / WireGuard file import. Definitions sync through the one
         file; secrets stay in the profile. -->
    <Group title="Add a VPN">
      <form class="net-form" on:submit|preventDefault={addVpn}>
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
          <div class="net-grid">
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
            <label class="ewe-field net-grid__wide">
              <span class="ewe-field__label">Pre-shared key (IPsec) <span class="ewe-field__optional">optional</span></span>
              <input class="ewe-input" type="password" bind:value={addPsk} />
              <span class="ewe-field__helper">Leave it empty if your VPN has none.</span>
            </label>
          </div>
        {:else}
          <div class="ewe-field">
            <span class="ewe-field__label">File</span>
            <div class="flex gap-2">
              <input class="ewe-input flex-1" placeholder="/home/you/work.{addKind === 'openvpn' ? 'ovpn' : 'conf'}" aria-label="Path to the file" bind:value={addPath} />
              <button class="ewe-btn ewe-btn--secondary" type="button" on:click={pickFile}>Choose…</button>
            </div>
          </div>
        {/if}
        {#if addError}
          <p class="ewe-field__helper ewe-field__helper--error"><Icon name="alert" />{addError}</p>
        {/if}
        <div class="flex items-center justify-between gap-3">
          <span class="ewe-field__helper">
            {addKind === "l2tp"
              ? "Kept in the profile. The definition syncs; the secrets never do."
              : "Imported into NetworkManager. Credentials are asked for once, the first time you connect."}
          </span>
          <button
            class="ewe-btn ewe-btn--primary"
            type="submit"
            disabled={addBusy ||
              (addKind === "l2tp" ? !addName.trim() || !addGateway.trim() || !addUser || !addPass : !addPath.trim())}
          >
            {addBusy ? "Adding…" : addKind === "l2tp" ? "Add VPN" : "Import VPN"}
          </button>
        </div>
      </form>
    </Group>

    {#if st.sshHosts.length}
      <Group title="SSH hosts (~/.ssh/config)">
        {#each st.sshHosts as h (h)}
          <div class="ewe-row ewe-row--dense"><span class="ewe-row__lead"><Icon name="terminal" /></span><span class="ewe-row__title font-mono">{h}</span></div>
        {/each}
      </Group>
    {/if}

    <p class="note">
      NetworkManager manages these. You can also switch VPNs from the VPN card in Quick settings; the
      first connection asks for credentials once and keeps them in the profile.
    </p>
  {/if}
</Page>
