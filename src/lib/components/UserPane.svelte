<script>
  // User — who you are on this machine, and your account in the cloud.
  //
  // RFC-005 + RFC-006 (2026-09-03): Settings SHOWS the account, it does not
  // run it. ewe-sync is the account app and owns every verb that changes the
  // account or moves data — sign in, sign out, push, restore, machines,
  // folders — so those controls live there and nowhere else. Adding a mail
  // account went with them. What is left here is read-only status plus the
  // per-machine preferences that are genuinely Settings' own (avatar, name,
  // avatar shape).
  // Google stays: it is an OPTIONAL extra for Gmail and a Drive folder, needs
  // the user's own client file, and ewe-sync does not cover it. This app never
  // touches a token; every verb goes through the shell over IPC.
  // Mail notifications are the Mail plugin's own setting (Komble → Plugins →
  // Mail → Options); the mail account is ewe-sync's.
  import { onMount, onDestroy } from "svelte";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import * as api from "../api.js";
  import { prefs, errorMsg, flashApplied, shellUp } from "../stores.js";
  import { setPrefs } from "../overrides.js";
  import KV from "./ui/KV.svelte";
  import Seg from "./ui/Seg.svelte";

  let info = null;
  let syncApp = false; // ewe-sync (RFC-006) installed → "Open ewe Sync"
  api.syncAppInstalled().then((v) => (syncApp = !!v)).catch(() => {});
  let cloud = null; // `qs ipc call cloud status` (null = no answer yet)
  let google = null; // `qs ipc call google status`
  let gclient = null; // google_client_info
  let faceVersion = 0;
  let nameEdit = "";
  let editingName = false;
  let busy = false; // an IPC verb is in flight
  let timer;
  let avatarVersion = 0;

  const home = () => info?.user ? `/home/${info.user}` : "";
  $: faceUrl = info?.hasFace ? convertFileSrc(`${home()}/.face`) + `?v=${faceVersion}` : "";
  $: cloudAvatar = cloud?.avatarPath ? convertFileSrc(cloud.avatarPath) + `?v=${avatarVersion}` : "";

  /** The last good status stays when a reply is missing or unreadable: one
   *  bad poll (the shell restarting under us) is not "the shell is gone".
   *  Whether the shell runs at all is App.svelte's probe ($shellUp). */
  async function ipcStatus(target, prev) {
    try {
      const raw = (await api.qsIpc(target, "status")).trim();
      return raw ? JSON.parse(raw) : prev;
    } catch {
      return prev;
    }
  }

  async function refresh() {
    try {
      info = await api.userInfo();
    } catch (e) {
      errorMsg.set(String(e));
    }
    const wasSigned = cloud?.signedIn;
    cloud = await ipcStatus("cloud", cloud);
    google = await ipcStatus("google", google);
    if (cloud?.signedIn && !wasSigned) avatarVersion++;
    try {
      gclient = await api.googleClientInfo();
    } catch {
      gclient = null;
    }
  }

  onMount(() => {
    refresh();
    let n = 0;
    timer = setInterval(async () => {
      n++;
      // live while anything is in flight or signed out (Wi-Fi joined from the
      // Network pane must re-enable Sign in without a reopen); every 15 s otherwise
      const live =
        !cloud ||
        cloud.busy ||
        cloud.syncState === "syncing" ||
        cloud.pendingRestore ||
        !cloud.signedIn ||
        (google && google.busy);
      if (live || n % 7 === 0) await refresh();
    }, 2000);
  });
  onDestroy(() => clearInterval(timer));

  /** Pick an image → center-crop square → 512² PNG → ~/.face. */
  async function pickAvatar() {
    const sel = await openDialog({
      multiple: false,
      filters: [{ name: "Images", extensions: ["jpg", "jpeg", "png", "webp"] }]
    });
    if (typeof sel !== "string") return;
    try {
      const img = new Image();
      img.src = convertFileSrc(sel);
      await new Promise((res, rej) => {
        img.onload = res;
        img.onerror = () => rej(new Error("could not read the image"));
      });
      const side = Math.min(img.naturalWidth, img.naturalHeight);
      const c = document.createElement("canvas");
      c.width = c.height = 512;
      c.getContext("2d").drawImage(
        img,
        (img.naturalWidth - side) / 2, (img.naturalHeight - side) / 2, side, side,
        0, 0, 512, 512
      );
      const b64 = c.toDataURL("image/png").split(",")[1];
      const warn = await api.saveAvatar(b64);
      if (warn) errorMsg.set(warn);
      else flashApplied("Picture changed");
      faceVersion++;
      refresh();
    } catch (e) {
      errorMsg.set(String(e));
    }
  }

  /** The account's avatar (already cached by ewe-cloud) as ~/.face. */
  async function useCloudPhoto() {
    if (!cloud?.avatarPath) return;
    try {
      const img = new Image();
      img.src = cloudAvatar;
      await new Promise((res, rej) => {
        img.onload = res;
        img.onerror = () => rej(new Error("could not read the account avatar"));
      });
      const c = document.createElement("canvas");
      c.width = c.height = 512;
      c.getContext("2d").drawImage(img, 0, 0, 512, 512);
      const warn = await api.saveAvatar(c.toDataURL("image/png").split(",")[1]);
      if (warn) errorMsg.set(warn);
      else flashApplied("Picture changed");
      faceVersion++;
      refresh();
    } catch (e) {
      errorMsg.set(String(e));
    }
  }

  async function saveName() {
    editingName = false;
    if (!nameEdit.trim() || nameEdit.trim() === info?.realName) return;
    try {
      await api.setRealName(nameEdit.trim());
      flashApplied("Name updated");
      refresh();
    } catch (e) {
      errorMsg.set(String(e));
    }
  }

  /** Everything that CHANGES the account happens in ewe Sync (ewe-sync). */
  const openSync = () => api.openSyncApp().catch((e) => errorMsg.set(String(e)));

  const shapes = [
    { label: "Circle", value: "circle" },
    { label: "Rounded", value: "rounded" },
    { label: "Square", value: "square" }
  ];
  // "Oct 10, 1:00 PM" — the year only when it isn't this one
  const fmtSync = (iso) => {
    if (!iso) return "never";
    try {
      const d = new Date(iso);
      const sameYear = d.getFullYear() === new Date().getFullYear();
      return d.toLocaleString([], { month: "short", day: "numeric", ...(sameYear ? {} : { year: "numeric" }), hour: "numeric", minute: "2-digit" });
    } catch {
      return iso;
    }
  };
  const fmtBytes = (n) => {
    n = Number(n) || 0;
    const u = ["B", "KB", "MB", "GB", "TB"];
    let i = 0;
    while (n >= 1024 && i < u.length - 1) {
      n /= 1024;
      i++;
    }
    return `${n < 10 && i > 0 ? n.toFixed(1) : Math.round(n)} ${u[i]}`;
  };

  // ── derived ────────────────────────────────────────────────────────────────
  $: quotaPct = cloud?.quota?.total > 0 ? Math.min(100, Math.round((cloud.quota.used / cloud.quota.total) * 100)) : null;
  // no answer yet: "Reading…" while the shell is up, the honest note when not
  $: noShell = cloud === null && !$shellUp;
  import Page from "./ui/Page.svelte";
  import Group from "./ui/Group.svelte";
  import Row from "./ui/Row.svelte";
  $: avatarShape = $prefs.avatarShape || "circle";
  $: avatarClass = avatarShape === "square" ? "ewe-avatar--square rounded-none" : avatarShape === "rounded" ? "ewe-avatar--square" : "";
