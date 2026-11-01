<script>
  import { onMount, onDestroy } from "svelte";
  import * as api from "../api.js";
  import { errorMsg, flashApplied } from "../stores.js";
  import ToggleRow from "./ui/ToggleRow.svelte";
  import SelectRow from "./ui/SelectRow.svelte";

  let info = null;
  let zones = [];
  let now = new Date();
  const tick = setInterval(() => (now = new Date()), 5000);
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
      flashApplied(`Time zone set to ${cityName(z)}`);
      setTimeout(refresh, 400);
    } catch (e) {
      errorMsg.set(String(e));
    }
  }

  async function toggleAuto() {
    const on = !(info?.auto ?? true);
    try {
      await api.setAutoTimezone(on);
      flashApplied(on ? "Automatic time zone on. Finding where you are…" : "Time zone kept as it is");
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
    if (!c.generated) flashApplied(`Preparing ${c.label}…`);
    try {
      await api.setLocale(c.code);
      flashApplied(`Language set to ${c.label}`);
      showLang = false;
      langQuery = "";
      await refreshLocale();
    } catch (e) {
      errorMsg.set(String(e));
    } finally {
      langBusy = false;
    }
  }

  const timeText = (d) => d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  const dateText = (d) => d.toLocaleDateString([], { weekday: "long", month: "long", day: "numeric" });
  $: auto = info?.auto ?? true;
  $: zoneText = info?.timezone ? `${cityName(info.timezone)} (${info.timezone.split("/")[0]})` : "—";
  import Page from "./ui/Page.svelte";
  import Group from "./ui/Group.svelte";
  import Row from "./ui/Row.svelte";
  import PickList from "./ui/PickList.svelte";
  import Sheet from "./ui/Sheet.svelte";
  import KV from "./ui/KV.svelte";
</script>

<Page title="Time and place" desc="The time zone, the clock and the language apps use.">
  <Group title="Clock">
    <KV k="Date and time" v="{dateText(now)}, {timeText(now)}" />
    <ToggleRow
      title="Network time"
      sub={info?.ntp ?? true ? (info?.ntpSynced ? "Keeps the clock right. In sync." : "Keeps the clock right. Syncing…") : "Keeps the clock right over the internet"}
      on={info?.ntp ?? true}
      toggled={toggleNtp}
    />
  </Group>

  <Group title="Time zone" desc="A zone you pick yourself stays until you turn automatic back on.">
    <ToggleRow
      title="Set the time zone automatically"
      sub="From where your network is, each time you connect. A VPN pauses it."
      on={auto}
      toggled={toggleAuto}
    />
    {#if auto}
      <KV k="Time zone" v={zoneText} />
    {:else}
      <SelectRow
        label="Region"
        options={regions.map((r) => ({ label: r, value: r }))}
        value={curRegion}
        picked={(r) => (pickedRegion = r)}
      />
      <SelectRow
        label="Zone"
        options={regionZones.map((z) => ({ label: cityName(z), value: z }))}
        value={info?.timezone || ""}
        picked={setZone}
      />
    {/if}
  </Group>

  <Group title="Language" desc="Apps that are already open switch after you sign out and back in.">
    <Row
      title="Display language"
      sub={loc && loc.current && !loc.generated ? "Needs setting up. ewe does it when you pick it." : "Every app's menus and dates"}
    >
      <span class="kv-value">{loc?.currentLabel || (loc ? "Not set" : "…")}</span>
      <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" on:click={() => ((langQuery = ""), (showLang = true))}>Change…</button>
    </Row>
  </Group>
</Page>

<!-- Pick a language in a Sheet -->
<Sheet open={showLang} title="Display language" onClose={() => ((showLang = false), (langQuery = ""))}>
  <div class={langBusy ? "pointer-events-none" : ""} aria-busy={langBusy}>
    <PickList
      bind:query={langQuery}
      placeholder="Search languages"
      items={langCandidates.slice(0, 60).map((c) => ({
        id: c.code,
        title: c.label,
        trail: c.generated ? "" : "Needs setup",
        selected: c.code === loc?.current,
        c
      }))}
      pick={(it) => setLocale(it.c)}
    />
  </div>
  <p class="note">A language marked “Needs setup” is prepared first. That takes a moment and asks for your password.</p>
</Sheet>
