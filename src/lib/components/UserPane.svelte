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
  import Card from "./ui/Card.svelte";
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
</script>

<div class="mx-auto max-w-3xl space-y-6 p-5 sm:p-8">
  <h1 class="text-lg font-semibold">User</h1>

  <Card>
    <div class="flex items-center gap-4 px-4 py-4">
      {#if faceUrl}
        <img
          src={faceUrl}
          alt="Avatar"
          class="h-16 w-16 object-cover
            {($prefs.avatarShape || 'circle') === 'circle' ? 'rounded-full' : ($prefs.avatarShape || 'circle') === 'rounded' ? 'rounded-xl' : 'rounded-none'}"
        />
      {:else}
        <div class="flex h-16 w-16 items-center justify-center rounded-full text-2xl font-bold text-white" style="background: var(--accent)">
          {(info?.realName || info?.user || "?").slice(0, 1).toUpperCase()}
        </div>
      {/if}
      <div class="min-w-0 flex-1">
        {#if editingName}
          <form on:submit|preventDefault={saveName} class="flex gap-2">
            <!-- svelte-ignore a11y_autofocus -->
            <input class="input flex-1" bind:value={nameEdit} autofocus on:blur={saveName} />
          </form>
        {:else}
          <button class="text-left text-base font-semibold hover:underline" title="Edit name"
            on:click={() => { nameEdit = info?.realName || ""; editingName = true; }}>
            {info?.realName || info?.user || "…"}
          </button>
        {/if}
        <div class="truncate text-xs text-zinc-400">{info ? `${info.user}@${info.host}` : ""}</div>
      </div>
      <div class="flex flex-col gap-1.5">
        <button class="btn-ghost !py-1 text-xs" on:click={pickAvatar}>Change avatar…</button>
        {#if cloud?.signedIn && cloud?.avatarPath}
          <button class="btn-ghost !py-1 text-xs" disabled={busy} on:click={useCloudPhoto}>Use account photo</button>
        {/if}
      </div>
    </div>
    <SelectRow
      label="Avatar shape"
      sub="Used by the lock screen and the shell."
      options={shapes}
      value={$prefs.avatarShape || "circle"}
      picked={(v) => setPrefs({ avatarShape: v })}
    />
    {#if info?.uptime}<KV k="Session" v={info.uptime} />{/if}
  </Card>

  <!-- ── Your account · Nextcloud ─────────────────────────────────────────── -->
  <!-- Read-only by design (RFC-006). ewe-sync owns the account; this shows it. -->
  <section>
    <div class="section-title">Your account · Nextcloud</div>
    <Card>
      {#if cloud === null}
        <div class="px-4 py-3 text-sm text-zinc-400">
          The shell is not running — your account is managed through it.
        </div>
      {:else if !cloud.signedIn}
        <div class="flex items-center justify-between gap-3 px-4 py-3">
          <div class="min-w-0">
            <div class="text-sm font-medium">Not signed in</div>
            <div class="text-xs text-zinc-400">
              Settings sync, your files as a folder, your calendar. Signing in happens in ewe-sync.
            </div>
          </div>
          {#if syncApp}
            <button class="btn-primary !py-1.5 shrink-0 text-xs" on:click={openSync}>Open ewe-sync</button>
          {/if}
        </div>
      {:else}
        <div class="flex items-center gap-3 px-4 py-3">
          {#if cloudAvatar}
            <img src={cloudAvatar} alt="" class="h-10 w-10 rounded-full object-cover" />
          {:else}
            <div class="flex h-10 w-10 items-center justify-center rounded-full text-base font-bold text-white" style="background: var(--accent)">
              {(cloud.displayName || cloud.user || "?").slice(0, 1).toUpperCase()}
            </div>
          {/if}
          <div class="min-w-0 flex-1">
            <div class="truncate text-sm font-medium">{cloud.displayName || cloud.user}</div>
            <div class="truncate text-xs text-zinc-400">
              {cloud.email ? `${cloud.email} · ` : ""}{cloud.serverHost || cloud.server}
              {#if cloud.offline}<span class="text-amber-500"> · offline</span>{/if}
            </div>
          </div>
        </div>
        {#if cloud.quota && cloud.quota.total > 0}
          <div class="px-4 py-2.5">
            <div class="flex items-baseline justify-between text-xs">
              <span class="text-zinc-400">Storage</span>
              <span>{fmtBytes(cloud.quota.used)} of {fmtBytes(cloud.quota.total)}</span>
            </div>
            <div class="mt-1.5 h-1.5 w-full overflow-hidden rounded-full bg-zinc-800">
              <div class="h-full rounded-full" style="width: {quotaPct}%; background: var(--accent)"></div>
            </div>
          </div>
        {:else if cloud.quota}
          <KV k="Storage" v={`${fmtBytes(cloud.quota.used)} used`} />
        {/if}
        <KV k="Files" v={cloud.filesMounted ? `${cloud.filesPath || "~/Nextcloud"} (mounted)` : cloud.filesPath ? `${cloud.filesPath} (not mounted)` : "—"} />
        <KV k="Calendar" v={cloud.calState === "ok" ? `${cloud.eventCount} upcoming event${cloud.eventCount === 1 ? "" : "s"}` : cloud.calState || "—"} />
        {#if cloud.error}
          <div class="px-4 py-2.5 text-xs text-amber-500">{cloud.error}</div>
        {/if}
      {/if}
    </Card>
  </section>

  <!-- ── Settings sync ────────────────────────────────────────────────────── -->
  <!-- Status only. Back up, sync, push and restore are ewe-sync's verbs. -->
  <section>
    <div class="section-title">Settings sync</div>
    <Card>
      {#if !cloud?.signedIn}
        <div class="px-4 py-3 text-sm text-zinc-400">
          Sign in from ewe-sync to keep this machine's settings, app list and looks in your account —
          and to bring them back on the next one.
        </div>
      {:else}
        <div class="px-4 py-3">
          <div class="text-sm font-medium">The one file</div>
          <div class="text-xs text-zinc-400">
            {cloud.syncState === "syncing"
              ? "Syncing…"
              : cloud.syncConflict
                ? `Another machine${cloud.remoteMachine ? ` (“${cloud.remoteMachine}”)` : ""} saved newer settings — resolve it in ewe-sync.`
                : cloud.syncError
                  ? cloud.syncError
                  : !cloud.lastSync
                    ? "Nothing is uploaded until this machine is backed up in ewe-sync."
                    : cloud.inSync
                      ? "Up to date."
                      : "Changes since the last sync."}
          </div>
        </div>
        <KV k="Backup in your account"
          v={cloud.remoteMachine || cloud.remoteModified ? `saved by “${cloud.remoteMachine || "another machine"}” · ${fmtSync(cloud.remoteModified)}` : "none yet"} />
        <KV k="This machine last synced"
          v={cloud.localSyncedAt ? fmtSync(cloud.localSyncedAt) + (cloud.inSync ? " · up to date" : "") : "never"} />
        <KV k="Auto-sync" v={cloud.autoSync ? "on" : "off"} />
        {#if syncApp}
          <div class="flex items-center justify-between gap-3 px-4 py-3">
            <div class="text-xs text-zinc-400">
              Backing up, restoring, your machines and folder sync all live in the account app.
            </div>
            <button class="btn-primary !py-1 shrink-0 text-xs" on:click={openSync}>Manage in ewe-sync</button>
          </div>
        {/if}
      {/if}
    </Card>
  </section>

  <!-- ── Mail ─────────────────────────────────────────────────────────────── -->
  <!-- What is connected, and the one preference that is this machine's. -->
  <section>
    <div class="section-title">Mail</div>
    <Card>
      <div class="px-4 py-3">
        <div class="text-sm font-medium">
          {mailSource === "imap"
            ? mail?.imapUser || "IMAP account"
            : mailSource === "gmail"
              ? "Gmail"
              : "No mail account"}
        </div>
        <div class="truncate text-xs text-zinc-400">
          {mailSource === "imap"
            ? `${mail?.imapHost || ""}${mail?.state === "auth" ? " · the server rejected the password" : mail?.state === "offline" ? " · offline" : mail?.unread ? ` · ${mail.unread} unread` : ""}`
            : mailSource === "gmail"
              ? "Through your Google client · unread badge in the Control Center"
              : "The inbox your provider gives you with the Nextcloud account, or any IMAP server. Added in ewe-sync."}
        </div>
      </div>
      {#if mail?.error && mailSource === "imap"}
        <div class="px-4 py-2 text-xs text-amber-500">{mail.error}</div>
      {/if}
      {#if mail && mailSource}
        <ToggleRow
          title="Notifications"
          sub="A notification for new mail while the desktop is up."
          on={!!mail.notify}
          toggled={() => call("mail", "setNotify", mail.notify ? "false" : "true")}
        />
      {/if}
    </Card>
  </section>

  <!-- ── Google · optional ────────────────────────────────────────────────── -->
  <!-- Read-only, like the account above: connecting, disconnecting and the
       client-file guidance all live in ewe-sync → Google. -->
  <section>
    <div class="section-title">Google · optional</div>
    <Card>
      <div class="px-4 py-3 text-xs text-zinc-400">
        For Gmail notifications and a Drive folder — never settings sync. ewe ships no Google client
        of its own; you bring your own OAuth client. ewe-sync → Google explains where it goes and
        connects the account.
      </div>
      <KV k="Client file" v={gclient ? (gclient.valid ? "found" : gclient.exists ? "found, but not a Desktop-app client JSON" : "missing") : google?.configured ? "found" : "missing"} />
      {#if google === null}
        <div class="px-4 py-3 text-sm text-zinc-400">The shell is not running — Google is managed through it.</div>
      {:else if !google.signedIn}
        <div class="px-4 py-2.5 text-xs text-zinc-400">Not connected.</div>
      {:else}
        <div class="flex items-center gap-3 px-4 py-3">
          {#if google.profile?.picture}
            <img src={google.profile.picture} alt="" class="h-9 w-9 rounded-full" />
          {/if}
          <div class="min-w-0 flex-1">
            <div class="truncate text-sm font-medium">{google.profile?.name || "Google"}</div>
            <div class="truncate text-xs text-zinc-400">{google.profile?.email || ""}</div>
          </div>
        </div>
        <KV k="Gmail" v={google.mailState === "scope" ? "no mail permission — reconnect in ewe-sync" : google.mailState === "ok" ? `${google.mailUnread || 0} unread` : google.mailState || "—"} />
        <KV k="Drive folder" v="~/Google Drive (mounted at sign-in)" />
      {/if}
      {#if syncApp}
        <div class="flex items-center justify-between gap-3 px-4 py-3">
          <div class="text-xs text-zinc-400">Connecting and disconnecting happen in the account app.</div>
          <button class="btn-primary !py-1 shrink-0 text-xs" on:click={openSync}>Manage in ewe-sync</button>
        </div>
      {/if}
    </Card>
  </section>
</div>
