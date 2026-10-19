<script>
  /**
   * One saved Wi-Fi or wired profile, in a Sheet: what is in effect now
   * (addresses, gateway, DNS, hardware address) and what can be changed —
   * IPv4 / IPv6 (automatic, manual, off), DNS servers and search domains,
   * DNS over TLS, metered, the MAC address and the MTU. Only the keys that
   * changed are sent (net_conn_apply), and the backend checks each value
   * again before nmcli sees it.
   */
  import * as api from "../api.js";
  import { flashApplied } from "../stores.js";
  import Sheet from "./ui/Sheet.svelte";
  import Group from "./ui/Group.svelte";
  import Row from "./ui/Row.svelte";
  import KV from "./ui/KV.svelte";
  import Seg from "./ui/Seg.svelte";
  import Toggle from "./ui/Toggle.svelte";
  import Alert from "./ui/Alert.svelte";

  export let uuid = "";
  export let onClose = () => {};
  export let changed = () => {};

  let d = null; // net_conn_details()
  let f = null; // the form
  let loadError = "";
  let error = "";
  let busy = false;
  let forgetArmed = false;

  // "a, b c" → "a,b,c": how the backend writes lists, so a compare is fair
  const norm = (v) =>
    String(v || "")
      .split(/[\s,]+/)
      .filter(Boolean)
      .join(",");
  const s = (k) => (d && d.settings[k] != null ? String(d.settings[k]) : "");
  const MAC_RE = /^[0-9a-f]{2}(:[0-9a-f]{2}){5}$/i;

  $: link = d && d.type === "802-11-wireless" ? "802-11-wireless" : "802-3-ethernet";

  async function load(id) {
    loadError = "";
    error = "";
    d = null;
    f = null;
    try {
      d = await api.netConnDetails(id);
    } catch (e) {
      loadError = String(e);
      return;
    }
    const lk = d.type === "802-11-wireless" ? "802-11-wireless" : "802-3-ethernet";
    const mac = s(`${lk}.cloned-mac-address`);
    const mtu = s(`${lk}.mtu`);
    const dot = parseInt(s("connection.dns-over-tls"), 10);
    f = {
      auto: s("connection.autoconnect") !== "no",
      metered: s("connection.metered") || "unknown",
      dot: Number.isNaN(dot) ? "-1" : String(dot),
      v4: s("ipv4.method") || "auto",
      v4addr: norm(s("ipv4.addresses")).replace(/,/g, ", "),
      v4gw: s("ipv4.gateway"),
      autoDns: s("ipv4.ignore-auto-dns") !== "yes",
      dns: [norm(s("ipv4.dns")), norm(s("ipv6.dns"))].filter(Boolean).join(",").replace(/,/g, ", "),
      search: norm(s("ipv4.dns-search")).replace(/,/g, ", "),
      v6: s("ipv6.method") || "auto",
      v6addr: norm(s("ipv6.addresses")).replace(/,/g, ", "),
      v6gw: s("ipv6.gateway"),
      mac: MAC_RE.test(mac) ? "custom" : mac || "default",
      macCustom: MAC_RE.test(mac) ? mac : "",
      mtu: /^\d+$/.test(mtu) && mtu !== "0" ? mtu : ""
    };
  }
  $: if (uuid) load(uuid);

  // the form → nmcli keys; only what differs from the profile is sent
  function changes() {
    const out = {};
    const isList = (k) => k.endsWith(".addresses") || k.endsWith(".dns") || k.endsWith(".dns-search");
    const put = (k, v) => {
      if (String(v) !== (isList(k) ? norm(s(k)) : s(k))) out[k] = String(v);
    };
    put("connection.autoconnect", f.auto ? "yes" : "no");
    put("connection.metered", f.metered);
    const dot = parseInt(s("connection.dns-over-tls"), 10);
    if ((Number.isNaN(dot) ? "-1" : String(dot)) !== f.dot) out["connection.dns-over-tls"] = f.dot;

    put("ipv4.method", f.v4);
    put("ipv4.addresses", f.v4 === "manual" ? norm(f.v4addr) : "");
    put("ipv4.gateway", f.v4 === "manual" ? f.v4gw.trim() : "");
    put("ipv6.method", f.v6);
    put("ipv6.addresses", f.v6 === "manual" ? norm(f.v6addr) : "");
    put("ipv6.gateway", f.v6 === "manual" ? f.v6gw.trim() : "");

    const all = norm(f.dns).split(",").filter(Boolean);
    put("ipv4.dns", all.filter((a) => !a.includes(":")).join(","));
    put("ipv6.dns", all.filter((a) => a.includes(":")).join(","));
    put("ipv4.dns-search", norm(f.search));
    put("ipv4.ignore-auto-dns", f.autoDns ? "no" : "yes");
    put("ipv6.ignore-auto-dns", f.autoDns ? "no" : "yes");

    put(`${link}.cloned-mac-address`, f.mac === "custom" ? f.macCustom.trim() : f.mac === "default" ? "" : f.mac);
    const mtu = f.mtu.trim() === "" ? "0" : f.mtu.trim();
    const curMtu = /^\d+$/.test(s(`${link}.mtu`)) ? s(`${link}.mtu`) : "0";
    if (mtu !== curMtu) out[`${link}.mtu`] = mtu;
    return out;
  }

  function check() {
    if (f.v4 === "manual" && !norm(f.v4addr)) return "Manual IPv4 needs at least one address, like 192.168.1.20/24.";
    if (f.v6 === "manual" && !norm(f.v6addr)) return "Manual IPv6 needs at least one address.";
    if (!f.autoDns && !norm(f.dns)) return "Add a DNS server, or turn Automatic DNS back on.";
    if (f.mac === "custom" && !MAC_RE.test(f.macCustom.trim())) return "A MAC address looks like 02:1a:2b:3c:4d:5e.";
    return "";
  }

  async function save() {
    error = check();
    if (error) return;
    const c = changes();
    if (Object.keys(c).length === 0) return onClose();
    busy = true;
    try {
      await api.netConnApply(uuid, c);
      flashApplied(`${d.name} updated`);
      changed();
      onClose();
    } catch (e) {
      error = String(e);
    }
    busy = false;
  }

  // two steps: the first press arms it (4 s), the second forgets
  async function forget() {
    if (!forgetArmed) {
      forgetArmed = true;
      setTimeout(() => (forgetArmed = false), 4000);
      return;
    }
    busy = true;
    try {
      await api.netConnForget(uuid);
      flashApplied(`Forgot ${d.name}`);
      changed();
      onClose();
    } catch (e) {
      error = String(e);
    }
    busy = false;
  }

  const PRESETS = [
    { label: "Cloudflare", dns: "1.1.1.1, 1.0.0.1, 2606:4700:4700::1111" },
    { label: "Google", dns: "8.8.8.8, 8.8.4.4, 2001:4860:4860::8888" },
    { label: "Quad9", dns: "9.9.9.9, 149.112.112.112, 2620:fe::fe" }
  ];
  const usePreset = (p) => {
    f.dns = p.dns;
    f.autoDns = false;
  };

  const METHODS = [
    ["auto", "Automatic"],
    ["manual", "Manual"],
    ["disabled", "Off"]
  ];
  // a method set elsewhere (link-local, shared, ignore …) stays pickable
  const withCurrent = (cur) => (METHODS.some((o) => o[0] === cur) ? METHODS : [...METHODS, [cur, cur]]);
  const live = (k) => (d && d.live[k]) || [];
