<script>
  import { onMount } from "svelte";
  import * as api from "../api.js";
  import { errorMsg, flashApplied } from "../stores.js";
  import { windowRulesLuaText } from "../hypr.js";
  import IconBtn from "./ui/IconBtn.svelte";
  import * as Select from "./ui/select/index.js";

  // Per-app open rules: workspace and/or float/tile. Source of truth is
  // quickshell/window-rules.json; every save also regenerates
  // hypr/generated/windowrules.lua and does a `hyprctl reload` (rules are
  // named, so reloads never stack duplicates). Rules apply when a window
  // OPENS — already-open windows are not moved.
  let rules = []; // [{name, class, workspace: 0|1..8, mode: ""|"float"|"tile"}]
  let allApps = [];
  let query = "";
  let showAdd = false;

  const WS = [
    { label: "Any", value: 0 },
    ...Array.from({ length: 8 }, (_, i) => ({ label: `Workspace ${i + 1}`, value: i + 1 }))
  ];
  const MODES = [
    { label: "As usual", value: "" },
    { label: "Floating", value: "float" },
    { label: "Tiled", value: "tile" }
  ];
  // bits-ui reads "" as "nothing selected" — same sentinel trick as SelectRow.
  const EMPTY = " empty";
  const enc = (v) => (String(v ?? "") === "" ? EMPTY : String(v));

  onMount(async () => {
    try {
      const j = JSON.parse((await api.readConfig("quickshell/window-rules.json")) || "{}");
      if (j && Array.isArray(j.rules)) rules = j.rules;
    } catch {}
    try {
      allApps = await api.desktopApps();
    } catch (e) {
      errorMsg.set(String(e));
    }
  });

  async function save() {
    try {
      // RFC-001: routes through ewe-conf, which regenerates windowrules.lua
      await api.writeConfig("quickshell/window-rules.json", JSON.stringify({ rules }, null, 2));
      await api.reloadHyprland();
      flashApplied("Saved. Applies when the app next opens.");
    } catch (e) {
      errorMsg.set(String(e));
    }
  }

  function add(a) {
    // Wayland app-ids usually equal the desktop-file id; X11/Electron apps
    // advertise theirs via StartupWMClass. Editable per-row for the outliers.
    const klass = (a.wmClass || a.id || "").trim();
    if (!klass || rules.some((r) => r.class === klass)) {
      showAdd = false;
      query = "";
      return;
    }
    rules = [...rules, { name: a.name, class: klass, workspace: 0, mode: "" }];
    showAdd = false;
    query = "";
    save();
  }
  function remove(i) {
    rules = rules.filter((_, j) => j !== i);
    save();
  }
  function patch(i, p) {
    rules = rules.map((r, j) => (j === i ? { ...r, ...p } : r));
    save();
  }

  $: candidates = allApps.filter(
    (a) =>
      !rules.some((r) => r.class === ((a.wmClass || a.id || "").trim())) &&
      (query.trim() === "" || a.name.toLowerCase().includes(query.trim().toLowerCase()))
  );
  import Page from "./ui/Page.svelte";
  import Group from "./ui/Group.svelte";
  import Row from "./ui/Row.svelte";
  import Icon from "./ui/Icon.svelte";
  import PickList from "./ui/PickList.svelte";
  import Sheet from "./ui/Sheet.svelte";
</script>

<Page title="Window rules" desc="Always open an app on a chosen workspace, or make it float or tile.">
  <svelte:fragment slot="actions">
    <button class="ewe-btn ewe-btn--secondary" data-setting="Add rule" on:click={() => ((query = ""), (showAdd = true))}>
      <Icon name="plus" />Add rule…
    </button>
  </svelte:fragment>

  {#if rules.length === 0}
    <Group title="Rules">
      <div class="ewe-empty">
        <span class="ewe-empty__icon"><Icon name="windowRules" /></span>
        <div class="ewe-empty__title">No rules yet</div>
        <div class="ewe-empty__desc">Add one to always open an app on a chosen workspace, or make it float or tile.</div>
        <div class="ewe-empty__actions">
          <button class="ewe-btn ewe-btn--secondary" on:click={() => ((query = ""), (showAdd = true))}><Icon name="plus" />Add rule…</button>
        </div>
      </div>
    </Group>
  {:else}
    <Group title="Rules" desc="They apply when the app's window opens. If one isn't caught, edit the window class under its name.">
      {#each rules as r, i (r.class)}
        <Row title={r.name || r.class}>
          <svelte:fragment slot="text">
            <input
              class="rule-class"
              title="The window class this rule matches. Change it if the app's window isn't caught."
              aria-label="Window class for {r.name || r.class}"
              value={r.class}
              on:change={(e) => patch(i, { class: e.target.value.trim() || r.class })}
            />
          </svelte:fragment>
          <Select.Root
            type="single"
            value={enc(r.workspace || 0)}
            onValueChange={(raw) => patch(i, { workspace: Number(raw) || 0 })}
          >
            <Select.Trigger class="select-trigger" aria-label="Workspace for {r.name || r.class}">
              {(WS.find((o) => enc(o.value) === enc(r.workspace || 0)) || WS[0]).label}
            </Select.Trigger>
            <Select.Content>
              {#each WS as o (o.value)}
                <Select.Item value={enc(o.value)} label={o.label} />
              {/each}
            </Select.Content>
          </Select.Root>
          <Select.Root
            type="single"
            value={enc(r.mode || "")}
            onValueChange={(raw) => patch(i, { mode: raw === EMPTY ? "" : raw })}
          >
            <Select.Trigger class="select-trigger" aria-label="Floating or tiled, for {r.name || r.class}">
              {(MODES.find((o) => enc(o.value) === enc(r.mode || "")) || MODES[0]).label}
            </Select.Trigger>
            <Select.Content>
              {#each MODES as o (enc(o.value))}
                <Select.Item value={enc(o.value)} label={o.label} />
              {/each}
            </Select.Content>
          </Select.Root>
          <IconBtn name="x" title="Remove the rule for {r.name || r.class}" danger go={() => remove(i)} />
        </Row>
      {/each}
    </Group>
  {/if}
</Page>

<!-- Add a rule: pick the app in a Sheet; the page behind stays as it is -->
<Sheet open={showAdd} title="Add a rule for…" onClose={() => (showAdd = false)}>
  <PickList
    bind:query
    placeholder="Search apps"
    empty="Every app already has a rule."
    items={candidates.slice(0, 60).map((a) => ({ id: a.id, title: a.name, trail: a.comment, a }))}
    pick={(it) => add(it.a)}
  />
</Sheet>
