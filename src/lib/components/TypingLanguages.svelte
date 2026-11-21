<script>
  // Keyboard and mouse → Typing in other languages: Chinese, Japanese,
  // Korean, Vietnamese, Indic and more through fcitx5, installed only when
  // someone adds a language. ewe-ime owns the rest ([desktop.ime] in
  // ewe.conf, fcitx5's setup, the session); this asks it, and installs
  // packages through the one install_packages command (one password prompt).
  import { onMount } from "svelte";
  import * as api from "../api.js";
  import { errorMsg, flashApplied, toast } from "../stores.js";
  import Group from "./ui/Group.svelte";
  import Row from "./ui/Row.svelte";
  import ToggleRow from "./ui/ToggleRow.svelte";
  import IconBtn from "./ui/IconBtn.svelte";
  import Icon from "./ui/Icon.svelte";
  import Alert from "./ui/Alert.svelte";
  import Sheet from "./ui/Sheet.svelte";
  import PickList from "./ui/PickList.svelte";

  let st = null; // ewe-ime status
  let unavailable = ""; // an ewe without ewe-ime
  let busy = ""; // the language being installed / changed ("*" = all)
  let showAdd = false;
  let addQuery = "";

  async function refresh() {
    try {
      st = await api.imeStatus();
      unavailable = "";
    } catch (e) {
      unavailable = String(e);
    }
  }
  onMount(refresh);

  $: engines = st?.engines || [];
  $: missingAny = engines.some((e) => !e.installed) || (engines.length > 0 && st && !st.installed);
  $: candidates = (st?.catalog || []).filter((c) => {
    if (engines.some((e) => e.id === c.id)) return false;
    const q = addQuery.trim().toLowerCase();
    return (
      q === "" ||
      c.language.toLowerCase().includes(q) ||
      c.method.toLowerCase().includes(q) ||
      (c.native || "").toLowerCase().includes(q)
    );
  });

  async function add(c) {
    showAdd = false;
    addQuery = "";
    busy = c.id;
    try {
      if (c.packages?.length) {
        toast(`Installing **${c.language}**… ewe asks for your password once.`);
        await api.installPackages(c.packages);
      }
      st = await api.ime(["add", c.id]);
      flashApplied(`${c.language} added. Ctrl+Space switches to it.`);
    } catch (e) {
      errorMsg.set(String(e));
    } finally {
      busy = "";
      refresh();
    }
  }

  async function installMissing() {
    busy = "*";
    try {
      toast("Installing… ewe asks for your password once.");
      await api.installPackages(st.missing || []);
      st = await api.ime(["apply"]).then(() => api.imeStatus());
      flashApplied("Installed");
    } catch (e) {
      errorMsg.set(String(e));
    } finally {
      busy = "";
      refresh();
    }
  }

  async function remove(e) {
    busy = e.id;
    try {
      const r = await api.ime(["remove", e.id]);
      st = r;
      if (r.note) toast(r.note);
      else flashApplied(`${e.language} removed`);
    } catch (err) {
      errorMsg.set(String(err));
    } finally {
      busy = "";
    }
  }

  async function setOn(on) {
    busy = "*";
    try {
      st = await api.ime([on ? "enable" : "disable"]);
    } catch (e) {
      errorMsg.set(String(e));
    } finally {
      busy = "";
    }
  }

  const sub = (e) =>
    !e.installed
      ? "Not installed on this computer"
      : busy === e.id
        ? "Working…"
        : e.method;
</script>

{#if !unavailable}
  <Group
    title="Typing in other languages"
    desc="Chinese, Japanese, Korean, Vietnamese, Hindi and more."
  >
    <svelte:fragment slot="action">
      <button
        class="ewe-btn ewe-btn--secondary ewe-btn--sm"
        disabled={!st || busy !== ""}
        on:click={() => ((addQuery = ""), (showAdd = true))}
      >
        <Icon name="plus" />Add a language…
      </button>
    </svelte:fragment>
    {#if !st}
      <Row sub="Reading…" />
    {:else if engines.length === 0}
      <Row
        title="No languages added"
        sub="Nothing is installed until you add one."
        setting="Typing in other languages"
      />
    {:else}
      <ToggleRow
        title="Type in these languages"
        sub="Ctrl+Space switches to them. Super+Space still changes the layout."
        on={st.enabled}
        dim={busy !== ""}
        toggled={(v) => setOn(v)}
      />
      {#each engines as e (e.id)}
        <div class="ewe-row" class:is-dim={!st.enabled}>
          <span class="ewe-badge layout-code ime-mark"><span class="ewe-badge__label">{e.glyph}</span></span>
          <div class="ewe-row__text">
            <div class="ewe-row__title">{e.language}{#if e.native}<span class="ime-native">{e.native}</span>{/if}</div>
            <div class="ewe-row__desc">{sub(e)}</div>
          </div>
          <div class="ewe-row__trail">
            {#if busy === e.id}
              <span class="ewe-spinner ewe-spinner--sm" role="status" aria-label="Working…"></span>
            {/if}
            <IconBtn name="x" title="Remove {e.language}" danger disabled={busy !== ""} go={() => remove(e)} />
          </div>
        </div>
      {/each}
      {#if st.configtool && st.installed}
        <Row title="Language options" sub="Fuzzy Pinyin, dictionaries, how many suggestions show">
          <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" on:click={() => api.ime(["configure"]).catch((e) => errorMsg.set(String(e)))}>
            Open…
          </button>
        </Row>
      {/if}
    {/if}
    <svelte:fragment slot="after">
      {#if st && engines.length && missingAny}
        <Alert tone="warning" title="Not installed on this computer">
          {(st.missing || []).length
            ? "This computer got the list from your account. Install the missing parts to type them here."
            : "Some languages need their part installed."}
          <svelte:fragment slot="actions">
            <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" disabled={busy !== ""} on:click={installMissing}>
              {#if busy === "*"}<span class="ewe-spinner ewe-spinner--sm" aria-hidden="true"></span>{/if}Install
            </button>
          </svelte:fragment>
        </Alert>
      {:else if st && st.on && st.needs_signout}
        <Alert tone="info" title="Sign out once for older apps">
          Most apps can use it now. Apps that were already open pick it up when they restart, and older (X11)
          apps after you sign out and back in.
        </Alert>
      {/if}
    </svelte:fragment>
  </Group>
{/if}

<!-- Add a language: pick it in a Sheet -->
<Sheet open={showAdd} title="Add a language" onClose={() => ((showAdd = false), (addQuery = ""))}>
  <PickList
    bind:query={addQuery}
    placeholder="Search languages"
    items={candidates.map((c) => ({
      id: c.id,
      title: `${c.language} — ${c.method}`,
      trail: c.installed ? c.native : `${c.native} · Install`,
      c
    }))}
    pick={(it) => add(it.c)}
  />
  <p class="note">
    A language marked “Install” is downloaded first (one password prompt). Removing a language later leaves its
    programs installed; Komble can remove them.
  </p>
</Sheet>

<style>
  /* the engine's mark (拼 あ 한) reads in the text face, not the mono one */
  .ime-mark {
    font-family: var(--font-sans);
  }
  .ime-native {
    margin-left: var(--space-s);
    color: var(--text-muted);
    font-weight: var(--font-weight-regular);
  }
</style>
