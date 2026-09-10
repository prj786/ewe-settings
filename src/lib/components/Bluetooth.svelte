<script>
  // Bluetooth — bluez through `ewe-bt` (src-tauri/src/backend.rs). The
  // pairing DIALOG is not here: it is the shell's (BtAgent + BtPairing), the
  // one place bluez's "confirm 123456?" / "type the PIN" is answered — a pair
  // started from this pane pops it on the desktop. This pane only lists,
  // starts and forgets.
  import { onMount, onDestroy } from "svelte";
  import * as api from "../api.js";
  import { errorMsg, flashApplied, shellUp } from "../stores.js";
  import Card from "./ui/Card.svelte";
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
</script>

<div class="mx-auto max-w-3xl space-y-6 p-5 sm:p-8">
  <h1 class="text-lg font-semibold">Bluetooth</h1>

  {#if !st}
    <p class="text-sm text-dim">Checking Bluetooth…</p>
  {:else if !st.adapter}
    <Card>
      <div class="px-4 py-3 text-sm text-dim">
        {st.error || "No Bluetooth adapter found."}
        {#if !st.error}
          If this machine has one, it may be blocked: <code>rfkill unblock bluetooth</code>, or check
          <code>systemctl status bluetooth</code>.
        {/if}
      </div>
    </Card>
  {:else}
    <section>
      <Card>
        <ToggleRow
          title="Bluetooth"
          sub={powered ? `On · this computer is “${st.adapter.alias}”` : "Off"}
          on={powered}
          toggled={togglePower}
        />
        {#if powered}
          <ToggleRow
            title="Visible to other devices"
            sub="Lets a phone or another computer find this one and start the pairing from its side. Turns itself off after 3 minutes."
            on={st.adapter.discoverable}
            toggled={toggleDiscoverable}
          />
        {/if}
      </Card>
    </section>

    {#if powered}
      <section>
        <div class="section-title">My devices</div>
        <Card>
          {#each paired as d (d.address)}
            <div class="flex items-center gap-3 px-4 py-2.5">
              <Icon code={glyph(d)} size={16} class={d.connected ? "text-[var(--brand-fg-1)]" : "text-dim"} />
              <div class="min-w-0 flex-1">
                <div class="truncate text-sm {d.connected ? 'font-semibold' : ''}">{d.name}</div>
                <div class="text-xs text-dim">{status(d)}{d.trusted ? "" : " · not trusted"}</div>
              </div>
              <button
                class="btn-ghost !py-1 text-xs"
                disabled={busy === d.address}
                on:click={() => toggleConnect(d)}
              >
                {d.connected ? "Disconnect" : "Connect"}
              </button>
              <IconBtn icon={TRASH} title="Forget this device" danger disabled={busy === d.address} go={() => forget(d)} />
            </div>
          {:else}
            <div class="px-4 py-3 text-sm text-dim">No paired devices yet — search below.</div>
          {/each}
        </Card>
      </section>

      <section>
        <div class="section-title flex items-center justify-between">
          <span>Nearby devices</span>
          <button class="btn-ghost !py-0.5 text-xs" disabled={scanning} on:click={scan}>
            {scanning ? "Searching…" : "Search"}
          </button>
        </div>
        <Card>
          {#each nearby as d (d.address)}
            <div class="flex items-center gap-3 px-4 py-2.5">
              <Icon code={glyph(d)} size={16} class="text-dim" />
              <div class="min-w-0 flex-1">
                <div class="truncate text-sm">{d.name}</div>
                <div class="text-xs text-dim">
                  {busy === d.address ? "Pairing… answer the prompt on the desktop if one appears" : d.rssi != null ? `Signal ${d.rssi} dBm` : ""}
                </div>
              </div>
              <button class="btn-primary !py-1 text-xs" disabled={busy !== ""} on:click={() => pair(d)}>
                {busy === d.address ? "Pairing…" : "Pair"}
              </button>
            </div>
          {:else}
            <div class="px-4 py-3 text-sm text-dim">
              {scanning
                ? "Searching… put the device in pairing mode (hold its button until it blinks)."
                : "Nothing found yet. Put the device in pairing mode, then Search."}
            </div>
          {/each}
        </Card>
      </section>

      <p class="text-xs text-dim">
        When a device shows a code, the desktop asks you to confirm it. Paired devices are trusted, so they
        reconnect on their own.
        {#if !$shellUp}
          <span class="text-warning">The ewe shell is not running — only devices without a code (most headphones) can pair right now.</span>
        {/if}
      </p>
    {/if}
  {/if}
</div>
