<script>
  import { onMount, onDestroy } from "svelte";
  import * as api from "../api.js";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
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
      credError = "Username and password are required.";
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

  const bars = (s) => (s >= 75 ? "▂▄▆█" : s >= 50 ? "▂▄▆" : s >= 25 ? "▂▄" : "▂");
</script>

<div class="mx-auto max-w-3xl space-y-6 p-5 sm:p-8">
  <h1 class="text-lg font-semibold">Networking</h1>

  {#if !st}
    <p class="text-sm text-dim">Checking network state…</p>
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
                  <span class="w-10 shrink-0 font-mono text-xs text-dim">{bars(w.signal)}</span>
                  <span class="min-w-0 flex-1 truncate text-sm {w.active ? 'font-semibold' : ''}">{w.ssid}</span>
                  {#if w.sec}<span class="text-xs text-dim">🔒</span>{/if}
                  {#if w.active}
                    <span class="text-xs font-medium" style="color: var(--accent)">Connected</span>
                  {:else if busy === w.ssid}
                    <span class="text-xs text-dim">Connecting…</span>
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
              <div class="px-4 py-3 text-sm text-dim">No networks found.</div>
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
          <div class="px-4 py-3 text-sm text-dim">Nothing connected.</div>
        {/each}
        {#each st.ips as ip (ip)}
          <KV k="IP" v={ip} />
        {/each}
      </Card>
    </section>

    <section>
      <div class="section-title">VPN</div>
      {#if st.vpn.length}
        <Card>
          {#each st.vpn as v (v.name)}
            <div>
              <div class="flex items-center justify-between gap-3 px-4 py-2.5">
                <span class="min-w-0 flex-1 truncate text-sm {v.active ? 'font-semibold' : ''}">{v.name}</span>
                {#if busy === v.name}
                  <span class="text-xs text-dim">…</span>
                {:else}
                  <Toggle on={v.active} toggled={() => vpnToggle(v)} />
                {/if}
              </div>
              {#if credTarget === v.name && !v.active}
                <form class="space-y-2 px-4 pb-3" on:submit|preventDefault={() => saveCredentials(v)}>
                  <p class="text-xs text-dim">
                    This VPN needs your credentials once — they are kept in the profile, so the toggle
                    works from then on.
                  </p>
                  <!-- svelte-ignore a11y_autofocus -->
                  <input class="input w-full" placeholder="Username" autofocus bind:value={credUser} />
                  <input class="input w-full" type="password" placeholder="Password" bind:value={credPass} />
                  {#if credNeedsPsk}
                    <input
                      class="input w-full"
                      type="password"
                      placeholder="Pre-shared key (IPsec) — leave empty if none"
                      bind:value={credPsk}
                    />
                  {/if}
                  {#if credError}
                    <div class="text-xs text-warning">{credError}</div>
                  {/if}
                  <div class="flex justify-end gap-2">
                    <button class="btn-ghost !py-1 text-xs" type="button" on:click={closeCredentials}>Cancel</button>
                    <button class="btn-primary !py-1 text-xs" type="submit" disabled={busy === v.name}>
                      {busy === v.name ? "Connecting…" : "Connect"}
                    </button>
                  </div>
                </form>
              {/if}
            </div>
          {/each}
        </Card>
      {/if}

      <!-- Add VPN: an L2TP/IPsec profile from its four facts (no file), or an
           OpenVPN / WireGuard file import. Definitions sync through the one
           file; secrets stay in the profile. -->
      <div class={st.vpn.length ? "mt-3" : ""}>
        <Card>
          <form class="space-y-2 px-4 py-3" on:submit|preventDefault={addVpn}>
            <div class="flex items-center justify-between gap-3">
              <span class="text-sm font-medium">Add VPN</span>
              <select class="input !py-1 text-xs" bind:value={addKind}>
                <option value="l2tp">L2TP / IPsec</option>
                <option value="openvpn">OpenVPN file (.ovpn)</option>
                <option value="wireguard">WireGuard file (.conf)</option>
              </select>
            </div>
            {#if addKind === "l2tp"}
              <div class="grid grid-cols-1 gap-2 sm:grid-cols-2">
                <input class="input" placeholder="Name (e.g. Work)" bind:value={addName} />
                <input class="input" placeholder="Server (gateway)" bind:value={addGateway} />
                <input class="input" placeholder="Username" bind:value={addUser} />
                <input class="input" type="password" placeholder="Password" bind:value={addPass} />
                <input
                  class="input sm:col-span-2"
                  type="password"
                  placeholder="Pre-shared key (IPsec) — leave empty if your VPN has none"
                  bind:value={addPsk}
                />
              </div>
            {:else}
              <div class="flex gap-2">
                <input class="input flex-1" placeholder="Path to the file" bind:value={addPath} />
                <button class="btn-ghost !py-1 text-xs" type="button" on:click={pickFile}>Choose…</button>
              </div>
            {/if}
            {#if addError}
              <div class="text-xs text-warning">{addError}</div>
            {/if}
            <div class="flex items-center justify-between gap-3">
              <span class="text-xs text-dim">
                {addKind === "l2tp"
                  ? "Stored in the profile; the definition syncs, the secrets never do."
                  : "Imported into NetworkManager; credentials are asked once on first connect."}
              </span>
              <button
                class="btn-primary !py-1 text-xs"
                type="submit"
                disabled={addBusy ||
                  (addKind === "l2tp" ? !addName.trim() || !addGateway.trim() || !addUser || !addPass : !addPath.trim())}
              >
                {addBusy ? "Adding…" : addKind === "l2tp" ? "Add" : "Import"}
              </button>
            </div>
          </form>
        </Card>
      </div>
    </section>

    {#if st.sshHosts.length}
      <section>
        <div class="section-title">SSH hosts (~/.ssh/config)</div>
        <Card>
          {#each st.sshHosts as h (h)}
            <div class="px-4 py-2 font-mono text-xs text-dim">{h}</div>
          {/each}
        </Card>
      </section>
    {/if}

    <p class="text-xs text-dim dark:text-dim">
      Managed by NetworkManager. VPN profiles can also be toggled from the Control Center's VPN card;
      the first connect asks for credentials once and keeps them in the profile.
    </p>
  {/if}
</div>
