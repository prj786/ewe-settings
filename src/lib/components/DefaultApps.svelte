<script>
  import { onMount } from "svelte";
  import * as api from "../api.js";
  import { APP_CATS } from "../hypr.js";
  import { errorMsg, flashApplied } from "../stores.js";
  import Card from "./ui/Card.svelte";
  import SelectRow from "./ui/SelectRow.svelte";

  let defaults = {}; // category key → desktop id
  let choices = {}; // category key → [desktop ids]

  const pretty = (id) => String(id || "").replace(/\.desktop$/, "") || "—";

  async function refresh() {
    for (const c of APP_CATS) {
      api.mimeDefault(c.mime).then((d) => (defaults = { ...defaults, [c.key]: d })).catch(() => {});
      api.mimeApps(c.mime).then((a) => (choices = { ...choices, [c.key]: a })).catch(() => {});
    }
  }
  onMount(refresh);

  async function set(cat, id) {
    const c = APP_CATS.find((x) => x.key === cat);
    if (!c) return;
    try {
      await api.setMimeDefault(id, c.mimes, cat === "Browser");
      flashApplied();
      setTimeout(refresh, 400);
    } catch (e) {
      errorMsg.set(String(e));
    }
  }
</script>

<div class="mx-auto max-w-3xl space-y-6 p-5 sm:p-8">
  <h1 class="text-lg font-semibold">Default applications</h1>

  <Card>
    {#each APP_CATS as c (c.key)}
      {@const opts = [
        ...new Set([...(choices[c.key] || []), ...(defaults[c.key] ? [defaults[c.key]] : [])])
      ].map((id) => ({ label: pretty(id), value: id }))}
      {#if opts.length}
        <SelectRow label={c.key} options={opts} value={defaults[c.key] || ""} picked={(v) => set(c.key, v)} />
      {:else}
        <div class="flex items-center justify-between px-4 py-3">
          <span class="text-sm font-medium">{c.key}</span>
          <span class="text-xs text-zinc-400">no handlers installed</span>
        </div>
      {/if}
    {/each}
  </Card>

  <p class="text-xs text-zinc-400 dark:text-zinc-500">
    Stored in ~/.config/mimeapps.list — read natively by every GTK app and by xdg-open.
  </p>
</div>
