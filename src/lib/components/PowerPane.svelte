<script>
  import { onMount } from "svelte";
  import * as api from "../api.js";
  import { prefs, errorMsg, flashApplied } from "../stores.js";
  import { setPrefs } from "../overrides.js";
  import Card from "./ui/Card.svelte";
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
</script>

<div class="mx-auto max-w-3xl space-y-6 p-5 sm:p-8">
  <h1 class="text-lg font-semibold">Power</h1>

  <section>
    <div class="section-title">When the lid closes</div>
    <Card>
      <ToggleRow
        title="Suspend even when docked"
        sub="On: closing the lid always suspends. Off (default): with an external monitor connected, keep working and just blank the built-in panel."
        on={$prefs.lidDockedSuspend === true}
        toggled={() => setPrefs({ lidDockedSuspend: !($prefs.lidDockedSuspend === true) })}
      />
    </Card>
    <p class="mt-2 text-xs text-zinc-400 dark:text-zinc-500">
      With the laptop alone, closing the lid always suspends. The session locks on the way down,
      before the machine sleeps.
    </p>
  </section>

  <section>
    <div class="section-title">Battery</div>
    <Card>
      {#if bat}
        <KV k="Charge" v={bat.capacity != null ? `${bat.capacity}%  ·  ${bat.status || ""}` : "—"} />
        <KV k="Health" v={healthText(bat)} />
        {#if bat.chargeLimit != null && bat.chargeLimitWritable}
          <SelectRow
            label="Charge ceiling"
            options={[
              { label: "60%  (longest life)", value: 60 },
              { label: "80%  (balanced)", value: 80 },
              { label: "100%  (full capacity)", value: 100 }
            ]}
            value={bat.chargeLimit}
            picked={chargeLimit}
          />
        {:else if bat.chargeLimit != null}
          <div class="px-4 py-2.5 text-xs text-amber-500">
            Charge ceiling is {bat.chargeLimit}%, but the attribute is root-only here.
            Re-run install.sh to add the udev rule, or manage it with asusctl.
          </div>
        {:else}
          <div class="px-4 py-2.5 text-xs text-zinc-400">This machine's battery exposes no charge-ceiling control.</div>
        {/if}
      {:else}
        <div class="px-4 py-3 text-sm text-zinc-400">No battery detected.</div>
      {/if}
    </Card>
  </section>

  {#if kbd?.present}
    <section>
      <div class="section-title">Keyboard backlight</div>
      <Card>
        <SelectRow
          label="Level"
          options={Array.from({ length: (kbd.max ?? 3) + 1 }, (_, i) => ({
            label: i === 0 ? "Off" : `${i} / ${kbd.max}`,
            value: i
          }))}
          value={kbd.value ?? 0}
          picked={kbdLevel}
        />
        <KV k="Hotkeys" v="the keyboard-backlight keys step this too" />
      </Card>
    </section>
  {/if}

  <section>
    <div class="section-title">Performance</div>
    <Card>
      {#if info?.profiles?.length}
        <SelectRow
          label="Power profile"
          sub="power-profiles-daemon"
          options={info.profiles.map((p) => ({ label: p, value: p }))}
          value={info.profile}
          picked={profile}
        />
      {:else}
        <KV k="Profile daemon" v="none — kernel defaults" />
      {/if}
    </Card>
  </section>

  <section>
    <div class="section-title">On battery</div>
    <Card>
      <ToggleRow
        title="Low-power mode"
        sub="Slow background polling down while unplugged; screensaver/lock/suspend land sooner."
        on={$prefs.lowPowerEnabled !== false}
        toggled={() => setPrefs({ lowPowerEnabled: !($prefs.lowPowerEnabled !== false) })}
      />
    </Card>
  </section>
</div>
