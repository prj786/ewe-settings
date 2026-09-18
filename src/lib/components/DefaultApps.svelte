<script>
  import { onMount } from "svelte";
  import * as api from "../api.js";
  import { APP_CATS } from "../hypr.js";
  import { errorMsg, flashApplied } from "../stores.js";
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
  import Page from "./ui/Page.svelte";
  import Group from "./ui/Group.svelte";
  import Row from "./ui/Row.svelte";
  const sentence = (s) => s.charAt(0) + s.slice(1).toLowerCase();
</script>

<Page title="Default apps" desc="Which app opens links, mail, text, pictures, videos and folders.">
  <Group>
    {#each APP_CATS as c (c.key)}
      {@const opts = [
        ...new Set([...(choices[c.key] || []), ...(defaults[c.key] ? [defaults[c.key]] : [])])
      ].map((id) => ({ label: pretty(id), value: id }))}
      {#if opts.length}
        <SelectRow label={sentence(c.key)} options={opts} value={defaults[c.key] || ""} picked={(v) => set(c.key, v)} />
      {:else}
        <Row title={sentence(c.key)}>No apps installed for this</Row>
      {/if}
    {/each}
  </Group>
  <p class="note">
    Stored in ~/.config/mimeapps.list, which every GTK app and xdg-open read.
  </p>
</Page>
