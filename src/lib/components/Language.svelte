<script>
  import { onMount } from "svelte";
  import * as api from "../api.js";
  import { errorMsg, flashApplied } from "../stores.js";
  import Page from "./ui/Page.svelte";
  import Group from "./ui/Group.svelte";
  import Row from "./ui/Row.svelte";
  import ToggleRow from "./ui/ToggleRow.svelte";
  import PickList from "./ui/PickList.svelte";
  import Sheet from "./ui/Sheet.svelte";
  import Icon from "./ui/Icon.svelte";
  import IconBtn from "./ui/IconBtn.svelte";

  // ── Display language ──────────────────────────────────────────────────────
  // The list is every UTF-8 locale glibc knows, not just the generated ones —
  // picking an ungenerated one runs locale-gen behind one polkit prompt, so
  // the user never has to know locale.gen exists. The regional formats
  // (Time and place → Formats) are kept as they are.
  let loc = null;
  let langQuery = "";
  let showLang = false;
  let langBusy = false;
  async function refreshLocale() {
    try {
      loc = await api.localeInfo();
      refreshPacks();
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
      refreshSpell();
    } catch (e) {
      errorMsg.set(String(e));
    } finally {
      langBusy = false;
    }
  }

  // ── Also for your apps ────────────────────────────────────────────────────
  // Firefox, Thunderbird and LibreOffice speak a language only with its pack;
  // offered for the ones that are installed, plus the language's spelling
  // dictionary when the official repositories have it.
  let packs = null;
  let packsBusy = false;
  async function refreshPacks() {
    if (!loc?.current) return;
    try {
      packs = await api.languagePacks(loc.current);
    } catch (e) {
      packs = null;
    }
  }
  $: missingPacks = (packs?.packs || []).filter((p) => !p.installed);

  async function install(names, what) {
    flashApplied(`Installing ${what}…`);
    const res = await api.installPackages(names);
    const n = (res?.installed || []).length;
    flashApplied(n ? `${what} installed` : `${what} was already installed`);
    return res;
  }

  async function installPacks(list) {
    if (packsBusy || !list.length) return;
    packsBusy = true;
    try {
      await install(
        list.map((p) => p.package),
        list.length === 1 ? `${list[0].app} in ${packs.label}` : `Language packs for ${packs.label}`
      );
      await refreshPacks();
      refreshSpell();
    } catch (e) {
      errorMsg.set(String(e));
    } finally {
      packsBusy = false;
    }
  }

  // ── Spell checking ────────────────────────────────────────────────────────
  // The languages live in ewe.conf ([desktop.language] spellcheck, synced);
  // unset = automatic (the system language's dictionary + US English). ewe
  // Browser follows the list as it changes. A dictionary is a system package
  // (Firefox, Thunderbird, LibreOffice and other apps read /usr/share/hunspell);
  // ewe Browser brings its own, for the languages Chromium has.
  let spell = null;
  let spellBusy = false;
  let showAdd = false;
  let addQuery = "";
  async function refreshSpell() {
    try {
      spell = await api.spellInfo();
    } catch (e) {
      errorMsg.set(String(e));
    }
  }
  // Chromium's spelling languages (Electron 44's availableSpellCheckerLanguages):
  // ewe Browser checks a chosen language only when it is one of these
  const BROWSER = new Set(
    "af bg ca cs cy da de el en en-AU en-CA en-GB en-US es es-419 es-AR es-ES es-MX es-US et fa fo fr he hi hr hu hy id it ko lt lv nb nl pl pt pt-BR pt-PT ro ru sh sk sl sq sr sv ta tg tr uk vi".split(
      " "
    )
  );
  const inBrowser = (code) => {
    const tag = code.split("-")[0].replace("_", "-");
    return BROWSER.has(tag) || BROWSER.has(tag.split("-")[0]);
  };
  $: chosen = spell?.chosen || [];
  $: addItems = (spell?.available || [])
    .filter((d) => {
      const q = addQuery.trim().toLowerCase();
      return q === "" || d.label.toLowerCase().includes(q) || d.code.toLowerCase().includes(q);
    })
    .slice(0, 80)
    .map((d) => ({
      id: d.code,
      title: d.label,
      trail: d.state === "installed" ? "Ready" : d.state === "repo" ? "Download" : "In Komble",
      d
    }));

  async function saveChosen(list, msg) {
    try {
      await api.setConf("desktop.language.spellcheck", list);
      if (msg) flashApplied(msg);
      await refreshSpell();
    } catch (e) {
      errorMsg.set(String(e));
    }
  }
  async function toggleSpell(on) {
    try {
      await api.setConf("desktop.language.check_spelling", on);
      flashApplied(on ? "Spell checking on" : "Spell checking off");
      await refreshSpell();
    } catch (e) {
      errorMsg.set(String(e));
    }
  }
  function remove(code) {
    const list = chosen.map((c) => c.code).filter((c) => c !== code);
    if (list.length) saveChosen(list);
  }
  async function getDictionary(d) {
    if (spellBusy || !d.package) return;
    if (d.source === "aur" || d.state === "aur") {
      try {
        const ok = await api.kombleSearch(d.package);
        if (!ok) errorMsg.set("Komble is not installed.");
        else flashApplied(`Komble is looking for ${d.label}`);
      } catch (e) {
        errorMsg.set(String(e));
      }
      return;
    }
    spellBusy = true;
    try {
      await install([d.package], `${d.label} spelling`);
      await refreshSpell();
    } catch (e) {
      errorMsg.set(String(e));
    } finally {
      spellBusy = false;
    }
  }
  async function addLanguage(d) {
    if (spellBusy) return;
    showAdd = false;
    addQuery = "";
    const list = [...chosen.map((c) => c.code).filter((c) => c !== d.code), d.code];
    if (d.state === "repo") {
      spellBusy = true;
      try {
        await install([d.package], `${d.label} spelling`);
      } catch (e) {
        errorMsg.set(String(e));
        spellBusy = false;
        return;
      }
      spellBusy = false;
    }
    await saveChosen(list, `${d.label} added`);
    if (d.state === "aur") getDictionary(d);
  }

  function rowSub(d) {
    const parts = [];
    if (d.state === "missing") {
      if (d.source === "repo") parts.push("Dictionary not on this computer");
      else if (d.source === "aur") parts.push("Its dictionary is in Komble");
      else parts.push("No dictionary for it yet");
    }
    if (!inBrowser(d.code)) parts.push("Not in ewe Browser yet");
    return parts.join(" · ");
  }

  onMount(() => {
    refreshLocale();
    refreshSpell();
  });
