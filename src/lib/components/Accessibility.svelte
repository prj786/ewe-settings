<script>
  /**
   * Accessibility (design/system/components/AccessibilityModes): four role
   * remaps that work with every scheme and look preset. Stored in ewe.conf
   * [desktop.accessibility] through ewe-conf; ewe-theme applies them to the
   * shell's tokens and to this app's (lib/theme.js), so the page itself
   * changes the moment a mode does.
   */
  import * as api from "../api.js";
  import { errorMsg } from "../stores.js";
  import { theme, refreshTheme } from "../theme.js";
  import Page from "./ui/Page.svelte";
  import Group from "./ui/Group.svelte";
  import Row from "./ui/Row.svelte";
  import ToggleRow from "./ui/ToggleRow.svelte";
  import Seg from "./ui/Seg.svelte";

  let busy = false;
  // the live values are what ewe-theme last built from, never a second copy
  $: a = ($theme && $theme.accessibility) || {};

  async function set(key, value) {
    busy = true;
    try {
      await api.setConf(`desktop.accessibility.${key}`, value);
      await refreshTheme();
    } catch (e) {
      errorMsg.set(String(e));
    }
    busy = false;
  }
  const truthy = (v) => v === true || String(v).toLowerCase() === "true";
</script>

<Page title="Accessibility" desc="Change how the desktop looks and moves. Every mode works with every scheme.">
  <Group title="Seeing">
    <ToggleRow
      icon="contrast"
      title="Increase contrast"
      sub="Stronger outlines and text, a 2px focus ring, and a solid bar and dock."
      on={truthy(a.increase_contrast)}
      dim={busy}
      toggled={() => set("increase_contrast", !truthy(a.increase_contrast))}
    />
    <ToggleRow
      icon="eye"
      title="Reduce transparency"
      sub="The bar, dock and lock screen turn solid with no blur. App blur and window transparency turn off."
      on={truthy(a.reduce_transparency)}
      dim={busy}
      toggled={() => set("reduce_transparency", !truthy(a.reduce_transparency))}
    />
    <Row icon="textSize" title="Text size" sub="Text and the controls around it grow to fit. At 130% the bar’s icons are one size larger." dim={busy}>
      <Seg
        label="Text size"
        options={[[100, "100%"], [115, "115%"], [130, "130%"]]}
        value={Number(a.text_scale ?? 100)}
        disabled={busy}
        picked={(v) => set("text_scale", v)}
      />
    </Row>
  </Group>

  <Group title="Motion">
    <ToggleRow
      icon="motion"
      title="Reduce motion"
      sub="Slides and zooms become quick fades. Nothing bounces or scales. Spinners keep turning."
      on={truthy(a.reduce_motion)}
      dim={busy}
      toggled={() => set("reduce_motion", !truthy(a.reduce_motion))}
    />
  </Group>
</Page>
