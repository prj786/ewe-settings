<script>
  import { onMount } from "svelte";
  import * as api from "../api.js";
  import { errorMsg, flashApplied } from "../stores.js";
  import { windowRulesLuaText } from "../hypr.js";
  import Card from "./ui/Card.svelte";
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
    ...Array.from({ length: 8 }, (_, i) => ({ label: `Desktop ${i + 1}`, value: i + 1 }))
  ];
  const MODES = [
    { label: "Default", value: "" },
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
      flashApplied("Applied — takes effect when the app opens");
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
</script>

<div class="mx-auto max-w-3xl space-y-6 p-5 sm:p-8">
  <div class="flex flex-wrap items-center justify-between gap-2">
    <h1 class="text-lg font-semibold">Window rules</h1>
    <button class="btn-primary !py-1.5 text-xs" on:click={() => (showAdd = !showAdd)}>
      {showAdd ? "Cancel" : "+ Add rule"}
    </button>
  </div>

  {#if showAdd}
    <Card>
      <div class="p-3">
        <input class="input" placeholder="Search applications…" bind:value={query} />
      </div>
      <div class="max-h-72 overflow-y-auto">
        {#each candidates.slice(0, 40) as a (a.id)}
          <button
            class="flex w-full items-center gap-3 px-4 py-2 text-left text-sm hover:bg-black/5 dark:hover:bg-white/5"
            on:click={() => add(a)}
          >
            <span class="min-w-0 flex-1 truncate">{a.name}</span>
            <span class="max-w-48 truncate text-xs text-zinc-400">{a.comment || a.exec}</span>
          </button>
        {/each}
      </div>
    </Card>
  {/if}

  {#if rules.length === 0}
    <div class="card p-6 text-center text-sm text-zinc-400">
      No rules yet. Add one to always open an app on a chosen desktop, or force it to float or tile.
    </div>
  {:else}
    <Card>
      {#each rules as r, i (r.class)}
        <div class="flex flex-wrap items-center gap-x-3 gap-y-2 px-4 py-2.5">
          <div class="min-w-0 flex-1 basis-40">
            <div class="truncate text-sm font-medium">{r.name || r.class}</div>
            <input
              class="mt-0.5 w-full max-w-48 border-0 bg-transparent p-0 font-mono text-xs text-zinc-400 outline-none focus:text-zinc-200 dark:text-zinc-500"
              title="Window class the rule matches (edit if the app's real class differs)"
              value={r.class}
              on:change={(e) => patch(i, { class: e.target.value.trim() || r.class })}
            />
          </div>
          <Select.Root
            type="single"
            value={enc(r.workspace || 0)}
            onValueChange={(raw) => patch(i, { workspace: Number(raw) || 0 })}
          >
            <Select.Trigger class="!w-auto min-w-28">
              <span data-slot="select-value" class="truncate">
                {(WS.find((o) => enc(o.value) === enc(r.workspace || 0)) || WS[0]).label}
              </span>
            </Select.Trigger>
            <Select.Content class="max-h-72 p-1">
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
            <Select.Trigger class="!w-auto min-w-28">
              <span data-slot="select-value" class="truncate">
                {(MODES.find((o) => enc(o.value) === enc(r.mode || "")) || MODES[0]).label}
              </span>
            </Select.Trigger>
            <Select.Content class="max-h-72 p-1">
              {#each MODES as o (enc(o.value))}
                <Select.Item value={enc(o.value)} label={o.label} />
              {/each}
            </Select.Content>
          </Select.Root>
          <IconBtn icon={'<path d="M18 6 6 18"/><path d="m6 6 12 12"/>'} title="Remove" danger go={() => remove(i)} />
        </div>
      {/each}
    </Card>
  {/if}

  <p class="text-xs text-zinc-400 dark:text-zinc-500">
    Rules apply when the app's window opens: "Desktop N" sends it to that workspace, "Floating"/"Tiled"
    overrides how it joins the layout. Matching is by window class — edit the small value under the app
    name if a window isn't caught (find the real class with <code>hyprctl activewindow</code>).
  </p>
</div>
