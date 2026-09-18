<script>
  import { onMount } from "svelte";
  import * as api from "../api.js";
  import { shortcutsModel } from "../hypr.js";

  let rows = [];
  onMount(async () => {
    try {
      rows = shortcutsModel(await api.readConfig("hypr/SHORTCUTS.md"));
    } catch {}
  });
  import Page from "./ui/Page.svelte";
  import Group from "./ui/Group.svelte";
  import Row from "./ui/Row.svelte";
  // SHORTCUTS.md is a flat list of headings and rows; group it for the page.
  $: groups = rows.reduce((acc, r) => {
    if (r.h || !acc.length) acc.push({ title: r.h ? r.a : "", rows: [] });
    if (!r.h) acc[acc.length - 1].rows.push(r);
    return acc;
  }, []);
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
