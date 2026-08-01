<script>
  import { onMount, onDestroy } from "svelte";
  import * as api from "../api.js";
  import { errorMsg, flashApplied } from "../stores.js";
  import Card from "./ui/Card.svelte";
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
    if (w.sec && w.sec !== "" && pwText === "" && pwTarget !== w.ssid) {
      // secured network: open the password row (a saved profile connects
      // without one — submitting empty tries that first)
      pwTarget = pwTarget === w.ssid ? "" : w.ssid;
      pwText = "";
      return;
    }
    busy = w.ssid;
    try {
      await api.wifiConnect(w.ssid, pwText || null);
      flashApplied(`Connected to ${w.ssid}`);
      pwTarget = "";
      pwText = "";
    } catch (e) {
      errorMsg.set(String(e));
    }
    busy = "";
    setTimeout(refresh, 1200);
  }

  async function vpnToggle(v) {
    busy = v.name;
    try {
      await api.connectionSet(v.name, !v.active);
      flashApplied(v.active ? `${v.name} disconnected` : `${v.name} connected`);
    } catch (e) {
      errorMsg.set(String(e));
    }
    busy = "";
    setTimeout(refresh, 1200);
  }

  const bars = (s) => (s >= 75 ? "▂▄▆█" : s >= 50 ? "▂▄▆" : s >= 25 ? "▂▄" : "▂");
</script>

<div class="mx-auto max-w-3xl space-y-6 p-5 sm:p-8">
  <h1 class="text-lg font-semibold">Networking</h1>

  {#if !st}
    <p class="text-sm text-zinc-400">Checking network state…</p>
  {:else}
    {#if st.hasWifi}
      <section>
        <div class="section-title">Wi-Fi</div>
        <Card>
          <div class="flex items-center justify-between px-4 py-3">
            <span class="text-sm font-medium">Wi-Fi</span>
            <Toggle on={st.wifiOn} toggled={toggleWifi} />
          </div>
          {#if st.wifiOn}
            {#each st.wifi as w (w.ssid)}
              <div>
                <button
                  class="flex w-full items-center gap-3 px-4 py-2.5 text-left hover:bg-white/5"
                  on:click={() => connect(w)}
                >
                  <span class="w-10 shrink-0 font-mono text-xs text-zinc-400">{bars(w.signal)}</span>
                  <span class="min-w-0 flex-1 truncate text-sm {w.active ? 'font-semibold' : ''}">{w.ssid}</span>
                  {#if w.sec}<span class="text-xs text-zinc-500">🔒</span>{/if}
                  {#if w.active}
                    <span class="text-xs font-medium" style="color: var(--accent)">Connected</span>
                  {:else if busy === w.ssid}
                    <span class="text-xs text-zinc-400">Connecting…</span>
                  {/if}
                </button>
                {#if pwTarget === w.ssid && !w.active}
                  <form
                    class="flex gap-2 px-4 pb-3"
                    on:submit|preventDefault={() => connect(w)}
                  >
                    <!-- svelte-ignore a11y_autofocus -->
                    <input
                      class="input flex-1"
                      type="password"
                      placeholder="Password (empty = use saved profile)"
                      autofocus
                      bind:value={pwText}
                    />
                    <button class="btn-primary !py-1 text-xs" type="submit">Connect</button>
                  </form>
                {/if}
              </div>
            {:else}
              <div class="px-4 py-3 text-sm text-zinc-400">No networks found.</div>
            {/each}
          {/if}
        </Card>
      </section>
    {/if}

    <section>
      <div class="section-title">Active connections</div>
      <Card>
        {#each st.active as c (c.name + c.dev)}
          <KV k={c.name} v={`${c.type} · ${c.dev} · ${c.state}`} />
        {:else}
          <div class="px-4 py-3 text-sm text-zinc-400">Nothing connected.</div>
        {/each}
        {#each st.ips as ip (ip)}
          <KV k="IP" v={ip} />
        {/each}
      </Card>
    </section>

    {#if st.vpn.length}
      <section>
        <div class="section-title">VPN</div>
        <Card>
          {#each st.vpn as v (v.name)}
            <div class="flex items-center justify-between gap-3 px-4 py-2.5">
              <span class="min-w-0 flex-1 truncate text-sm {v.active ? 'font-semibold' : ''}">{v.name}</span>
              {#if busy === v.name}
                <span class="text-xs text-zinc-400">…</span>
              {:else}
                <Toggle on={v.active} toggled={() => vpnToggle(v)} />
              {/if}
            </div>
          {/each}
        </Card>
      </section>
    {/if}

    {#if st.sshHosts.length}
      <section>
        <div class="section-title">SSH hosts (~/.ssh/config)</div>
        <Card>
          {#each st.sshHosts as h (h)}
            <div class="px-4 py-2 font-mono text-xs text-zinc-400">{h}</div>
          {/each}
        </Card>
      </section>
    {/if}

    <p class="text-xs text-zinc-400 dark:text-zinc-500">
      Managed by NetworkManager. VPN profiles are added with
      <code class="rounded bg-zinc-800 px-1">nmcli connection import</code> or a network applet;
      once saved they can be toggled here.
    </p>
  {/if}
</div>