</script>

<Page title="User" desc="Who you are on this computer, and the accounts it's signed in to.">
  <svelte:fragment slot="actions">
    {#if syncApp}
      <button class="ewe-btn ewe-btn--secondary" on:click={openSync}>Open ewe Sync</button>
    {/if}
  </svelte:fragment>

  <Group title="You">
    <div class="ewe-row user-head" data-setting="Picture and name">
      {#if faceUrl}
        <span class="ewe-avatar ewe-avatar--xl {avatarClass}" style="background-image: url('{faceUrl}')" role="img" aria-label="Your picture"></span>
      {:else}
        <span class="ewe-avatar ewe-avatar--xl {avatarClass}" aria-hidden="true">
          {(info?.realName || info?.user || "?").slice(0, 1).toUpperCase()}
        </span>
      {/if}
      <div class="ewe-row__text">
        {#if editingName}
          <form on:submit|preventDefault={saveName}>
            <!-- svelte-ignore a11y_autofocus -->
            <input class="ewe-input w-full" aria-label="Your name" bind:value={nameEdit} autofocus on:blur={saveName} />
          </form>
        {:else}
          <button class="user-name" title="Change your name"
            on:click={() => { nameEdit = info?.realName || ""; editingName = true; }}>
            {info?.realName || info?.user || "…"}
          </button>
        {/if}
        <div class="ewe-row__desc">{info ? `${info.user} on ${info.host}` : ""}</div>
      </div>
      <div class="ewe-row__trail flex-col items-end">
        <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" on:click={pickAvatar}>Change picture…</button>
        {#if cloud?.signedIn && cloud?.avatarPath}
          <button class="ewe-btn ewe-btn--ghost ewe-btn--sm" disabled={busy} on:click={useCloudPhoto}>Use account picture</button>
        {/if}
      </div>
    </div>
    <Row title="Picture shape" sub="On the lock screen and in the shell">
      <Seg label="Picture shape" options={shapes} value={avatarShape} picked={(v) => setPrefs({ avatarShape: v })} />
    </Row>
    {#if info?.uptime}<KV k="Signed in for" v={info.uptime} />{/if}
  </Group>

  <!-- Read-only by design (RFC-006). ewe Sync owns the account; this shows it. -->
  <Group title="Nextcloud account">
    {#if noShell}
      <Row title="Nextcloud account" sub="Can't be read while the shell isn't running." />
    {:else if cloud === null}
      <Row title="Nextcloud account" sub="Reading…" />
    {:else if !cloud.signedIn}
      <Row title="Not signed in" setting="Nextcloud account" sub="Sign in to keep your settings, files and calendar in your own cloud.">
        {#if syncApp}
          <button class="ewe-btn ewe-btn--primary ewe-btn--sm" on:click={openSync}>Sign in…</button>
        {/if}
      </Row>
    {:else}
      <div class="ewe-row" data-setting="Nextcloud account">
        {#if cloudAvatar}
          <span class="ewe-avatar" style="background-image: url('{cloudAvatar}')" role="img" aria-label="Account picture"></span>
        {:else}
          <span class="ewe-avatar" aria-hidden="true">{(cloud.displayName || cloud.user || "?").slice(0, 1).toUpperCase()}</span>
        {/if}
        <div class="ewe-row__text">
          <div class="ewe-row__title font-medium">{cloud.displayName || cloud.user}</div>
          <div class="ewe-row__desc">
            {cloud.email ? `${cloud.email} · ` : ""}{cloud.serverHost || cloud.server}{#if cloud.offline}<span class="text-warning"> · Offline</span>{/if}
          </div>
        </div>
      </div>
      {#if cloud.quota && cloud.quota.total > 0}
        <div class="ewe-row ewe-row--block">
          <div class="ewe-meter" class:ewe-meter--warning={quotaPct >= 90}>
            <div class="ewe-meter__head">
              <span>Storage</span>
              <span class="ewe-meter__value">{fmtBytes(cloud.quota.used)} of {fmtBytes(cloud.quota.total)}</span>
            </div>
            <div class="ewe-meter__track" role="meter" aria-label="Storage used" aria-valuenow={quotaPct} aria-valuemin="0" aria-valuemax="100">
              <div class="ewe-meter__fill" style="--value: {quotaPct}%"></div>
            </div>
          </div>
        </div>
      {:else if cloud.quota}
        <KV k="Storage" v={`${fmtBytes(cloud.quota.used)} used`} />
      {/if}
      <KV k="Files" v={cloud.filesMounted ? `${cloud.filesPath || "~/Nextcloud"}` : cloud.filesPath ? `${cloud.filesPath} (not connected)` : "—"} />
      <KV k="Calendar" v={cloud.calState === "ok" ? `${cloud.eventCount} upcoming event${cloud.eventCount === 1 ? "" : "s"}` : cloud.calState || "—"} />
      {#if cloud.error}
        <Row sub={cloud.error} />
      {/if}
    {/if}
  </Group>

  <!-- Status only. Back up, sync, push and restore are ewe Sync's verbs. -->
  <Group title="Settings sync" desc="Backups, restoring and your other computers are in ewe Sync.">
    {#if noShell}
      <Row title="Sync status" sub="Unknown while the shell isn't running. ewe Sync still backs up and restores." />
    {:else if cloud === null}
      <Row title="Sync status" sub="Reading…" />
    {:else if !cloud.signedIn}
      <Row title="Sync status" sub="Off. Sign in to bring this computer's settings, apps and look to the next one." />
    {:else}
      <Row
        title="Sync status"
        sub={cloud.syncState === "syncing"
          ? "Syncing…"
          : cloud.syncConflict
            ? cloud.syncError || "The copy in your account changed since this computer last synced."
            : cloud.syncError
              ? cloud.syncError
              : !cloud.lastSync
                ? "Nothing is uploaded until you back up this computer in ewe Sync."
                : cloud.inSync
                  ? "Up to date"
                  : "Changed since the last sync"}
      >
        {#if cloud.syncConflict && syncApp}
          <button class="ewe-btn ewe-btn--primary ewe-btn--sm" on:click={openSync}>Resolve in ewe Sync</button>
        {/if}
      </Row>
      <KV k="Backup in your account"
        v={cloud.remoteMachine || cloud.remoteModified ? `From “${cloud.remoteMachine || "another computer"}”, ${fmtSync(cloud.remoteModified)}` : "None yet"} />
      <KV k="This computer last synced"
        v={cloud.localSyncedAt ? fmtSync(cloud.localSyncedAt) : "Never"} />
      <KV k="Sync by itself" v={cloud.autoSync ? "On" : "Off"} />
    {/if}
  </Group>

  <!-- Read-only, like the account above: connecting, disconnecting and the
       client-file guidance all live in ewe Sync → Google. -->
  <Group title="Google" desc="Optional, for Gmail and a Drive folder. You connect it in ewe Sync, with your own Google client file.">
    <KV k="Your Google client file" setting="Google" v={gclient ? (gclient.valid ? "Found" : gclient.exists ? "Found, but it isn't a desktop app file" : "Not added") : google?.configured ? "Found" : "Not added"} />
    {#if google === null && !$shellUp}
      <Row title="Google account" sub="Can't be read while the shell isn't running." />
    {:else if google === null}
      <Row title="Google account" sub="Reading…" />
    {:else if !google.signedIn}
      <KV k="Google account" v="Not connected" />
    {:else}
      <div class="ewe-row">
        {#if google.profile?.picture}
          <span class="ewe-avatar" style="background-image: url('{google.profile.picture}')" role="img" aria-label="Google account picture"></span>
        {/if}
        <div class="ewe-row__text">
          <div class="ewe-row__title font-medium">{google.profile?.name || "Google"}</div>
          <div class="ewe-row__desc">{google.profile?.email || ""}</div>
        </div>
      </div>
      <KV k="Gmail" v={google.mailState === "scope" ? "No mail permission. Reconnect in ewe Sync." : google.mailState === "ok" ? `${google.mailUnread || 0} unread` : google.mailState || "—"} />
      <KV k="Drive folder" v="~/Google Drive, when you're signed in" />
    {/if}
  </Group>
</Page>
