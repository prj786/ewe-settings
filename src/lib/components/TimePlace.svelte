<script>
  import { onMount, onDestroy } from "svelte";
  import * as api from "../api.js";
  import { errorMsg, flashApplied } from "../stores.js";
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
    refreshLocale();
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

  // ── Language ──────────────────────────────────────────────────────────────
  // The list is every UTF-8 locale glibc knows, not just the generated ones —
  // picking an ungenerated one runs locale-gen behind one polkit prompt, so
  // the user never has to know locale.gen exists.
  let loc = null;
  let langQuery = "";
  let showLang = false;
  let langBusy = false;
  async function refreshLocale() {
    try {
      loc = await api.localeInfo();
    } catch (e) {
      errorMsg.set(String(e));
    }
  }
  $: langCandidates = (loc?.choices || []).filter((c) => {
    const q = langQuery.trim().toLowerCase();
    return q === "" || c.label.toLowerCase().includes(q) || c.code.toLowerCase().includes(q);
  });

  async function setLocale(c) {
    if (langBusy || c.code === loc?.current) return;
    langBusy = true;
    if (!c.generated) flashApplied(`Generating ${c.label}…`);
    try {
      await api.setLocale(c.code);
      flashApplied(`Language → ${c.label}`);
      showLang = false;
      langQuery = "";
      await refreshLocale();
    } catch (e) {
      errorMsg.set(String(e));
    } finally {
      langBusy = false;
    }
  }

  const timeText = (d) =>
    d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
  const dateText = (d) =>
    d.toLocaleDateString([], { weekday: "long", year: "numeric", month: "long", day: "numeric" });
  import Page from "./ui/Page.svelte";
  import Group from "./ui/Group.svelte";
  import Row from "./ui/Row.svelte";
  import PickList from "./ui/PickList.svelte";
</script>

<Page title="Time and place" desc="The time zone, the clock and the language apps use.">
  <div class="ewe-list">
    <div class="clock-hero">
      <div class="clock-hero__time">{timeText(now)}</div>
      <div class="clock-hero__date">{dateText(now)}</div>
      {#if info}
        <div class="clock-hero__meta">
          {info.timezone}{#if info.ntp} · {info.ntpSynced ? "Clock synced" : "Syncing…"}{/if}
        </div>
      {/if}
    </div>
  </div>

  <Group title="Time zone">
    <ToggleRow
      title="Set the time zone automatically"
      sub="From your network location, on every new connection. Two providers must agree before it changes; a VPN pauses it."
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
      picked={setZone}
    />
    <svelte:fragment slot="after">
      <p class="note">
        Picking a zone yourself keeps it until you turn automatic back on. The clock itself always
        syncs over the network; only the zone goes stale when you travel.
      </p>
    </svelte:fragment>
  </Group>

  <Group title="Clock">
    <ToggleRow
      title="Network time"
      sub="Keep the clock synced with systemd-timesyncd (NTP)."
      on={info?.ntp ?? true}
      toggled={toggleNtp}
    />
  </Group>

  <Group title="Language">
    <Row title="Display language">
      <svelte:fragment slot="text">
        <div class="ewe-row__desc">
          {#if loc}
            <span class="font-mono">{loc.current || "Not set"}</span>{#if loc.current && !loc.generated} · not generated yet{/if}
          {:else}
            …
          {/if}
        </div>
      </svelte:fragment>
      <span>{loc?.currentLabel || "—"}</span>
      {#if !showLang}
        <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" on:click={() => (showLang = true)}>Change…</button>
      {/if}
    </Row>
    {#if showLang}
      <div class={langBusy ? "pointer-events-none" : ""} aria-busy={langBusy}>
        <PickList
          bind:query={langQuery}
          placeholder="Search languages"
          items={langCandidates.slice(0, 40).map((c) => ({
            id: c.code,
            title: c.label,
            trail: `${c.code} · ${c.generated ? "ready" : "generate"}`,
            selected: c.code === loc?.current,
            c
          }))}
          pick={(it) => setLocale(it.c)}
        />
        <div class="flex justify-end p-1">
          <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" on:click={() => ((showLang = false), (langQuery = ""))}>Cancel</button>
        </div>
      </div>
    {/if}
    <svelte:fragment slot="after">
      <p class="note">
        Sets the system language (LANG) for every app. A language marked “generate” is built first,
        which takes a moment and asks for your password. Open apps keep their language until you sign
        out and back in.
      </p>
    </svelte:fragment>
  </Group>
</Page>
