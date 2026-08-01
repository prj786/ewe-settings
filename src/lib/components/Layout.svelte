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
    </Card>
  </section>
</div>
