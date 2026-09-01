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

  <section>
    <div class="section-title">Language</div>
    <Card>
      <div class="flex flex-wrap items-center justify-between gap-x-3 gap-y-1.5 px-4 py-3">
        <div class="min-w-0">
          <div class="text-sm font-medium">Display language</div>
          <div class="text-xs text-zinc-400 dark:text-zinc-500">
            {#if loc}
              <span class="font-mono">{loc.current || "unset"}</span>
              {#if loc.current && !loc.generated}&nbsp;·&nbsp;not generated{/if}
            {:else}
              …
            {/if}
          </div>
        </div>
        <div class="flex items-center gap-2">
          <span class="truncate text-sm">{loc?.currentLabel || "—"}</span>
          {#if !showLang}
            <button class="btn-ghost !py-1 text-xs" on:click={() => (showLang = true)}>Change</button>
          {/if}
        </div>
      </div>
      {#if showLang}
        <div class="px-4 py-3 {langBusy ? 'pointer-events-none opacity-50' : ''}">
          <!-- svelte-ignore a11y_autofocus -->
          <input class="input mb-2" placeholder="Search languages…" bind:value={langQuery} autofocus />
          <div class="max-h-64 overflow-y-auto rounded-lg border border-zinc-200 dark:border-zinc-700/60">
            {#each langCandidates.slice(0, 40) as c (c.code)}
              <button
                class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-sm hover:bg-black/5 dark:hover:bg-white/5 {c.code === loc?.current ? 'font-medium' : ''}"
                on:click={() => setLocale(c)}
              >
                <span class="min-w-0 flex-1 truncate">{c.label}</span>
                <span class="shrink-0 font-mono text-[11px] text-zinc-400 dark:text-zinc-500">{c.code}</span>
                {#if c.generated}
                  <span class="w-16 shrink-0 rounded bg-zinc-200/70 px-1.5 py-0.5 text-center text-[10px] uppercase dark:bg-zinc-700/60">ready</span>
                {:else}
                  <span class="w-16 shrink-0 text-center text-[10px] uppercase text-zinc-400 dark:text-zinc-500">generate</span>
                {/if}
              </button>
            {:else}
              <div class="px-3 py-2 text-xs text-zinc-400 dark:text-zinc-500">No match.</div>
            {/each}
          </div>
          <button class="btn-ghost mt-2 !py-1 text-xs" on:click={() => ((showLang = false), (langQuery = ""))}>
            Cancel
          </button>
        </div>
      {/if}
    </Card>
    <p class="mt-2 text-xs text-zinc-400 dark:text-zinc-500">
      Sets the system language (LANG) for every app. Locales marked "generate" are built first,
      which takes a moment and asks for your password. Running apps keep their current language —
      sign out and back in to see the change everywhere.
    </p>
  </section>
</div>
