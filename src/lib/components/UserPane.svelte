<script>
  // User — who you are on this machine, and your account in the cloud.
  //
  // RFC-005 + RFC-006 (2026-09-03): Settings SHOWS the account, it does not
  // run it. ewe-sync is the account app and owns every verb that changes the
  // account or moves data — sign in, sign out, push, restore, machines,
  // folders — so those controls live there and nowhere else. Adding a mail
  // account went with them. What is left here is read-only status plus the
  // per-machine preferences that are genuinely Settings' own (avatar, name,
  // avatar shape, the new-mail notification).
  // Google stays: it is an OPTIONAL extra for Gmail and a Drive folder, needs
  // the user's own client file, and ewe-sync does not cover it. This app never
  // touches a token; every verb goes through the shell over IPC.
  import { onMount, onDestroy } from "svelte";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import * as api from "../api.js";
  import { prefs, errorMsg, flashApplied } from "../stores.js";
  import { setPrefs } from "../overrides.js";
  import KV from "./ui/KV.svelte";
  import SelectRow from "./ui/SelectRow.svelte";
  import ToggleRow from "./ui/ToggleRow.svelte";

  let info = null;
  let syncApp = false; // ewe-sync (RFC-006) installed → "Manage in ewe-sync"
  api.syncAppInstalled().then((v) => (syncApp = !!v)).catch(() => {});
  let cloud = null; // `qs ipc call cloud status` (null = shell absent)
  let google = null; // `qs ipc call google status`
  let mail = null; // `qs ipc call mail status`
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

  async function ipcStatus(target) {
    try {
      const raw = (await api.qsIpc(target, "status")).trim();
      return raw ? JSON.parse(raw) : null;
    } catch {
      return null; // shell not running
    }
  }

  async function refresh() {
    try {
      info = await api.userInfo();
    } catch (e) {
      errorMsg.set(String(e));
    }
    const wasSigned = cloud?.signedIn;
    cloud = await ipcStatus("cloud");
    google = await ipcStatus("google");
    mail = await ipcStatus("mail");
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
      else flashApplied("Avatar updated");
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
      else flashApplied("Avatar updated");
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

  /** One shell verb; the pane re-reads status shortly after. */
  async function call(target, verb, arg) {
    busy = true;
    try {
      await api.qsIpc(target, verb, arg);
      setTimeout(refresh, 800);
    } catch (e) {
      errorMsg.set(String(e));
    }
    busy = false;
  }

  /** Everything that CHANGES the account happens in ewe-sync. */
  const openSync = () => api.openSyncApp().catch((e) => errorMsg.set(String(e)));

  const shapes = [
    { label: "Circle", value: "circle" },
    { label: "Rounded", value: "rounded" },
    { label: "Square", value: "square" }
  ];
  const fmtSync = (iso) => {
    if (!iso) return "never";
    try {
      return new Date(iso).toLocaleString();
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
  $: mailSource = mail?.source || (mail?.imapConfigured ? "imap" : google?.signedIn ? "gmail" : "");
  import Page from "./ui/Page.svelte";
  import Group from "./ui/Group.svelte";
  import Row from "./ui/Row.svelte";
  $: avatarShape = $prefs.avatarShape || "circle";
  $: avatarClass = avatarShape === "square" ? "ewe-avatar--square rounded-none" : avatarShape === "rounded" ? "ewe-avatar--square" : "";
</script>

<Page title="User" desc="Who you are on this computer, and the accounts it's signed in to.">
  <Group>
    <div class="ewe-row user-head">
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
          <button class="user-name" title="Edit your name"
            on:click={() => { nameEdit = info?.realName || ""; editingName = true; }}>
            {info?.realName || info?.user || "…"}
          </button>
        {/if}
        <div class="ewe-row__desc">{info ? `${info.user}@${info.host}` : ""}</div>
      </div>
      <div class="ewe-row__trail flex-col items-end">
        <button class="ewe-btn ewe-btn--secondary ewe-btn--sm" on:click={pickAvatar}>Change picture…</button>
        {#if cloud?.signedIn && cloud?.avatarPath}
          <button class="ewe-btn ewe-btn--ghost ewe-btn--sm" disabled={busy} on:click={useCloudPhoto}>Use account picture</button>
        {/if}
      </div>
    </div>
    <SelectRow
      label="Picture shape"
      sub="On the lock screen and in the shell."
      options={shapes}
      value={avatarShape}
      picked={(v) => setPrefs({ avatarShape: v })}
    />
    {#if info?.uptime}<KV k="Session" v={info.uptime} />{/if}
  </Group>

  <!-- Read-only by design (RFC-006). ewe-sync owns the account; this shows it. -->
  <Group title="Your account · Nextcloud">
    {#if cloud === null}
      <Row sub="The shell isn't running, and your account is managed through it." />
    {:else if !cloud.signedIn}
      <Row title="Not signed in" sub="Settings sync, your files as a folder, your calendar. You sign in from ewe-sync.">
        {#if syncApp}
          <button class="ewe-btn ewe-btn--primary" on:click={openSync}>Open ewe-sync</button>
        {/if}
      </Row>
    {:else}
      <div class="ewe-row">
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
      <KV k="Files" v={cloud.filesMounted ? `${cloud.filesPath || "~/Nextcloud"} (mounted)` : cloud.filesPath ? `${cloud.filesPath} (not mounted)` : "—"} />
      <KV k="Calendar" v={cloud.calState === "ok" ? `${cloud.eventCount} upcoming event${cloud.eventCount === 1 ? "" : "s"}` : cloud.calState || "—"} />
      {#if cloud.error}
        <Row><span class="text-warning">{cloud.error}</span></Row>
      {/if}
    {/if}
  </Group>

  <!-- Status only. Back up, sync, push and restore are ewe-sync's verbs. -->
  <Group title="Settings sync">
    {#if !cloud?.signedIn}
      <Row sub="Sign in from ewe-sync to keep this computer's settings, apps and look in your account, and bring them back on the next one." />
    {:else}
      <Row
        title="The one file"
        sub={cloud.syncState === "syncing"
          ? "Syncing…"
          : cloud.syncConflict
            ? `Another computer${cloud.remoteMachine ? ` (“${cloud.remoteMachine}”)` : ""} saved newer settings. Resolve it in ewe-sync.`
            : cloud.syncError
              ? cloud.syncError
              : !cloud.lastSync
                ? "Nothing is uploaded until you back up this computer in ewe-sync."
                : cloud.inSync
                  ? "Up to date."
                  : "Changed since the last sync."}
      />
      <KV k="Backup in your account"
        v={cloud.remoteMachine || cloud.remoteModified ? `Saved by “${cloud.remoteMachine || "another computer"}” · ${fmtSync(cloud.remoteModified)}` : "None yet"} />
      <KV k="This computer last synced"
        v={cloud.localSyncedAt ? fmtSync(cloud.localSyncedAt) + (cloud.inSync ? " · up to date" : "") : "Never"} />
      <KV k="Auto-sync" v={cloud.autoSync ? "On" : "Off"} />
      {#if syncApp}
        <Row sub="Backups, restoring, your computers and folder sync are all in the account app.">
          <button class="ewe-btn ewe-btn--secondary" on:click={openSync}>Open ewe-sync</button>
        </Row>
      {/if}
    {/if}
  </Group>

  <!-- What is connected, and the one preference that is this machine's. -->
  <Group title="Mail">
    <Row
      title={mailSource === "imap"
        ? mail?.imapUser || "IMAP account"
        : mailSource === "gmail"
          ? "Gmail"
          : "No mail account"}
      sub={mailSource === "imap"
        ? `${mail?.imapHost || ""}${mail?.state === "auth" ? " · The server rejected the password" : mail?.state === "offline" ? " · Offline" : mail?.unread ? ` · ${mail.unread} unread` : ""}`
        : mailSource === "gmail"
          ? "Through your Google client. Unread mail shows as a badge in Quick settings."
          : "The inbox that comes with your Nextcloud account, or any IMAP server. You add it in ewe-sync."}
    />
    {#if mail?.error && mailSource === "imap"}
      <Row><span class="text-warning">{mail.error}</span></Row>
    {/if}
    {#if mail && mailSource}
      <ToggleRow
        title="Notifications"
        sub="A notification for new mail while you're signed in."
        on={!!mail.notify}
        toggled={() => call("mail", "setNotify", mail.notify ? "false" : "true")}
      />
    {/if}
  </Group>

  <!-- Read-only, like the account above: connecting, disconnecting and the
       client-file guidance all live in ewe-sync → Google. -->
  <Group title="Google (optional)">
    <Row sub="For Gmail notifications and a Drive folder, never settings sync. ewe ships no Google client of its own: you bring your own OAuth client, and ewe-sync → Google explains where it goes and connects the account." />
    <KV k="Client file" v={gclient ? (gclient.valid ? "Found" : gclient.exists ? "Found, but not a desktop app client JSON" : "Missing") : google?.configured ? "Found" : "Missing"} />
    {#if google === null}
      <Row sub="The shell isn't running, and Google is managed through it." />
    {:else if !google.signedIn}
      <Row sub="Not connected." />
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
      <KV k="Gmail" v={google.mailState === "scope" ? "No mail permission. Reconnect in ewe-sync." : google.mailState === "ok" ? `${google.mailUnread || 0} unread` : google.mailState || "—"} />
      <KV k="Drive folder" v="~/Google Drive (mounted when you sign in)" />
    {/if}
    {#if syncApp}
      <Row sub="You connect and disconnect Google in the account app.">
        <button class="ewe-btn ewe-btn--secondary" on:click={openSync}>Open ewe-sync</button>
      </Row>
    {/if}
  </Group>
</Page>
