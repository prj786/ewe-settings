<script>
  import { prefs, pane } from "../stores.js";
  import { ACCENTS } from "../hypr.js";
  import { Checkbox } from "./ui/checkbox/index.js";
  import {
    setAccent,
    setTransparency,
    setPrefs,
    applyBorder
  } from "../overrides.js";

  const shellStyles = [
    {
      id: "flock",
      name: "Flock",
      sub: "The ewe look in soft dark greys.",
      swatch: "#1c1c1e"
    },
    {
      id: "blacksheep",
      name: "Black Sheep",
      sub: "The same look on absolute black (#020202) surfaces.",
      swatch: "#020202"
    }
  ];

  let busy = false;
  async function run(fn) {
    busy = true;
    try {
      await fn();
    } catch (e) {
      console.error(e);
    }
    busy = false;
  }
</script>

<div class="mx-auto max-w-3xl space-y-6 p-5 sm:p-8">
  <h1 class="text-lg font-semibold">Appearance</h1>

  <section>
    <div class="section-title">Shell style</div>
    <div class="card p-4">
      <div class="mb-3 grid grid-cols-2 gap-2.5">
        {#each shellStyles as st (st.id)}
          <button
            class="rounded-lg border p-3 text-left transition-colors
              {($prefs.themeName || 'flock') === st.id
              ? 'border-[var(--accent)] ring-1 ring-[var(--accent)]'
              : 'border-zinc-300 hover:bg-zinc-100 dark:border-zinc-700 dark:hover:bg-zinc-800/60'}"
            disabled={busy}
            on:click={() => run(() => setPrefs({ themeName: st.id }))}
          >
            <span class="mb-2 block h-8 w-full rounded border border-zinc-700/60" style="background: {st.swatch}"></span>
            <span class="block text-sm font-medium">{st.name}</span>
            <span class="block text-xs text-zinc-400 dark:text-zinc-500">{st.sub}</span>
          </button>
        {/each}
      </div>
      <p class="text-xs text-zinc-400 dark:text-zinc-500">
        Bar, dock and panels follow the style; shape, Phosphor icons and your accent stay identical. Applies instantly.
      </p>
    </div>
  </section>

  <section>
    <div class="section-title">Accent colour</div>
    <div class="card p-4">
        <div class="mb-3 flex flex-wrap gap-2.5">
          {#each ACCENTS as a (a.hex)}
            <button
              title={a.name}
              class="h-8 w-8 rounded-full transition-transform hover:scale-110
                {($prefs.accent || '#0a84ff').toLowerCase() === a.hex ? 'ring-2 ring-offset-2 ring-offset-white dark:ring-offset-zinc-900' : ''}"
              style="background: {a.hex}; --tw-ring-color: {a.hex}"
              disabled={busy}
              on:click={() => run(() => setAccent(a.hex))}
            ></button>
          {/each}
        </div>
        <p class="text-xs text-zinc-400 dark:text-zinc-500">
          Applies to the shell, window borders and GTK/Qt apps.
        </p>
    </div>
  </section>

  <section>
    <div class="section-title">Windows & animations</div>
    <div class="card divide-y divide-zinc-200 dark:divide-zinc-700/60">
      <!-- Light mode is parked until it is actually fully light — the DE is
           dark-only for now, so no colour-scheme toggle here. -->
      <div class="flex items-center justify-between gap-3 px-4 py-3">
        <div>
          <div class="text-sm font-medium">Tint window borders</div>
          <div class="text-xs text-zinc-400 dark:text-zinc-500">Active window border follows the accent.</div>
        </div>
        <Checkbox
          checked={!!$prefs.tintBorders}
          disabled={busy}
          aria-label="Tint window borders"
          onCheckedChange={(v) =>
            run(async () => {
              await setPrefs({ tintBorders: v });
              await applyBorder();
            })}
        />
      </div>
      <div class="flex items-center justify-between gap-3 px-4 py-3">
        <div>
          <div class="text-sm font-medium">Window transparency</div>
          <div class="text-xs text-zinc-400 dark:text-zinc-500">Unfocused windows slightly translucent.</div>
        </div>
        <Checkbox
          checked={$prefs.windowTransparency !== false}
          disabled={busy}
          aria-label="Window transparency"
          onCheckedChange={(v) => run(() => setTransparency(v))}
        />
      </div>
      <div class="flex items-center justify-between gap-3 px-4 py-3">
        <div>
          <div class="text-sm font-medium">Event sounds</div>
          <div class="text-xs text-zinc-400 dark:text-zinc-500">Chimes for notifications, volume, screenshots and power events.</div>
        </div>
        <Checkbox
          checked={$prefs.eventSounds !== false}
          disabled={busy}
          aria-label="Event sounds"
          onCheckedChange={(v) => run(() => setPrefs({ eventSounds: v }))}
        />
      </div>
      <div class="flex items-center justify-between gap-3 px-4 py-3">
        <div>
          <div class="text-sm font-medium">Animations</div>
          <div class="text-xs text-zinc-400 dark:text-zinc-500">Speed, presets, curves and styles moved to their own page.</div>
        </div>
        <button
          class="shrink-0 rounded-full bg-zinc-200/70 px-3 py-1 text-xs font-medium text-zinc-600 transition-colors hover:bg-zinc-300/70 dark:bg-zinc-700/60 dark:text-zinc-200 dark:hover:bg-zinc-600/60"
          on:click={() => pane.set("animations")}
        >
          Open Animations
        </button>
      </div>
    </div>
  </section>
</div>
