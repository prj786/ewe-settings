<script>
  /**
   * A settings page (App shell, Settings page). One grammar on every page:
   * the title with a one-line description and at most one page action, an
   * inline alert only when something needs attention, then overline groups
   * of rows. The header stays put; the groups scroll under it.
   */
  import { shellUp, shellRestarting, restartShellNow } from "../../stores.js";
  import Alert from "./Alert.svelte";
  export let title = "";
  export let desc = "";
</script>

<header class="ewe-headerbar ewe-headerbar--page page-head">
  <div class="ewe-headerbar__titles">
    <h1 class="ewe-headerbar__title">{title}</h1>
    {#if desc}<p class="ewe-headerbar__subtitle">{desc}</p>{/if}
  </div>
  {#if $$slots.actions}
    <div class="ewe-headerbar__actions"><slot name="actions" /></div>
  {/if}
</header>
<div class="ewe-appwin__scroll page-scroll">
  <div class="page">
    {#if !$shellUp}
      <Alert tone="warning" title="The shell isn't running">
        Changes are saved and apply when it starts again.
        <svelte:fragment slot="actions">
          <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" disabled={$shellRestarting} on:click={restartShellNow}>
            {$shellRestarting ? "Starting…" : "Start the shell"}
          </button>
        </svelte:fragment>
      </Alert>
    {/if}
    <slot />
  </div>
</div>
