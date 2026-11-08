<script>
  import { onMount } from "svelte";
  import * as api from "../api.js";
  import { prefs, errorMsg, flashApplied } from "../stores.js";
  import { setPrefs } from "../overrides.js";
  import ToggleRow from "./ui/ToggleRow.svelte";
  import SelectRow from "./ui/SelectRow.svelte";
  import KV from "./ui/KV.svelte";

  let info = null;
  // Game mode (ewe 0.42, bin/ewe-game): undefined = still asking, null = this
  // ewe has no ewe-game yet
  let game = undefined;
  let gameBusy = false;

  async function refresh() {
    try {
      info = await api.powerInfo();
    } catch (e) {
      errorMsg.set(String(e));
    }
    refreshGame();
  }
  async function refreshGame() {
    try {
      game = await api.gameStatus();
    } catch (e) {
      game = null;
    }
  }
  onMount(refresh);

  async function setGame(on) {
    gameBusy = true;
    try {
      game = await api.gameSet(on);
      flashApplied(on ? "Game mode on" : "Game mode off, everything put back");
      setTimeout(refresh, 400);
    } catch (e) {
      errorMsg.set(String(e));
    }
    gameBusy = false;
  }

  $: bat = info?.battery;
  $: kbd = info?.kbdBacklight;

  async function chargeLimit(v) {
    try {
      await api.setChargeLimit(Number(v));
      flashApplied(`Charge limit set to ${v}%`);
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
      ? `${Math.round(b.health * 100)}% of new${b.cycles != null ? ` · ${b.cycles} charges` : ""}`
      : "Unknown";
  const statusWord = (s) =>
    ({ Charging: "charging", Discharging: "on battery", Full: "full", "Not charging": "plugged in, not charging" })[s] ||
    String(s || "").toLowerCase();
  const profileWord = (p) =>
    ({ "power-saver": "Power saver", balanced: "Balanced", performance: "Performance" })[p] ||
    p.charAt(0).toUpperCase() + p.slice(1);
  import Page from "./ui/Page.svelte";
  import Group from "./ui/Group.svelte";
  import Row from "./ui/Row.svelte";
  import Icon from "./ui/Icon.svelte";
</script>

<Page title="Power" desc="The lid, the battery, the keyboard light and the power profile.">
  <Group title="When the lid closes" desc="With no external display, closing the lid always suspends. The session locks first.">
    <ToggleRow
      title="Suspend even when docked"
      sub="Off: with an external display connected, keep working and turn off the built-in screen"
      on={$prefs.lidDockedSuspend === true}
      toggled={() => setPrefs({ lidDockedSuspend: !($prefs.lidDockedSuspend === true) })}
    />
  </Group>

  <Group title="Battery">
    {#if !info}
      <Row sub="Checking…" />
    {:else if bat}
      <KV k="Charge" v={bat.capacity != null ? `${bat.capacity}%${bat.status ? `, ${statusWord(bat.status)}` : ""}` : "—"} />
      <KV k="Health" v={healthText(bat)} />
      {#if bat.chargeLimit != null && bat.chargeLimitWritable}
        <SelectRow
          label="Charge limit"
          sub="Stop charging here to make the battery last longer"
          options={[
            { label: "60%, longest life", value: 60 },
            { label: "80%, balanced", value: 80 },
            { label: "100%, full", value: 100 }
          ]}
          value={bat.chargeLimit}
          picked={chargeLimit}
        />
      {:else if bat.chargeLimit != null}
        <Row title="Charge limit" sub="ewe needs one more permission to change it. Running the ewe installer again adds it.">
          {bat.chargeLimit}%
        </Row>
      {:else}
        <Row title="Charge limit" sub="This battery has no charge limit">—</Row>
      {/if}
    {:else}
      <div class="ewe-empty ewe-empty--compact">
        <span class="ewe-empty__icon"><Icon name="power" /></span>
        <div class="ewe-empty__title">No battery</div>
        <div class="ewe-empty__desc">This computer runs on mains power.</div>
      </div>
    {/if}
  </Group>

  {#if kbd?.present}
    <Group title="Keyboard backlight">
      <SelectRow
        label="Brightness"
        sub="The keyboard's own light keys change it too"
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
        sub="Balanced suits most days. Power saver makes the battery last longer."
        options={info.profiles.map((p) => ({ label: profileWord(p), value: p }))}
        value={info.profile}
        picked={profile}
      />
    {:else}
      <KV k="Power profile" sub="Install power-profiles-daemon from Komble to choose one" v="Not available" />
    {/if}
  </Group>

  <Group
    title="Gaming"
    desc="Game mode keeps banners in the list and turns off window effects. Ultra goes further. Everything goes back when it ends."
  >
    {#if game === undefined}
      <Row sub="Checking…" />
    {:else if game === null}
      <Row title="Game mode" sub="Comes with ewe 0.42. Update ewe from Komble to get it.">—</Row>
    {:else}
      <ToggleRow
        title="Game mode"
        sub={game.on
          ? `On, ${game.level === "ultra" ? "Ultra" : "Balanced"}${game.reasons?.includes("manual") ? "" : ", while a game runs"}`
          : "Also in Quick settings"}
        on={game.on}
        dim={gameBusy}
        toggled={() => !gameBusy && setGame(!game.on)}
      />
      <SelectRow
        label="Game mode level"
        sub={($prefs.gameLevel || "ultra") === "ultra"
          ? "Also the Performance power profile, ewe's background apps closed, and a fullscreen game sent straight to the display"
          : "Quiet banners, no window effects, and GameMode's CPU boost"}
        options={[
          { label: "Ultra", value: "ultra" },
          { label: "Balanced", value: "balanced" }
        ]}
        value={$prefs.gameLevel || "ultra"}
        picked={(v) => setPrefs({ gameLevel: v })}
      />
      <ToggleRow
        title="Turn on while a game runs"
        sub="A Steam game, gamescope, or any game that asks GameMode for a boost"
        on={$prefs.gameAuto !== false}
        toggled={() => setPrefs({ gameAuto: $prefs.gameAuto === false })}
      />
      <ToggleRow
        title="Close background apps"
        sub="Ultra closes ewe Sync and ewe Browser's background process, and opens them again afterwards"
        on={$prefs.gameCloseApps !== false}
        dim={($prefs.gameLevel || "ultra") !== "ultra"}
        toggled={() => setPrefs({ gameCloseApps: $prefs.gameCloseApps === false })}
      />
      {#if game.on && game.steps?.length}
        {#each game.steps as s}
          <KV k={s.ok ? "Done" : "Skipped"} v={s.note} hint={s.note} />
        {/each}
      {/if}
      {#if !game.ready?.gamemode}
        <Row title="CPU boost" sub="Install gamemode from Komble, or the gaming stack, for the CPU boost">Not installed</Row>
      {:else if !game.ready?.boost_allowed}
        <Row title="CPU boost" sub="GameMode needs ewe's permission for it. Updating ewe (sudo pacman -Syu) adds it.">Not allowed yet</Row>
      {/if}
    {/if}
  </Group>

  <Group title="On battery">
    <ToggleRow
      title="Low power mode"
      sub="While unplugged, check for changes less often, and the screensaver, lock and suspend come sooner"
      on={$prefs.lowPowerEnabled !== false}
      toggled={() => setPrefs({ lowPowerEnabled: !($prefs.lowPowerEnabled !== false) })}
    />
  </Group>
</Page>
