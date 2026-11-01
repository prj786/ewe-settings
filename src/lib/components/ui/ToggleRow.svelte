<script>
  // A setting with a switch: a click anywhere on the row toggles it
  // (Switch card, "Clicking anywhere on the settings row toggles").
  import Row from "./Row.svelte";
  import Toggle from "./Toggle.svelte";
  export let title = "";
  export let sub = "";
  export let icon = "";
  export let on = false;
  export let dim = false;
  export let indent = false;
  export let toggled = () => {};
</script>

<Row {title} {sub} {icon} {dim} {indent} onRow={() => toggled(!on)}>
  <Toggle {on} disabled={dim} label={title} {toggled} />
</Row>
