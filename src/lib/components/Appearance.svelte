<script>
  import { prefs } from "../stores.js";
  import { ACCENTS } from "../hypr.js";
  import {
    setAccent,
    setTransparency,
    setAnimationSpeed,
    setPrefs,
    applyBorder
  } from "../overrides.js";

  // Mirrors the shell's presets: Off/Fast/Normal/Slow = shell durations
  // 0/150/300/500 ms (the multiplier divides the base durations) — steps far
  // enough apart to actually feel different.
  const animSpeeds = [
    { label: "Off", value: 0 },
    { label: "Fast", value: 2 },
    { label: "Normal", value: 1 },
    { label: "Slow", value: 0.6 }
  ];

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
        <input
          type="checkbox"
          class="h-4 w-4 accent-[var(--accent)]"
          checked={!!$prefs.tintBorders}
          disabled={busy}
          on:change={(e) =>
            run(async () => {
              await setPrefs({ tintBorders: e.target.checked });
              await applyBorder();
            })}
        />
      </div>
      <div class="flex items-center justify-between gap-3 px-4 py-3">
        <div>
          <div class="text-sm font-medium">Window transparency</div>
          <div class="text-xs text-zinc-400 dark:text-zinc-500">Unfocused windows slightly translucent.</div>
        </div>
        <input
          type="checkbox"
          class="h-4 w-4 accent-[var(--accent)]"
          checked={$prefs.windowTransparency !== false}
          disabled={busy}
          on:change={(e) => run(() => setTransparency(e.target.checked))}
        />
      </div>
      <div class="flex flex-wrap items-center justify-between gap-x-3 gap-y-1.5 px-4 py-3">
        <div>
          <div class="text-sm font-medium">Animation speed</div>
          <div class="text-xs text-zinc-400 dark:text-zinc-500">Shell and window animations.</div>
        </div>
        <div class="flex flex-wrap gap-1.5">
          {#each animSpeeds as s (s.value)}
            <button
              class="rounded-full px-3 py-1 text-xs font-medium transition-colors
                {Math.abs(($prefs.animationSpeed ?? 1) - s.value) < 0.01
                ? 'text-white'
                : 'bg-zinc-200/70 text-zinc-500 hover:bg-zinc-300/70 dark:bg-zinc-700/60 dark:text-zinc-300'}"
              style={Math.abs(($prefs.animationSpeed ?? 1) - s.value) < 0.01 ? "background: var(--accent)" : ""}
              disabled={busy}
              on:click={() => run(() => setAnimationSpeed(s.value))}
            >
              {s.label}
            </button>
          {/each}
        </div>
      </div>
    </div>
  </section>
</div>
