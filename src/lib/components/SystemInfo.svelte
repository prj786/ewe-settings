<script>
  import { onMount } from "svelte";
  import * as api from "../api.js";
  import { version } from "../stores.js";
  import Card from "./ui/Card.svelte";
  import KV from "./ui/KV.svelte";

  let d = null;
  onMount(async () => {
    try {
      d = await api.diagnostics();
    } catch {}
  });
</script>

<div class="mx-auto max-w-3xl space-y-6 p-5 sm:p-8">
  <h1 class="text-lg font-semibold">System</h1>

  <Card>
    <KV k="hypr-shell" v={$version || "unknown"} />
    {#if d}
      <KV k="Hyprland" v={d.hypr || "—"} />
      <KV k="Kernel" v={d.kernel || "—"} />
      <KV k="GPU driver" v={d.gpu || "—"} />
      <KV k="Memory" v={d.mem || "—"} />
      <KV k="Disk (/)" v={d.disk || "—"} />
    {:else}
      <div class="px-4 py-3 text-sm text-zinc-400">Checking…</div>
    {/if}
  </Card>

  {#if d}
    <section>
      <div class="section-title">Session health</div>
      <Card>
        {#each [
          ["Graphical session", d.gsession],
          ["Desktop portal", d.portal],
          ["Portal (Hyprland)", d.portal_hypr],
          ["Portal (GTK)", d.portal_gtk]
        ] as [label, state] (label)}
          <div class="flex items-center justify-between px-4 py-2.5">
            <span class="text-sm">{label}</span>
            <span class="flex items-center gap-2 text-xs text-zinc-400">
              {state || "—"}
              <span class="h-2.5 w-2.5 rounded-full {state === 'active' ? 'bg-green-500' : 'bg-red-500'}"></span>
            </span>
          </div>
        {/each}
        <KV k="Default browser" v={d.browser || "—"} />
      </Card>
    </section>
  {/if}
</div>
