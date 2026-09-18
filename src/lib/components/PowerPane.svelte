<script>
  import { onMount } from "svelte";
  import * as api from "../api.js";
  import { prefs, errorMsg, flashApplied } from "../stores.js";
  import { setPrefs } from "../overrides.js";
  import ToggleRow from "./ui/ToggleRow.svelte";
  import SelectRow from "./ui/SelectRow.svelte";
  import KV from "./ui/KV.svelte";

  let info = null;

  async function refresh() {
    try {
      info = await api.powerInfo();
    } catch (e) {
      errorMsg.set(String(e));
    }
  }
  onMount(refresh);

  $: bat = info?.battery;
  $: kbd = info?.kbdBacklight;

  async function chargeLimit(v) {
    try {
      await api.setChargeLimit(Number(v));
      flashApplied(`Charge ceiling → ${v}%`);
      setTimeout(refresh, 400);
    } catch (e) {
      errorMsg.set(String(e));
    }
  }

  async function profile(p) {
    try {
      await api.setPowerProfile(p);
      flashApplied();
      setTimeout(refresh, 400);
    } catch (e) {
      errorMsg.set(String(e));
    }
  }

  async function kbdLevel(v) {
    try {
      await api.setKbdBacklight(Number(v));
      flashApplied();
      setTimeout(refresh, 400);
    } catch (e) {
      errorMsg.set(String(e));
    }
  }

  const healthText = (b) =>
    b?.health != null
      ? `${Math.round(b.health * 100)}% of design${b.cycles != null ? `  ·  ${b.cycles} cycles` : ""}`
      : "unknown";
  import Page from "./ui/Page.svelte";
  import Group from "./ui/Group.svelte";
  import Row from "./ui/Row.svelte";
</script>

<Page title="Power" desc="The lid, the battery, the keyboard light and the power profile.">
  <Group title="When the lid closes">
    <ToggleRow
      title="Suspend even when docked"
      sub="On: closing the lid always suspends. Off: with an external display connected, keep working and turn off the built-in screen."
      on={$prefs.lidDockedSuspend === true}
      toggled={() => setPrefs({ lidDockedSuspend: !($prefs.lidDockedSuspend === true) })}
    />
    <svelte:fragment slot="after">
      <p class="note">
        With no external display, closing the lid always suspends. The session locks first.
      </p>
    </svelte:fragment>
  </Group>

  <Group title="Battery">
    {#if bat}
      <KV k="Charge" v={bat.capacity != null ? `${bat.capacity}% · ${bat.status || ""}` : "—"} />
      <KV k="Health" v={healthText(bat)} />
      {#if bat.chargeLimit != null && bat.chargeLimitWritable}
        <SelectRow
          label="Charge limit"
          sub="Stop charging here to make the battery last longer."
          options={[
            { label: "60% (longest life)", value: 60 },
            { label: "80% (balanced)", value: 80 },
            { label: "100% (full capacity)", value: 100 }
          ]}
          value={bat.chargeLimit}
          picked={chargeLimit}
        />
      {:else if bat.chargeLimit != null}
        <Row title="Charge limit" sub="Only root can change it here. Run install.sh again to add the udev rule, or use asusctl.">
          {bat.chargeLimit}%
        </Row>
      {:else}
        <Row title="Charge limit" sub="This battery has no charge limit setting." />
      {/if}
    {:else}
      <Row sub="No battery found." />
    {/if}
  </Group>

  {#if kbd?.present}
    <Group title="Keyboard backlight">
      <SelectRow
        label="Level"
        sub="The keyboard's backlight keys change it too."
        options={Array.from({ length: (kbd.max ?? 3) + 1 }, (_, i) => ({
          label: i === 0 ? "Off" : `${i} of ${kbd.max}`,
          value: i
        }))}
        value={kbd.value ?? 0}
        picked={kbdLevel}
      />
    </Group>
  {/if}

  <Group title="Performance">
    {#if info?.profiles?.length}
      <SelectRow
        label="Power profile"
        sub="Set by power-profiles-daemon."
        options={info.profiles.map((p) => ({ label: p.charAt(0).toUpperCase() + p.slice(1), value: p }))}
        value={info.profile}
        picked={profile}
      />
    {:else}
      <KV k="Power profile" v="Kernel defaults (no profile daemon)" />
    {/if}
  </Group>

  <Group title="On battery">
    <ToggleRow
      title="Low power mode"
      sub="Check for changes less often while unplugged. The screensaver, lock and suspend come sooner."
      on={$prefs.lowPowerEnabled !== false}
      toggled={() => setPrefs({ lowPowerEnabled: !($prefs.lowPowerEnabled !== false) })}
    />
  </Group>
</Page>
