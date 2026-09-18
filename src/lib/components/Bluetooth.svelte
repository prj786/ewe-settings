<script>
  // Bluetooth — bluez through `ewe-bt` (src-tauri/src/backend.rs). The
  // pairing DIALOG is not here: it is the shell's (BtAgent + BtPairing), the
  // one place bluez's "confirm 123456?" / "type the PIN" is answered — a pair
  // started from this pane pops it on the desktop. This pane only lists,
  // starts and forgets.
  import { onMount, onDestroy } from "svelte";
  import * as api from "../api.js";
  import { errorMsg, flashApplied, shellUp } from "../stores.js";
  import ToggleRow from "./ui/ToggleRow.svelte";
  import IconBtn from "./ui/IconBtn.svelte";
  import Icon from "./ui/Icon.svelte";

  let st = null;        // {adapter:{powered,discoverable,discovering,alias}, devices:[…]}
  let busy = "";        // address with a pair/connect/disconnect in flight
  let scanning = false;
  let timer;
  let scanTimer;
  const SCAN_SECS = 30;

  async function refresh() {
    try {
      st = await api.btStatus();
    } catch (e) {
      // no adapter / no ewe-bt: the pane says so instead of a banner every 3 s
      st = { adapter: null, devices: [], error: String(e) };
    }
  }
  onMount(() => {
    refresh();
    timer = setInterval(refresh, 3000);
  });
  onDestroy(() => {
    clearInterval(timer);
    clearTimeout(scanTimer);
  });

  async function togglePower() {
    try {
      await api.btPower(!st.adapter.powered);
      setTimeout(refresh, 800);
    } catch (e) {
      errorMsg.set(String(e));
    }
  }
  async function toggleDiscoverable() {
    try {
      await api.btDiscoverable(!st.adapter.discoverable);
      setTimeout(refresh, 500);
    } catch (e) {
      errorMsg.set(String(e));
    }
  }
  async function scan() {
    if (scanning) return;
    try {
      await api.btScan(SCAN_SECS);
      scanning = true;
      clearTimeout(scanTimer);
      scanTimer = setTimeout(() => (scanning = false), SCAN_SECS * 1000 + 500);
    } catch (e) {
      errorMsg.set(String(e));
    }
  }

  async function pair(d) {
    busy = d.address;
    errorMsg.set("");
    try {
      const r = await api.btPair(d.address);
      if (r && r.warning) errorMsg.set(`${d.name} paired, but did not connect: ${r.warning}`);
      else flashApplied(`${d.name} connected`);
    } catch (e) {
      errorMsg.set(`${d.name}: ${e}`);
    }
    busy = "";
    refresh();
  }
  async function toggleConnect(d) {
    busy = d.address;
    errorMsg.set("");
    try {
      if (d.connected) {
        await api.btDisconnect(d.address);
        flashApplied(`${d.name} disconnected`);
      } else {
        await api.btConnect(d.address);
        flashApplied(`${d.name} connected`);
      }
    } catch (e) {
      errorMsg.set(`${d.name}: ${e}`);
    }
    busy = "";
    refresh();
  }
  async function forget(d) {
    busy = d.address;
    try {
      await api.btForget(d.address);
      flashApplied(`${d.name} forgotten`);
    } catch (e) {
      errorMsg.set(`${d.name}: ${e}`);
    }
    busy = "";
    refresh();
  }

  // bluez's Icon string → the same Lucide glyphs the shell's BtAgent.glyph uses
  const GLYPH = {
    "audio-headset": 0xe5bd,
    "audio-headphones": 0xe0f1,
    "audio-card": 0xe166,
    "audio-speakers": 0xe166,
    "input-keyboard": 0xe284,
    "input-mouse": 0xe28e,
    "input-gaming": 0xe0df,
    "input-tablet": 0xe17e,
    phone: 0xe163,
    computer: 0xe1cd,
    "video-display": 0xe11d,
    printer: 0xe141,
    "camera-photo": 0xe064,
    "camera-video": 0xe064,
    "multimedia-player": 0xe122,
    "network-wireless": 0xe1ae
  };
  const glyph = (d) => GLYPH[d.icon] || (d.connected ? 0xe1b8 : 0xe05c);
  const TRASH =
    '<path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/><line x1="10" x2="10" y1="11" y2="17"/><line x1="14" x2="14" y1="11" y2="17"/>';

  function status(d) {
    if (busy === d.address) return d.paired ? (d.connected ? "Disconnecting…" : "Connecting…") : "Pairing…";
    if (d.connected) return d.battery != null ? `Connected · ${d.battery}%` : "Connected";
    return d.paired ? "Not connected" : "";
  }

  $: powered = !!(st && st.adapter && st.adapter.powered);
  $: paired = st ? st.devices.filter((d) => d.paired) : [];
  // unpaired devices with a name — bare addresses are beacons and TVs advertising, noise
  $: nearby = st ? st.devices.filter((d) => !d.paired && d.name && d.name !== d.address) : [];
  $: if (st && st.adapter && st.adapter.discovering && !scanning) {
    // discovery someone else started (Quick Settings has the tab open) — show it
    scanning = true;
    clearTimeout(scanTimer);
    scanTimer = setTimeout(() => (scanning = false), 4000);
  }
  import Page from "./ui/Page.svelte";
  import Group from "./ui/Group.svelte";
  import Row from "./ui/Row.svelte";
  import Alert from "./ui/Alert.svelte";