</script>

<Sheet open={!!uuid} title={d ? d.name : "Connection"} {onClose}>
  {#if loadError}
    <Alert tone="danger" title="Couldn't read this connection">{loadError}</Alert>
  {:else if !d || !f}
    <p class="note">Reading the connection…</p>
  {:else}
    <Group title="Now">
      {#if d.dev}
        <KV k="Device" v={d.dev} mono />
        {#if d.live.HWADDR}<KV k="Hardware address" v={d.live.HWADDR} mono />{/if}
        {#each live("IP4.ADDRESS") as a (a)}<KV k="IPv4 address" v={a} mono />{/each}
        {#if live("IP4.GATEWAY").length}<KV k="Gateway" v={live("IP4.GATEWAY")[0]} mono />{/if}
        {#each live("IP6.ADDRESS") as a (a)}<KV k="IPv6 address" v={a} mono />{/each}
        {#if live("IP4.DNS").length || live("IP6.DNS").length}
          <KV k="DNS servers" v={[...live("IP4.DNS"), ...live("IP6.DNS")].join(", ")} mono />
        {/if}
        {#if live("IP4.DOMAIN").length}<KV k="Domain" v={live("IP4.DOMAIN").join(", ")} mono />{/if}
      {:else}
        <Row sub="Not connected. Changes apply the next time it connects." />
      {/if}
    </Group>

    <Group title="General">
      <Row title="Connect automatically" sub="Join whenever it is in reach.">
        <Toggle label="Connect automatically" on={f.auto} toggled={(v) => (f.auto = v)} />
      </Row>
      <Row title="Metered" sub="Apps hold back large downloads on a metered connection." block>
        <Seg
          label="Metered"
          value={f.metered}
          options={[["unknown", "Automatic"], ["yes", "Yes"], ["no", "No"]]}
          picked={(v) => (f.metered = v)}
        />
      </Row>
      <Row title="MAC address" sub="What this device shows the network. Stable is a private address that stays the same on this network." block>
        <Seg
          label="MAC address"
          value={f.mac}
          options={[["default", "Default"], ["permanent", "Device"], ["stable", "Stable"], ["random", "Random"], ["custom", "Custom"]]}
          picked={(v) => (f.mac = v)}
        />
        {#if f.mac === "custom"}
          <input class="ewe-input font-mono" placeholder="02:1a:2b:3c:4d:5e" aria-label="Custom MAC address" bind:value={f.macCustom} />
        {/if}
      </Row>
    </Group>

    <Group title="IPv4">
      <Row title="Configuration" block>
        <Seg label="IPv4 configuration" value={f.v4} options={withCurrent(f.v4)} picked={(v) => (f.v4 = v)} />
      </Row>
      {#if f.v4 === "manual"}
        <div class="net-form net-grid">
          <label class="ewe-field">
            <span class="ewe-field__label">Address</span>
            <input class="ewe-input font-mono" placeholder="192.168.1.20/24" bind:value={f.v4addr} />
          </label>
          <label class="ewe-field">
            <span class="ewe-field__label">Gateway <span class="ewe-field__optional">optional</span></span>
            <input class="ewe-input font-mono" placeholder="192.168.1.1" bind:value={f.v4gw} />
          </label>
        </div>
      {/if}
    </Group>

    <Group title="DNS">
      <Row title="Automatic DNS" sub="Use the servers the network hands out, as well as any below.">
        <Toggle label="Automatic DNS" on={f.autoDns} toggled={(v) => (f.autoDns = v)} />
      </Row>
      <div class="net-form">
        <label class="ewe-field">
          <span class="ewe-field__label">Servers {#if f.autoDns}<span class="ewe-field__optional">optional</span>{/if}</span>
          <input class="ewe-input font-mono" placeholder="1.1.1.1, 2606:4700:4700::1111" bind:value={f.dns} />
          <span class="ewe-field__helper">IPv4 and IPv6, separated by commas.</span>
        </label>
        <div class="flex flex-wrap gap-2">
          {#each PRESETS as p (p.label)}
            <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" type="button" on:click={() => usePreset(p)}>{p.label}</button>
          {/each}
        </div>
        <label class="ewe-field">
          <span class="ewe-field__label">Search domains <span class="ewe-field__optional">optional</span></span>
          <input class="ewe-input font-mono" placeholder="home.arpa, corp.example.com" bind:value={f.search} />
          <span class="ewe-field__helper">Short names are looked up in these. ~. sends every lookup to this connection's servers.</span>
        </label>
      </div>
      <Row title="DNS over TLS" sub="Encrypts lookups to servers that support it. Opportunistic falls back to plain DNS." block>
        <Seg
          label="DNS over TLS"
          value={f.dot}
          options={[["-1", "Default"], ["0", "Off"], ["1", "Opportunistic"], ["2", "On"]]}
          picked={(v) => (f.dot = v)}
        />
      </Row>
    </Group>

    <Group title="IPv6">
      <Row title="Configuration" block>
        <Seg label="IPv6 configuration" value={f.v6} options={withCurrent(f.v6)} picked={(v) => (f.v6 = v)} />
      </Row>
      {#if f.v6 === "manual"}
        <div class="net-form net-grid">
          <label class="ewe-field">
            <span class="ewe-field__label">Address</span>
            <input class="ewe-input font-mono" placeholder="2001:db8::20/64" bind:value={f.v6addr} />
          </label>
          <label class="ewe-field">
            <span class="ewe-field__label">Gateway <span class="ewe-field__optional">optional</span></span>
            <input class="ewe-input font-mono" placeholder="2001:db8::1" bind:value={f.v6gw} />
          </label>
        </div>
      {/if}
    </Group>

    <Group title="Advanced">
      <div class="net-form">
        <label class="ewe-field">
          <span class="ewe-field__label">MTU <span class="ewe-field__optional">optional</span></span>
          <input class="ewe-input font-mono" inputmode="numeric" placeholder="Automatic" bind:value={f.mtu} />
          <span class="ewe-field__helper">Leave it empty unless your network needs another packet size (576–9000).</span>
        </label>
      </div>
    </Group>

    {#if error}
      <Alert tone="danger" title="Couldn't save">{error}</Alert>
    {/if}
  {/if}

  <svelte:fragment slot="footer">
    {#if d}
      <button
        class="ewe-btn {forgetArmed ? 'ewe-btn--danger' : 'ewe-btn--ghost'} mr-auto"
        type="button"
        disabled={busy}
        on:click={forget}>{forgetArmed ? `Forget ${d.name}` : "Forget"}</button
      >
    {/if}
    <button class="ewe-btn ewe-btn--secondary" type="button" on:click={onClose}>Cancel</button>
    <button class="ewe-btn ewe-btn--primary" type="button" disabled={busy || !f} on:click={save}>
      {busy ? "Saving…" : "Save"}
    </button>
  </svelte:fragment>
</Sheet>
