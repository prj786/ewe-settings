<script>
  import { onMount } from "svelte";
  import * as api from "../api.js";
  import { shortcutsModel } from "../hypr.js";
  import { addons, loadAddons, hasAddon } from "../addons.js";

  let rows = [];
  onMount(async () => {
    loadAddons();
    try {
      rows = shortcutsModel(await api.readConfig("hypr/SHORTCUTS.md"));
    } catch {}
  });
  import Page from "./ui/Page.svelte";
  import Group from "./ui/Group.svelte";
  import Row from "./ui/Row.svelte";
  // A heading or row that names a plugin (`ewe.screenshot`, `ewe.cast`…)
  // is that plugin's bind: shown only while the plugin is installed and on,
  // so the page never lists a key that does nothing.
  const pluginOf = (text) => (String(text).match(/\b(ewe\.[a-z][a-z0-9_-]*)\b/) || [])[1] || "";
  const live = (text, s) => !pluginOf(text) || hasAddon(s, pluginOf(text));
  // SHORTCUTS.md is a flat list of headings and rows; group it for the page.
  $: groups = rows
    .reduce((acc, r) => {
      if (r.h || !acc.length) acc.push({ title: r.h ? r.a : "", rows: [] });
      if (!r.h) acc[acc.length - 1].rows.push(r);
      return acc;
    }, [])
    .filter((g) => !$addons.loaded || live(g.title, $addons))
    .map((g) => ({ ...g, rows: $addons.loaded ? g.rows.filter((r) => live(r.b, $addons)) : g.rows }))
    .filter((g) => g.rows.length);
  // "Super + Shift + W" → keys, each a Kbd
  const keys = (a) => String(a).split(/\s*\+\s*/).filter(Boolean);
</script>

<Page title="Keyboard shortcuts" desc="Every shortcut the desktop knows. They're always shown, never required.">
  {#if rows.length === 0}
    <p class="note">SHORTCUTS.md wasn't found.</p>
  {/if}

  {#each groups as g, gi (gi)}
    <Group title={g.title}>
      {#each g.rows as r, i (i)}
        <Row title={r.b} dense>
          <span class="ewe-kbd-combo">
            {#each keys(r.a) as k, ki (ki)}{#if ki}<span aria-hidden="true">+</span>{/if}<kbd class="ewe-kbd">{k}</kbd>{/each}
          </span>
        </Row>
      {/each}
    </Group>
  {/each}
</Page>
