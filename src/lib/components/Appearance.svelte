<script>
  import { prefs } from "../stores.js";
  import { ACCENTS } from "../hypr.js";
  import {
    setAccent,
    setColorScheme,
    setTransparency,
    setAnimationSpeed,
    setPrefs,
    applyBorder
  } from "../overrides.js";

  const animSpeeds = [
    { label: "Off", value: 0 },
    { label: "Relaxed", value: 0.7 },
    { label: "Default", value: 1 },
    { label: "Snappy", value: 1.5 },
    { label: "Fast", value: 2 }
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
      <span class="block text-sm font-medium">Flock</span>
      <span class="block text-xs text-zinc-400 dark:text-zinc-500">
        The hypr-shell look: solid dark surfaces, rounded corners, Phosphor icons, and the accent colour you pick below.
      </span>
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
      <div class="flex items-center justify-between gap-3 px-4 py-3">
        <div>
          <div class="text-sm font-medium">App colour scheme</div>
          <div class="text-xs text-zinc-400 dark:text-zinc-500">
            GTK and Qt apps follow this; the shell itself is always dark.
          </div>
        </div>
        <div class="flex gap-1.5">
          {#each [["dark", "Dark"], ["light", "Light"]] as [id, label] (id)}
            <button
              class="rounded-full px-3 py-1 text-xs font-medium transition-colors
                {($prefs.colorScheme || 'dark') === id
                ? 'text-white'
                : 'bg-zinc-200/70 text-zinc-500 hover:bg-zinc-300/70 dark:bg-zinc-700/60 dark:text-zinc-300'}"
              style={($prefs.colorScheme || "dark") === id ? "background: var(--accent)" : ""}
              disabled={busy}
              on:click={() => run(() => setColorScheme(id))}
            >
              {label}
            </button>
          {/each}
        </div>
      </div>
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
