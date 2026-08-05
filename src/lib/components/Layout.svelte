<script>
  import { onMount } from "svelte";
  import { prefs } from "../stores.js";
  import { layout, loadLayout, applyGaps, setTiling, setPrefs } from "../overrides.js";
  import Card from "./ui/Card.svelte";
  import ToggleRow from "./ui/ToggleRow.svelte";
  import SliderRow from "./ui/SliderRow.svelte";

  onMount(loadLayout);

  function setLayout(patch) {
    layout.update((l) => ({ ...l, ...patch }));
    applyGaps();
  }

  const iconSizes = [
    ["small", "Small"],
    ["normal", "Normal"],
    ["large", "Large"]
  ];
</script>

<div class="mx-auto max-w-3xl space-y-6 p-5 sm:p-8">
  <h1 class="text-lg font-semibold">Layout & Dock</h1>

  <section>
    <div class="section-title">Window behaviour</div>
    <Card>
      <ToggleRow
        title="Tiling"
        sub="Off: every new window opens floating, like a stacking desktop. Applies via a config reload."
        on={$prefs.tilingEnabled !== false}
        toggled={() => setTiling(!($prefs.tilingEnabled !== false))}
      />
    </Card>
  </section>

  <section>
    <div class="section-title">Gaps & borders</div>
    <Card>
      <SliderRow label="Inner gaps" value={$layout.gapsIn} from={0} to={40} unit=" px" moved={(v) => setLayout({ gapsIn: Math.round(v) })} />
      <SliderRow label="Outer gaps" value={$layout.gapsOut} from={0} to={60} unit=" px" moved={(v) => setLayout({ gapsOut: Math.round(v) })} />
      <SliderRow label="Border width" value={$layout.borderSize} from={0} to={8} unit=" px" moved={(v) => setLayout({ borderSize: Math.round(v) })} />
      <SliderRow label="Corner radius" value={$layout.rounding} from={0} to={24} unit=" px" moved={(v) => setLayout({ rounding: Math.round(v) })} />
    </Card>
  </section>

  <section>
    <div class="section-title">Dock</div>
    <Card>
      <ToggleRow
        title="Show the dock"
        sub="The bottom dock with pinned apps, launcher and places."
        on={$prefs.dockEnabled !== false}
        toggled={() => setPrefs({ dockEnabled: !($prefs.dockEnabled !== false) })}
      />
      <ToggleRow
        title="Intelligent autohide"
        sub="Slide away when a window needs the space; reveal on bottom-edge hover."
        dim={$prefs.dockEnabled === false}
        on={!!$prefs.dockAutohide}
        toggled={() => setPrefs({ dockAutohide: !$prefs.dockAutohide })}
      />
      <div
        class="flex flex-wrap items-center justify-between gap-x-3 gap-y-1.5 px-4 py-3
          {$prefs.dockEnabled === false ? 'pointer-events-none opacity-40' : ''}"
      >
        <div>
          <div class="text-sm font-medium">Icon size</div>
          <div class="text-xs text-zinc-400 dark:text-zinc-500">How big the dock buttons and workspace boxes are.</div>
        </div>
        <div class="flex gap-1.5">
          {#each iconSizes as [id, label] (id)}
            <button
              class="rounded-full px-3 py-1 text-xs font-medium transition-colors
                {($prefs.dockIconSize || 'normal') === id
                ? 'text-white'
                : 'bg-zinc-200/70 text-zinc-500 hover:bg-zinc-300/70 dark:bg-zinc-700/60 dark:text-zinc-300'}"
              style={($prefs.dockIconSize || "normal") === id ? "background: var(--accent)" : ""}
              on:click={() => setPrefs({ dockIconSize: id })}
            >
              {label}
            </button>
          {/each}
        </div>
      </div>
    </Card>
  </section>
</div>
