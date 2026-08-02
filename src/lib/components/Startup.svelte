<script>
  import { onMount } from "svelte";
  import * as api from "../api.js";
  import { errorMsg, flashApplied } from "../stores.js";
  import { cleanExec } from "../hypr.js";
  import Card from "./ui/Card.svelte";
  import Toggle from "./ui/Toggle.svelte";
  import IconBtn from "./ui/IconBtn.svelte";

  let apps = []; // [{name, exec, icon, enabled}]
  let allApps = [];
  let query = "";
  let showAdd = false;

  onMount(async () => {
    try {
      const j = JSON.parse((await api.readConfig("quickshell/startup-apps.json")) || "{}");
      if (j && Array.isArray(j.apps)) apps = j.apps;
    } catch {}
    try {
      allApps = await api.desktopApps();
    } catch (e) {
      errorMsg.set(String(e));
    }
  });

  async function save() {
    try {
      await api.writeConfig("quickshell/startup-apps.json", JSON.stringify({ apps }, null, 2));
      flashApplied("Saved — applies at next login");
    } catch (e) {
      errorMsg.set(String(e));
    }
  }

  function add(a) {
    const exec = cleanExec(a.exec);
    if (apps.some((x) => x.exec === exec)) return;
    apps = [...apps, { name: a.name, exec, icon: a.icon || "", enabled: true }];
    showAdd = false;
    query = "";
    save();
  }
  function remove(i) {
    apps = apps.filter((_, j) => j !== i);
    save();
  }
  function toggle(i) {
    apps = apps.map((a, j) => (j === i ? { ...a, enabled: a.enabled === false } : a));
    save();
  }

  $: candidates = allApps.filter(
    (a) =>
      !apps.some((x) => x.exec === cleanExec(a.exec)) &&
      (query.trim() === "" || a.name.toLowerCase().includes(query.trim().toLowerCase()))
  );
</script>

<div class="mx-auto max-w-3xl space-y-6 p-5 sm:p-8">
  <div class="flex flex-wrap items-center justify-between gap-2">
    <h1 class="text-lg font-semibold">Startup applications</h1>
    <button class="btn-primary !py-1.5 text-xs" on:click={() => (showAdd = !showAdd)}>
      {showAdd ? "Cancel" : "+ Add application"}
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

  {#if apps.length === 0}
    <div class="card p-6 text-center text-sm text-zinc-400">
      Nothing starts automatically. Add an application to launch it at every login.
    </div>
  {:else}
    <Card>
      {#each apps as a, i (a.exec)}
        <div class="flex items-center gap-3 px-4 py-2.5 {a.enabled === false ? 'opacity-50' : ''}">
          <div class="min-w-0 flex-1">
            <div class="truncate text-sm font-medium">{a.name}</div>
            <div class="truncate text-xs text-zinc-400">{a.exec}</div>
          </div>
          <Toggle on={a.enabled !== false} toggled={() => toggle(i)} />
          <IconBtn icon={'<path d="M18 6 6 18"/><path d="m6 6 12 12"/>'} title="Remove" danger go={() => remove(i)} />
        </div>
      {/each}
    </Card>
  {/if}

  <p class="text-xs text-zinc-400 dark:text-zinc-500">
    Enabled entries are launched by the shell's autostart at login. Changes apply at the next login.
  </p>
</div>
