<script>
  import { onMount } from "svelte";
  import * as api from "../api.js";
  import { errorMsg, flashApplied } from "../stores.js";
  import { cleanExec } from "../hypr.js";
  import Toggle from "./ui/Toggle.svelte";
  import IconBtn from "./ui/IconBtn.svelte";

  let apps = []; // [{name, exec, icon, enabled}]
  let allApps = [];
  let query = "";
  let showAdd = false;

  onMount(async () => {
    try {
      const j = JSON.parse((await api.readConfig("quickshell/startup-apps.json")) || "{}");
      if (j && Array.isArray(j.apps)) apps = j.apps;
    } catch {}
    try {
      allApps = await api.desktopApps();
    } catch (e) {
      errorMsg.set(String(e));
    }
  });

  async function save() {
    try {
      await api.writeConfig("quickshell/startup-apps.json", JSON.stringify({ apps }, null, 2));
      flashApplied("Saved. Applies the next time you sign in.");
    } catch (e) {
      errorMsg.set(String(e));
    }
  }

  function add(a) {
    const exec = cleanExec(a.exec);
    if (apps.some((x) => x.exec === exec)) return;
    apps = [...apps, { name: a.name, exec, icon: a.icon || "", enabled: true }];
    showAdd = false;
    query = "";
    save();
  }
  function remove(i) {
    apps = apps.filter((_, j) => j !== i);
    save();
  }
  function toggle(i) {
    apps = apps.map((a, j) => (j === i ? { ...a, enabled: a.enabled === false } : a));
    save();
  }

  $: candidates = allApps.filter(
    (a) =>
      !apps.some((x) => x.exec === cleanExec(a.exec)) &&
      (query.trim() === "" || a.name.toLowerCase().includes(query.trim().toLowerCase()))
  );
  import Page from "./ui/Page.svelte";
  import Group from "./ui/Group.svelte";
  import Row from "./ui/Row.svelte";
  import Icon from "./ui/Icon.svelte";
  import PickList from "./ui/PickList.svelte";
</script>

<Page title="Startup apps" desc="Apps that open by themselves every time you sign in.">
  <svelte:fragment slot="actions">
    <button class="ewe-btn {showAdd ? 'ewe-btn--secondary' : 'ewe-btn--primary'}" on:click={() => (showAdd = !showAdd)}>
      {#if !showAdd}<Icon name="plus" />{/if}
      {showAdd ? "Cancel" : "Add app"}
    </button>
  </svelte:fragment>

  {#if showAdd}
    <Group title="Add an app">
      <PickList
        bind:query
        placeholder="Search apps"
        items={candidates.slice(0, 40).map((a) => ({ id: a.id, title: a.name, trail: a.comment || a.exec, a }))}
        pick={(it) => add(it.a)}
      />
    </Group>
  {/if}

  {#if apps.length === 0}
    <div class="ewe-list">
      <div class="ewe-empty">
        <span class="ewe-empty__icon"><Icon name="rocket" /></span>
        <div class="ewe-empty__title">Nothing starts by itself</div>
        <div class="ewe-empty__desc">Add an app to open it every time you sign in.</div>
      </div>
    </div>
  {:else}
    <Group>
      {#each apps as a, i (a.exec)}
        <Row title={a.name} sub={a.exec}>
          <Toggle on={a.enabled !== false} label={a.name} toggled={() => toggle(i)} />
          <IconBtn name="x" title="Remove {a.name}" danger go={() => remove(i)} />
        </Row>
      {/each}
    </Group>
  {/if}

  <p class="note">The shell opens enabled apps when you sign in. Changes apply the next time you sign in.</p>
</Page>
