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
    refreshFormats();
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

  // ── Formats ───────────────────────────────────────────────────────────────
  // Dates, times, numbers, money, measurement and paper from one region
  // (LC_TIME, LC_NUMERIC, LC_MONETARY, LC_MEASUREMENT, LC_PAPER), separate
  // from the display language (Language page). Set with localectl behind one
  // polkit prompt; the locale is generated first when it has to be. Apps read
  // them when they start, so the change shows after signing out.
  let fmt = null;
  let fmtRegions = null;
  let fmtQuery = "";
  let showFmt = false;
  let fmtBusy = false;
  async function refreshFormats() {
    try {
      fmt = await api.formatsInfo();
    } catch (e) {
      errorMsg.set(String(e));
    }
  }
  async function loadRegions() {
    if (fmtRegions) return;
    try {
      fmtRegions = (await api.localeInfo()).choices || [];
    } catch (e) {
      errorMsg.set(String(e));
    }
  }
  // "sr_RS.UTF-8@latin" → "sr-Latn-RS": what Intl understands
  function bcp47(code) {
    const [base, mod] = String(code || "").split("@");
    const [lang, region] = base.split(".")[0].split("_");
    const script = { latin: "Latn", cyrillic: "Cyrl", devanagari: "Deva" }[mod];
    return [lang, script, region].filter(Boolean).join("-");
  }
  // what apps will show, written by the region itself
  function example(code, d, short = false) {
    try {
      const tag = bcp47(code) || undefined;
      const time = new Intl.DateTimeFormat(tag, { hour: "numeric", minute: "2-digit" }).format(d);
      if (short) return `${new Intl.DateTimeFormat(tag, { dateStyle: "short" }).format(d)} · ${time}`;
      const date = new Intl.DateTimeFormat(tag, { weekday: "long", day: "numeric", month: "long", year: "numeric" }).format(d);
      const num = new Intl.NumberFormat(tag, { maximumFractionDigits: 1 }).format(1234.5);
      return `${date} · ${time} · ${num}`;
    } catch (e) {
      return "";
    }
  }
  function weekStart(code) {
    try {
      const l = new Intl.Locale(bcp47(code));
      const wi = typeof l.getWeekInfo === "function" ? l.getWeekInfo() : l.weekInfo;
      const day = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"][(wi?.firstDay || 0) - 1];
      return day ? `Weeks start on ${day}` : "";
    } catch (e) {
      return "";
    }
  }
  $: fmtCode = fmt ? fmt.formats || fmt.lang : "";
  $: fmtCandidates = [
    { id: "", title: "Same as the display language", trail: fmt?.lang ? example(fmt.lang, now, true) : "", selected: fmt && !fmt.formats, c: null },
    ...(fmtRegions || [])
      .filter((c) => {
        const q = fmtQuery.trim().toLowerCase();
        return q === "" || c.label.toLowerCase().includes(q) || c.code.toLowerCase().includes(q);
      })
      .slice(0, 60)
      .map((c) => ({ id: c.code, title: c.label, trail: example(c.code, now, true), selected: fmt?.formats === c.code, c }))
  ].filter((it, i) => i > 0 || fmtQuery.trim() === "");

  async function setFormats(it) {
    if (fmtBusy) return;
    const code = it.c ? it.c.code : "";
    if (fmt && code === (fmt.formats || "") && !fmt.mixed) {
      showFmt = false;
      return;
    }
    fmtBusy = true;
    if (it.c && !it.c.generated) flashApplied(`Preparing ${it.c.label}…`);
    try {
      await api.setFormats(code);
      flashApplied(code ? `Formats of ${it.c.label}. Applies after you sign out.` : "Formats follow the language. Applies after you sign out.");
      showFmt = false;
      fmtQuery = "";
      await refreshFormats();
    } catch (e) {
      errorMsg.set(String(e));
    } finally {
      fmtBusy = false;
    }
  }

  // written the way the region's formats write them (Formats below)
  const fmtTag = (f) => (f ? bcp47(f.formats || f.lang) || [] : []);
  const timeText = (d, f) => d.toLocaleTimeString(fmtTag(f), { hour: "2-digit", minute: "2-digit" });
  const dateText = (d, f) => d.toLocaleDateString(fmtTag(f), { weekday: "long", month: "long", day: "numeric" });
  $: auto = info?.auto ?? true;
  $: zoneText = info?.timezone ? `${cityName(info.timezone)} (${info.timezone.split("/")[0]})` : "—";
  import Page from "./ui/Page.svelte";
  import Group from "./ui/Group.svelte";
  import Row from "./ui/Row.svelte";
  import PickList from "./ui/PickList.svelte";
  import Sheet from "./ui/Sheet.svelte";
  import KV from "./ui/KV.svelte";
</script>

<Page title="Time and place" desc="The clock, the time zone and how dates and numbers are written.">
  <Group title="Clock">
    <KV k="Date and time" v="{dateText(now, fmt)}, {timeText(now, fmt)}" />
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

  <Group title="Formats" desc="Dates, times, numbers and measurements. Applies after you sign out.">
    <Row title="Region" sub={fmt?.mixed ? "Set by hand. Pick one to put them in step." : fmt?.formats ? "" : "Same as the display language"}>
      <span class="kv-value">{fmt ? fmt.formatsLabel || fmt.langLabel || "Not set" : "…"}</span>
      <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" on:click={() => ((fmtQuery = ""), (showFmt = true), loadRegions())}>Change…</button>
    </Row>
    {#if fmtCode}
      <KV k="Example" v={example(fmtCode, now)} />
      {#if weekStart(fmtCode)}<KV k="Calendar" v={weekStart(fmtCode)} />{/if}
    {/if}
  </Group>
</Page>

<!-- Pick the formats' region in a Sheet -->
<Sheet open={showFmt} title="Formats" onClose={() => ((showFmt = false), (fmtQuery = ""))}>
  <div class={fmtBusy ? "pointer-events-none" : ""} aria-busy={fmtBusy}>
    <PickList bind:query={fmtQuery} placeholder="Search regions" items={fmtCandidates} pick={setFormats} />
  </div>
  <p class="note">Regions are written in their own way, as apps will show them. One that is not set up yet is prepared first and asks for your password.</p>
</Sheet>
