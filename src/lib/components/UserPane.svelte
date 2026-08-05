<script>
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
  let google = null; // parsed `qs ipc call google status` (null = shell absent)
  let faceVersion = 0;
  let nameEdit = "";
  let editingName = false;
  let gBusy = false;
  let timer;

  const home = () => info?.user ? `/home/${info.user}` : "";
  $: faceUrl = info?.hasFace ? convertFileSrc(`${home()}/.face`) + `?v=${faceVersion}` : "";

  async function refresh() {
    try {
      info = await api.userInfo();
    } catch (e) {
      errorMsg.set(String(e));
    }
    try {
      const raw = (await api.qsIpc("google", "status")).trim();
      google = raw ? JSON.parse(raw) : null;
    } catch {
      google = null; // shell not running — the account card says so
    }
  }
  onMount(() => {
    refresh();
    timer = setInterval(async () => {
      // keep the Google card live while signing in / syncing
      if (google && (google.busy || google.syncState === "syncing")) await refresh();
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

  async function useGooglePhoto() {
    if (!google?.profile?.picture) return;
    let url = String(google.profile.picture);
    url = /=s\d+(-c)?$/.test(url) ? url.replace(/=s\d+(-c)?$/, "=s512-c") : url;
    try {
      const warn = await api.avatarFromUrl(url);
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

  async function g(verb, arg) {
    gBusy = true;
    try {
      await api.qsIpc("google", verb, arg);
      setTimeout(refresh, 800);
    } catch (e) {
      errorMsg.set(String(e));
    }
    gBusy = false;
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
        {#if google?.signedIn && google?.profile?.picture}
          <button class="btn-ghost !py-1 text-xs" disabled={gBusy} on:click={useGooglePhoto}>Use Google photo</button>
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

  <section>
    <div class="section-title">Google account</div>
    <Card>
      {#if google === null}
        <div class="px-4 py-3 text-sm text-zinc-400">
          The shell is not running — the Google account is managed through it.
        </div>
      {:else if !google.configured}
        <div class="px-4 py-3 text-sm text-zinc-400">
          Not configured — add a client ID to google-oauth.json (see the ewe README).
        </div>
      {:else if !google.signedIn}
        <div class="flex items-center justify-between gap-3 px-4 py-3">
          <div>
            <div class="text-sm font-medium">Sign in with Google</div>
            <div class="text-xs text-zinc-400">
              {google.busy === "signin" ? "Waiting for the browser…" : "Calendar, Gmail badge and settings sync."}
            </div>
          </div>
          <button class="btn-primary !py-1.5 text-xs" disabled={gBusy || google.busy === "signin"} on:click={() => g("signIn")}>
            Sign in
          </button>
        </div>
        {#if google.error}
          <div class="px-4 py-2.5 text-xs text-amber-500">{google.error}</div>
        {/if}
        {#if !google.keyringOk}
          <div class="px-4 py-2.5 text-xs text-amber-500">
            The keyring (gnome-keyring / secret-tool) is unavailable — sign-in cannot store its token.
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
          <button class="btn-ghost !py-1 text-xs" disabled={gBusy} on:click={() => g("signOut")}>Sign out</button>
        </div>
        <div class="flex items-center justify-between gap-3 px-4 py-3">
          <div>
            <div class="text-sm font-medium">Settings sync</div>
            <div class="text-xs text-zinc-400">
              {google.syncState === "syncing"
                ? "Syncing…"
                : google.syncError
                  ? google.syncError
                  : `Last sync: ${fmtSync(google.lastSync)}`}
            </div>
          </div>
          <button class="btn-primary !py-1 text-xs" disabled={gBusy || google.syncState === "syncing"} on:click={() => g("syncNow")}>
            Sync now
          </button>
        </div>
        <ToggleRow
          title="Auto-sync"
          sub="Push a settings bundle to Drive shortly after Settings closes, when something changed."
          on={!!google.autoSync}
          toggled={() => g("setAutoSync", google.autoSync ? "false" : "true")}
        />
        <div class="px-4 py-2.5 text-xs text-zinc-400">
          Restoring a backup (including the package list) runs in the shell after sign-in on a fresh
          install; tokens stay in the system keyring and never pass through this app.
        </div>
      {/if}
    </Card>
  </section>
</div>