</script>

<Page title="Language" desc="The language of menus and dates, your apps' language packs, and spelling.">
  <Group title="Display language" desc="Apps that are already open switch after you sign out and back in.">
    <Row
      title="Display language"
      sub={loc && loc.current && !loc.generated ? "Needs setting up. ewe does it when you pick it." : "Every app's menus and dates"}
    >
      <span class="kv-value">{loc?.currentLabel || (loc ? "Not set" : "…")}</span>
      <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" on:click={() => ((langQuery = ""), (showLang = true))}>Change…</button>
    </Row>
  </Group>

  {#if packs && packs.packs.length}
    <Group title="Also for your apps" desc="Language packs so these apps speak {packs.label} too.">
      <svelte:fragment slot="action">
        {#if missingPacks.length > 1}
          <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" disabled={packsBusy} on:click={() => installPacks(missingPacks)}>
            <Icon name="download" />Install all
          </button>
        {/if}
      </svelte:fragment>
      {#each packs.packs as p (p.package)}
        <Row title={p.app} sub={p.package}>
          {#if p.installed}
            <span class="ewe-badge ewe-badge--success"><span class="ewe-badge__label">Installed</span></span>
          {:else}
            <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" disabled={packsBusy} on:click={() => installPacks([p])}>Install</button>
          {/if}
        </Row>
      {/each}
    </Group>
  {/if}

  <Group
    title="Spell checking"
    desc={spell?.auto ? "Chosen from the display language until you add or remove one." : "Words you type are checked in these languages."}
  >
    <svelte:fragment slot="action">
      <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" disabled={!spell || spellBusy} on:click={() => ((addQuery = ""), (showAdd = true))}>
        <Icon name="plus" />Add a language…
      </button>
    </svelte:fragment>
    <ToggleRow
      title="Check spelling as you type"
      sub="In ewe Browser, and in apps that use the system dictionaries"
      on={spell?.enabled ?? true}
      dim={!spell || !spell.confAvailable}
      toggled={toggleSpell}
    />
    {#each chosen as d (d.code)}
      <Row title={d.label} sub={rowSub(d)} dim={spell && !spell.enabled} setting={d === chosen[0] ? "Spelling languages" : ""}>
        {#if d.state === "missing" && d.source}
          <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" disabled={spellBusy} on:click={() => getDictionary(d)}>
            {d.source === "aur" ? "Open Komble" : "Download"}
          </button>
        {/if}
        <IconBtn name="x" title="Remove {d.label}" danger disabled={chosen.length <= 1} go={() => remove(d.code)} />
      </Row>
    {/each}
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

<!-- Add a spelling language -->
<Sheet open={showAdd} title="Add a spelling language" onClose={() => ((showAdd = false), (addQuery = ""))}>
  <PickList bind:query={addQuery} placeholder="Search languages" items={addItems} pick={(it) => addLanguage(it.d)} />
  <p class="note">“Download” installs the dictionary and asks for your password. “In Komble” opens Komble, which builds it for you.</p>
</Sheet>
