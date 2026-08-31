<script>
  import { onMount, onDestroy } from "svelte";
  import * as api from "../api.js";
  import { errorMsg, flashApplied } from "../stores.js";
  import Card from "./ui/Card.svelte";
  import ToggleRow from "./ui/ToggleRow.svelte";
  import SelectRow from "./ui/SelectRow.svelte";

  let info = null;
  let zones = [];
  let now = new Date();
  const tick = setInterval(() => (now = new Date()), 1000);
  onDestroy(() => clearInterval(tick));

  async function refresh() {
    try {
      info = await api.timeInfo();
    } catch (e) {
      errorMsg.set(String(e));
    }
  }
  onMount(async () => {
    refresh();
    try {
      zones = await api.listTimezones();
    } catch (e) {
      errorMsg.set(String(e));
    }
  });

  // region → zone, the picker every distro ships ("Asia" → "Tbilisi")
  $: regions = [...new Set(zones.map((z) => z.split("/")[0]))];
  $: curRegion = pickedRegion || (info?.timezone || "UTC").split("/")[0];
  let pickedRegion = "";
  $: regionZones = zones.filter((z) => z.split("/")[0] === curRegion);
  const cityName = (z) => z.split("/").slice(1).join("/").replaceAll("_", " ") || z;

  async function setZone(z) {
    try {
      await api.setTimezone(z);
      flashApplied(`Timezone → ${z}`);
      setTimeout(refresh, 400);
    } catch (e) {
      errorMsg.set(String(e));
    }
  }

  async function toggleAuto() {
    const on = !(info?.auto ?? true);
    try {
      await api.setAutoTimezone(on);
      flashApplied(on ? "Automatic timezone on — detecting…" : "Timezone pinned");
      pickedRegion = "";
      // auto-on kicks a two-provider detection; give it time to land
      setTimeout(refresh, on ? 9000 : 400);
      setTimeout(refresh, 400);
    } catch (e) {
      errorMsg.set(String(e));
    }
  }

  async function toggleNtp() {
    const on = !(info?.ntp ?? true);
    try {
      await api.setNtp(on);
      flashApplied(on ? "Network time on" : "Network time off");
      setTimeout(refresh, 400);
    } catch (e) {
      errorMsg.set(String(e));
    }
  }

  const timeText = (d) =>
    d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
  const dateText = (d) =>
    d.toLocaleDateString([], { weekday: "long", year: "numeric", month: "long", day: "numeric" });
</script>

<div class="mx-auto max-w-3xl space-y-6 p-5 sm:p-8">
  <h1 class="text-lg font-semibold">Time &amp; Place</h1>

  <section>
    <Card>
      <div class="px-4 py-5 text-center">
        <div class="font-mono text-4xl font-semibold tabular-nums">{timeText(now)}</div>
        <div class="mt-1 text-sm text-zinc-400 dark:text-zinc-500">{dateText(now)}</div>
        {#if info}
          <div class="mt-2 text-xs text-zinc-400 dark:text-zinc-500">
            {info.timezone}
            {#if info.ntp}&nbsp;·&nbsp;{info.ntpSynced ? "clock synced" : "syncing…"}{/if}
          </div>
        {/if}
      </div>
    </Card>
  </section>

  <section>
    <div class="section-title">Timezone</div>
    <Card>
      <ToggleRow
        title="Set timezone automatically"
        sub="From network location, on every new connection. Two independent providers must agree before the zone moves; VPNs pause detection."
        on={info?.auto ?? true}
        toggled={toggleAuto}
      />
      <SelectRow
        label="Region"
        options={regions.map((r) => ({ label: r, value: r }))}
        value={curRegion}
        dim={info?.auto ?? true}
        picked={(r) => (pickedRegion = r)}
      />
      <SelectRow
        label="Zone"
        options={regionZones.map((z) => ({ label: cityName(z), value: z }))}
        value={info?.timezone || ""}
        dim={info?.auto ?? true}
        width="w-64"
        picked={setZone}
      />
    </Card>
    <p class="mt-2 text-xs text-zinc-400 dark:text-zinc-500">
      Picking a zone by hand pins it: automatic detection stays out of the way until you turn it
      back on. The clock itself is always network-synced (below) — the zone is the only thing that
      goes stale when you travel.
    </p>
  </section>

  <section>
    <div class="section-title">Clock</div>
    <Card>
      <ToggleRow
        title="Network time (NTP)"
        sub="Keep the clock synced via systemd-timesyncd."
        on={info?.ntp ?? true}
        toggled={toggleNtp}
      />
    </Card>
  </section>
</div>
