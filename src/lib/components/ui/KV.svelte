<script>
  // A read-only fact as a settings row: the name on the left like any row,
  // the value in the control column in text-secondary — never the muted
  // "disabled" look (KeyValue card, Settings page).
  import Row from "./Row.svelte";
  export let k = "";
  export let v = "";
  export let sub = "";
  export let mono = false;
  export let hint = ""; // the full text behind a shortened value
  export let setting = ""; // what Find a setting looks for, when not the name
</script>

<Row title={k} {sub} {setting}>
  <span class="kv-value {mono ? 'ewe-kv__mono' : ''}" title={hint || v}>{v}</span>
</Row>
