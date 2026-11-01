<script>
  import { onMount } from "svelte";
  import * as api from "../api.js";
  import { APP_CATS } from "../hypr.js";
  import { errorMsg, flashApplied } from "../stores.js";
  import SelectRow from "./ui/SelectRow.svelte";

  let defaults = {}; // category key → desktop id
  let choices = {}; // category key → [desktop ids]
  let names = {}; // desktop id → the app's own name (its desktop entry)
  let loaded = false;

  // An app's name, never its desktop-file id; an id with no readable entry
  // is tidied ("org.gnome.Loupe.desktop" → "Loupe") as the last resort.
  const tidy = (id) => {
    const bare = String(id || "").replace(/\.desktop$/, "");
    const last = bare.split(".").pop() || bare;
    return last ? last.charAt(0).toUpperCase() + last.slice(1).replace(/[-_]+/g, " ") : "";
  };
  const nameOf = (id) => names[id] || tidy(id);

  async function refresh() {
    await Promise.all(
      APP_CATS.map(async (c) => {
        const [d, a] = await Promise.all([
          api.mimeDefault(c.mime).catch(() => ""),
          api.mimeApps(c.mime).catch(() => [])
        ]);
        defaults = { ...defaults, [c.key]: d };
        choices = { ...choices, [c.key]: a };
      })
    );
    const ids = [...new Set([...Object.values(defaults), ...Object.values(choices).flat()].filter(Boolean))];
    try {
      names = { ...names, ...(await api.appNames(ids)) };
    } catch {}
    loaded = true;
  }
  onMount(refresh);

  // the options for one kind of file: names, and the id only to tell two
  // apps with the same name apart
  function optionsFor(key) {
    const ids = [...new Set([...(choices[key] || []), ...(defaults[key] ? [defaults[key]] : [])])];
    const label = (id) => nameOf(id);
    const dup = (id) => ids.filter((x) => label(x) === label(id)).length > 1;
    return ids
      .map((id) => ({ label: dup(id) ? `${label(id)} (${tidy(id)})` : label(id), value: id }))
      .sort((a, b) => a.label.localeCompare(b.label));
  }

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
  import Icon from "./ui/Icon.svelte";
</script>

<Page title="Default apps" desc="Which app opens links, mail, text, pictures, videos and folders.">
  <Group title="Open with" desc="Every app follows these, and so do links you open from a terminal.">
    {#each APP_CATS as c (c.key)}
      {@const opts = optionsFor(c.key)}
      {#if !loaded}
        <Row title={sentence(c.key)}>…</Row>
      {:else if opts.length}
        <SelectRow label={sentence(c.key)} options={opts} value={defaults[c.key] || ""} picked={(v) => set(c.key, v)} />
      {:else}
        <Row title={sentence(c.key)}>None installed</Row>
      {/if}
    {/each}
  </Group>
</Page>
