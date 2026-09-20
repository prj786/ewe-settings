<script>
  import { tick } from "svelte";
  import { Switch } from "./switch/index.js";
  export let on = false;
  export let disabled = false;
  export let label = "";
  export let toggled = () => {};

  // A row disables its switch while the change applies, and a disabled
  // button drops keyboard focus — the next Space went nowhere. Remember that
  // the switch held focus when it was disabled, and hand it back after.
  let host;
  let hadFocus = false;
  $: if (host) {
    if (disabled) {
      hadFocus = hadFocus || host.contains(document.activeElement);
    } else if (hadFocus) {
      hadFocus = false;
      tick().then(() => {
        const a = document.activeElement;
        if (!a || a === document.body) host?.querySelector("button")?.focus();
      });
    }
  }
</script>

<!-- Controlled: `on` stays the single source of truth, so the switch is told
     its state and only ever reports intent back through toggled(). -->
<span bind:this={host} style="display: contents">
  <Switch
    checked={on}
    {disabled}
    aria-label={label || "Toggle"}
    onCheckedChange={(v) => toggled(v)}
  />
</span>
