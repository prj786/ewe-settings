<script>
  // User — who you are on this machine, and your account in the cloud.
  //
  // RFC-005: the account is a Nextcloud account (the shell's Cloud singleton,
  // IPC target "cloud"): identity, the sync of the one file, restore. Mail is
  // any IMAP inbox (ewe-mail) or Gmail. Google is an OPTIONAL extra for mail
  // and the Drive folder and only with the user's own client file — this app
  // never touches a token; every verb goes through the shell over IPC.
  // RFC-006: the account card becomes a launcher for Flock later; keep it flat.
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
  import KeyringNotes from "./KeyringNotes.svelte";

  let info = null;
  let cloud = null; // `qs ipc call cloud status` (null = shell absent)
  let google = null; // `qs ipc call google status`
  let mail = null; // `qs ipc call mail status`
  let gclient = null; // google_client_info
  let faceVersion = 0;
  let nameEdit = "";
  let editingName = false;
  let busy = false; // an IPC verb is in flight
  let timer;
  let online = true; // NetworkManager connectivity == full
  let server = ""; // the Nextcloud server field
  let avatarVersion = 0;

  // mail form
  let mailOpen = false;
  let mHost = "";
  let mPort = 993;
  let mUser = "";
  let mPass = "";
  let mStarttls = false;
  let mBusy = false;
  let mError = "";

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
    if (cloud && !server) server = cloud.lastServer || "";
    if (cloud?.signedIn && !wasSigned) avatarVersion++;
    try {
      gclient = await api.googleClientInfo();
    } catch {
      gclient = null;
    }
    try {
      online = (await api.netConnectivity()) === "full";
    } catch {
      online = true; // no nmcli: never block the button on a probe we cannot run
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
  const c = (verb, arg) => call("cloud", verb, arg);
  const g = (verb, arg) => call("google", verb, arg);

  function signInCloud() {
    let s = server.trim();
    if (!s) return;
    if (!/^https?:\/\//.test(s)) s = "https://" + s;
    s = s.replace(/\/+$/, "");
    if (!/^https:\/\/[A-Za-z0-9.\-_~%:/]+$/.test(s)) {
      errorMsg.set("The server must be an https:// address.");
      return;
    }
    server = s;
    c("signIn", s);
  }

  async function addMail() {
    mError = "";
    if (!mHost.trim() || !mUser.trim() || !mPass) {
      mError = "Server, user and password are all needed.";
      return;
    }
    mBusy = true;
    try {
      const r = await api.mailLogin(mHost.trim(), Number(mPort) || 993, mUser.trim(), mPass, mStarttls);
      if (r?.ok) {
        mPass = "";
        mailOpen = false;
        flashApplied("Mail account added");
        try {
          await api.qsIpc("mail", "refresh");
        } catch {
          /* shell absent: the badge picks it up at the next login */
        }
        setTimeout(refresh, 800);
      } else {
        mError = r?.message || r?.error || "Could not sign in to the mail server.";
      }
    } catch (e) {
      mError = String(e);
    }
    mBusy = false;
  }

  async function removeMail() {
    mBusy = true;
    try {
      const r = await api.mailLogout();
      if (r && r.ok === false) errorMsg.set(r.message || r.error);
      try {
        await api.qsIpc("mail", "refresh");
      } catch {
        /* shell absent */
      }
      setTimeout(refresh, 800);
    } catch (e) {
      errorMsg.set(String(e));
    }
    mBusy = false;
  }

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
  $: cKeyring = cloud?.keyringState || (cloud && cloud.keyringOk === false ? "unavailable" : "ok");
  $: cTrouble = !!cloud?.keyringTrouble || cKeyring === "locked";
  $: neverSynced = !cloud?.lastSync;
  $: quotaPct = cloud?.quota?.total > 0 ? Math.min(100, Math.round((cloud.quota.used / cloud.quota.total) * 100)) : null;
  $: gKeyring = google?.keyringState || (google && google.keyringOk === false ? "unavailable" : "ok");
  $: gTrouble = !!google?.keyringTrouble || gKeyring === "locked";
  $: clientPath = gclient?.path || google?.clientPath || "~/.config/ewe/oauth-client.json";
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
  <section>
    <div class="section-title">Your account · Nextcloud</div>
    <Card>
      {#if cloud === null}
        <div class="px-4 py-3 text-sm text-zinc-400">
          The shell is not running — your account is managed through it.
        </div>
      {:else if !cloud.signedIn}
        <div class="px-4 py-3">
          <div class="text-sm font-medium">Sign in to your Nextcloud</div>
          <div class="text-xs text-zinc-400">
            {cloud.busy === "signin"
              ? "Waiting for the browser — sign in on your server's page and allow “ewe”."
              : !online
                ? "You are offline — sign-in needs a connection. Join a network in Network first."
                : "Settings sync, your files as a folder, your calendar. Your own server, or a hosted account."}
          </div>
          <form class="mt-3 flex gap-2" on:submit|preventDefault={signInCloud}>
            <input class="input flex-1" placeholder="https://cloud.example.org" bind:value={server}
              disabled={busy || cloud.busy === "signin"} />
            {#if cloud.busy === "signin"}
              <button type="button" class="btn-ghost !py-1.5 text-xs" disabled={busy} on:click={() => c("cancelSignIn")}>Cancel</button>
            {:else}
              <button type="submit" class="btn-primary !py-1.5 text-xs"
                disabled={busy || !online || !server.trim() || cKeyring === "unavailable"}>
                Sign in
              </button>
            {/if}
          </form>
        </div>
        {#if cloud.reason === "revoked"}
          <div class="px-4 py-2.5 text-xs text-amber-500">
            The server no longer accepts this machine's app password (it was revoked in Security → Devices). Sign in again.
          </div>
        {/if}
        {#if google?.legacyGoogleSync}
          <div class="px-4 py-2.5 text-xs text-zinc-400">
            Settings sync now uses a Nextcloud account — sign in to keep your backups going. Your
            previous Google backup stays untouched on Drive.
          </div>
        {/if}
        <KeyringNotes state={cKeyring} trouble={cTrouble} resetDone={!!cloud.keyringResetDone}
          busy={cloud.busy === "signin"} {online} disabled={busy}
          onReset={() => c("keyringReset")} onLogOut={() => c("logOut")} />
        {#if cloud.error}
          <div class="px-4 py-2.5 text-xs text-amber-500">{cloud.error}</div>
        {/if}
        {#if cloud.loginUrl && cloud.busy === "signin"}
          <div class="flex items-center gap-4 px-4 py-2.5 text-xs">
            <button class="btn-ghost !py-1 text-xs" disabled={busy} on:click={() => c("openLoginUrl")}>Open the sign-in page</button>
            <button class="btn-ghost !py-1 text-xs" disabled={busy} on:click={() => c("copyLoginUrl")}>Copy the link</button>
          </div>
        {/if}
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
          <button class="btn-ghost !py-1 text-xs" disabled={busy} on:click={() => c("signOut")}>Sign out</button>
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
        <KeyringNotes state={cKeyring} trouble={cTrouble} resetDone={!!cloud.keyringResetDone}
          busy={false} {online} disabled={busy}
          onReset={() => c("keyringReset")} onLogOut={() => c("logOut")} />
        {#if cloud.error}
          <div class="px-4 py-2.5 text-xs text-amber-500">{cloud.error}</div>
        {/if}
      {/if}
    </Card>
  </section>

  <!-- ── Settings sync ────────────────────────────────────────────────────── -->
  <section>
    <div class="section-title">Settings sync</div>
    <Card>
      {#if !cloud?.signedIn}
        <div class="px-4 py-3 text-sm text-zinc-400">
          Sign in to your Nextcloud to keep this machine's settings, app list and looks in your account —
          and to bring them back on the next one.
        </div>
      {:else}
        <div class="flex items-center justify-between gap-3 px-4 py-3">
          <div>
            <div class="text-sm font-medium">The one file</div>
            <div class="text-xs text-zinc-400">
              {cloud.syncState === "syncing"
                ? "Syncing…"
                : cloud.syncConflict
                  ? `Another machine${cloud.remoteMachine ? ` (“${cloud.remoteMachine}”)` : ""} saved newer settings — restore it first, or push anyway.`
                  : cloud.syncError
                    ? cloud.syncError
                    : neverSynced
                      ? "Nothing is uploaded until you back this machine up."
                      : cloud.inSync
                        ? "Up to date."
                        : "Changes since the last sync."}
            </div>
          </div>
          <div class="flex shrink-0 gap-2">
            {#if cloud.syncConflict}
              <button class="btn-ghost !py-1 text-xs" disabled={busy || cloud.syncState === "syncing"} on:click={() => c("pushForce")}>
                Push anyway
              </button>
            {/if}
            <button class="btn-primary !py-1 text-xs" disabled={busy || cloud.syncState === "syncing" || cloud.offline}
              on:click={() => c(neverSynced ? "backUpNow" : "syncNow")}>
              {neverSynced ? "Back up this machine" : "Sync now"}
            </button>
          </div>
        </div>
        <KV k="Backup in your account"
          v={cloud.remoteMachine || cloud.remoteModified ? `saved by “${cloud.remoteMachine || "another machine"}” · ${fmtSync(cloud.remoteModified)}` : "none yet"} />
        <KV k="This machine last synced"
          v={cloud.localSyncedAt ? fmtSync(cloud.localSyncedAt) + (cloud.inSync ? " · up to date" : "") : "never — nothing is uploaded until you back it up"} />
        <ToggleRow
          title="Auto-sync"
          sub="Push the one file to your account shortly after it changes — Settings, Komble, anything."
          on={!!cloud.autoSync}
          toggled={() => c("setAutoSync", cloud.autoSync ? "false" : "true")}
        />
        {#if cloud.pendingRestore}
          <div class="px-4 py-3">
            <div class="text-sm font-medium">Restore this backup?</div>
            <div class="mt-1 text-xs text-zinc-400">
              {cloud.restoreSummary || "Your desktop, looks, app list and places from the backup replace this machine's."}
              This machine's current file is kept as a .bak next to it.
            </div>
            <div class="mt-3 flex gap-2">
              <button class="btn-primary !py-1 text-xs" disabled={busy} on:click={() => c("applyRestore")}>Restore</button>
              <button class="btn-ghost !py-1 text-xs" disabled={busy} on:click={() => c("cancelRestore")}>Cancel</button>
            </div>
          </div>
        {:else if cloud.remoteMachine || cloud.remoteModified}
          <div class="flex items-center justify-between gap-3 px-4 py-3">
            <div class="text-xs text-zinc-400">
              Bring the backup{cloud.remoteMachine ? ` from “${cloud.remoteMachine}”` : ""} onto this machine. Apps
              it lists appear in Komble → For you.
            </div>
            <button class="btn-ghost !py-1 text-xs" disabled={busy || cloud.syncState === "syncing" || cloud.offline}
              on:click={() => c("requestRestore")}>Restore…</button>
          </div>
        {/if}
      {/if}
    </Card>
  </section>

  <!-- ── Mail ─────────────────────────────────────────────────────────────── -->
  <section>
    <div class="section-title">Mail</div>
    <Card>
      <div class="flex items-center justify-between gap-3 px-4 py-3">
        <div class="min-w-0">
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
                : "The inbox your provider gives you with the Nextcloud account, or any IMAP server."}
          </div>
        </div>
        <div class="flex shrink-0 gap-2">
          {#if mailSource === "imap"}
            <button class="btn-ghost !py-1 text-xs" disabled={mBusy} on:click={removeMail}>Remove</button>
          {/if}
          <button class="btn-ghost !py-1 text-xs" disabled={mBusy} on:click={() => (mailOpen = !mailOpen)}>
            {mailOpen ? "Close" : mailSource === "imap" ? "Change…" : "Add mail account…"}
          </button>
        </div>
      </div>
      {#if mail?.error && mailSource === "imap"}
        <div class="px-4 py-2 text-xs text-amber-500">{mail.error}</div>
      {/if}
      {#if mailOpen}
        <form class="space-y-2 px-4 py-3" on:submit|preventDefault={addMail}>
          <div class="grid grid-cols-3 gap-2">
            <input class="input col-span-2" placeholder="imap.example.org" bind:value={mHost} disabled={mBusy} />
            <input class="input" type="number" min="1" max="65535" placeholder="993" bind:value={mPort} disabled={mBusy} />
          </div>
          <input class="input w-full" placeholder="you@example.org" bind:value={mUser} disabled={mBusy} autocomplete="username" />
          <input class="input w-full" type="password" placeholder="Password" bind:value={mPass} disabled={mBusy} autocomplete="current-password" />
          <label class="flex items-center gap-2 text-xs text-zinc-400">
            <input type="checkbox" bind:checked={mStarttls} disabled={mBusy} />
            STARTTLS (port 143 servers) instead of TLS
          </label>
          {#if mError}<div class="text-xs text-amber-500">{mError}</div>{/if}
          <div class="flex items-center justify-between gap-3">
            <div class="text-xs text-zinc-400">The password goes into the system keyring; only the server and user are recorded in ewe.conf.</div>
            <button type="submit" class="btn-primary !py-1 text-xs" disabled={mBusy}>{mBusy ? "Signing in…" : "Sign in"}</button>
          </div>
        </form>
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
  <section>
    <div class="section-title">Google · optional</div>
    <Card>
      <div class="px-4 py-3 text-xs text-zinc-400">
        For Gmail notifications and a Drive folder. ewe ships no Google client of its own — bring
        your own OAuth client (a Desktop-app client from the Google Cloud console) and drop it at
        <span class="font-mono">{clientPath}</span>.
      </div>
      <KV k="Client file" v={gclient ? (gclient.valid ? "found" : gclient.exists ? "found, but not a Desktop-app client JSON" : "missing") : google?.configured ? "found" : "missing"} />
      {#if google === null}
        <div class="px-4 py-3 text-sm text-zinc-400">The shell is not running — Google is managed through it.</div>
      {:else if !google.configured}
        <div class="px-4 py-2.5 text-xs text-zinc-400">
          Nothing to connect until the client file is there. Details: docs/GOOGLE-CLIENT.md in the ewe repo.
        </div>
      {:else if !google.signedIn}
        <div class="flex items-center justify-between gap-3 px-4 py-3">
          <div class="text-xs text-zinc-400">
            {google.busy === "signin"
              ? "Waiting for the browser…"
              : !online
                ? "You are offline — connecting needs a connection."
                : "Connect your Google account with your own client."}
          </div>
          {#if google.busy === "signin"}
            <button class="btn-ghost !py-1 text-xs" disabled={busy} on:click={() => g("cancelSignIn")}>Cancel</button>
          {:else}
            <button class="btn-primary !py-1 text-xs" disabled={busy || !online || gKeyring === "unavailable"} on:click={() => g("signIn")}>Connect</button>
          {/if}
        </div>
        <KeyringNotes state={gKeyring} trouble={gTrouble} resetDone={!!google.keyringResetDone}
          busy={google.busy === "signin"} {online} disabled={busy}
          onReset={() => g("keyringReset")} onLogOut={() => g("logOut")} />
        {#if google.error}
          <div class="px-4 py-2.5 text-xs text-amber-500">{google.error}</div>
        {/if}
        {#if google.consentUrl}
          <div class="flex items-center gap-4 px-4 py-2.5 text-xs">
            <button class="btn-ghost !py-1 text-xs" disabled={busy} on:click={() => g("openConsentUrl")}>Open the sign-in page</button>
            <button class="btn-ghost !py-1 text-xs" disabled={busy} on:click={() => g("copyConsentUrl")}>Copy the link</button>
          </div>
        {/if}
      {:else}
        <div class="flex items-center gap-3 px-4 py-3">
          {#if google.profile?.picture}
            <img src={google.profile.picture} alt="" class="h-9 w-9 rounded-full" />
          {/if}
          <div class="min-w-0 flex-1">
            <div class="truncate text-sm font-medium">{google.profile?.name || "Google"}</div>
            <div class="truncate text-xs text-zinc-400">{google.profile?.email || ""}</div>
          </div>
          <button class="btn-ghost !py-1 text-xs" disabled={busy} on:click={() => g("signOut")}>Disconnect</button>
        </div>
        <KV k="Gmail" v={google.mailState === "scope" ? "no mail permission — disconnect and connect again" : google.mailState === "ok" ? `${google.mailUnread || 0} unread` : google.mailState || "—"} />
        <KV k="Drive folder" v="~/Google Drive (mounted at sign-in)" />
        <KeyringNotes state={gKeyring} trouble={gTrouble} resetDone={!!google.keyringResetDone}
          busy={false} {online} disabled={busy}
          onReset={() => g("keyringReset")} onLogOut={() => g("logOut")} />
      {/if}
    </Card>
  </section>
</div>