</script>

<Page title="Bluetooth" desc="Pair headphones, keyboards and phones, and connect the ones you've paired.">
  {#if !st}
    <p class="note">Checking Bluetooth…</p>
  {:else if !st.adapter}
    <Alert tone="" title={st.error || "No Bluetooth adapter found"}>
      {#if !st.error}
        If this computer has one, it may be blocked. Try <code>rfkill unblock bluetooth</code>, or check
        <code>systemctl status bluetooth</code>.
      {/if}
    </Alert>
  {:else}
    <Group>
      <ToggleRow
        title="Bluetooth"
        sub={powered ? `On. This computer shows up as “${st.adapter.alias}”.` : "Off"}
        on={powered}
        toggled={togglePower}
      />
      {#if powered}
        <ToggleRow
          title="Visible to other devices"
          sub="Lets a phone or another computer find this one and start pairing from its side. Turns off by itself after 3 minutes."
          on={st.adapter.discoverable}
          toggled={toggleDiscoverable}
        />
      {/if}
    </Group>

    {#if powered}
      <Group title="My devices">
        {#each paired as d (d.address)}
          <div class="ewe-row" class:is-selected={d.connected}>
            <span class="ewe-row__lead"><Icon code={glyph(d)} /></span>
            <div class="ewe-row__text">
              <div class="ewe-row__title" class:font-medium={d.connected}>{d.name}</div>
              <div class="ewe-row__desc">{status(d)}{d.trusted ? "" : " · Not trusted"}</div>
            </div>
            <div class="ewe-row__trail">
              <button
                class="ewe-btn ewe-btn--secondary ewe-btn--sm"
                disabled={busy === d.address}
                on:click={() => toggleConnect(d)}
              >
                {d.connected ? "Disconnect" : "Connect"}
              </button>
              <IconBtn name="trash" title="Forget {d.name}" danger disabled={busy === d.address} go={() => forget(d)} />
            </div>
          </div>
        {:else}
          <Row sub="No paired devices yet. Search for one below." />
        {/each}
      </Group>

      <Group title="Nearby devices">
        <svelte:fragment slot="action">
          <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" disabled={scanning} on:click={scan}>
            {#if scanning}<span class="ewe-spinner ewe-spinner--sm" aria-hidden="true"></span>{/if}
            {scanning ? "Searching…" : "Search"}
          </button>
        </svelte:fragment>
        {#each nearby as d (d.address)}
          <div class="ewe-row">
            <span class="ewe-row__lead"><Icon code={glyph(d)} /></span>
            <div class="ewe-row__text">
              <div class="ewe-row__title">{d.name}</div>
              <div class="ewe-row__desc">
                {busy === d.address ? "Pairing… Answer the prompt on the desktop if one appears." : d.rssi != null ? `Signal ${d.rssi} dBm` : ""}
              </div>
            </div>
            <div class="ewe-row__trail">
              <button class="ewe-btn ewe-btn--primary ewe-btn--sm" disabled={busy !== ""} on:click={() => pair(d)}>
                {busy === d.address ? "Pairing…" : "Pair"}
              </button>
            </div>
          </div>
        {:else}
          <Row
            sub={scanning
              ? "Searching… Put the device in pairing mode (hold its button until it blinks)."
              : "Nothing found yet. Put the device in pairing mode, then search."}
          />
        {/each}
      </Group>

      <p class="note">
        When a device shows a code, the desktop asks you to confirm it. Paired devices are trusted, so
        they reconnect by themselves.
      </p>
      {#if !$shellUp}
        <Alert tone="warning">The ewe shell isn't running, so only devices without a code (most headphones) can pair right now.</Alert>
      {/if}
    {/if}
  {/if}
</Page>
