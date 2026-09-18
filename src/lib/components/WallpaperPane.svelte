<script>
  import { onMount } from "svelte";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import * as api from "../api.js";
  import { parseWallpapersConf, wallpapersConfText, isVideo, isGif } from "../hypr.js";
  import { errorMsg, flashApplied } from "../stores.js";
  import SelectRow from "./ui/SelectRow.svelte";
  import ToggleRow from "./ui/ToggleRow.svelte";

  let map = {}; // "*" or output name → file path
  let mode = "fill";
  let mute = true;
  let target = "*";
  let dir = "";
  let files = [];
  let outputs = [];
  let imgBackend = "";
  let videoBackend = "";
  // Thumbnails the asset protocol would not serve. The scope in
  // tauri.conf.json has to name every root a wallpaper can come from, and
  // `$HOME/**` does NOT cover a leading-dot directory — which is why
  // ~/.local/share/ewe (the deployed payload, where the shipped set lives) is
  // listed separately, and why $HOME/.face was already listed beside it.
  let broken = {};

  $: anyVideo = Object.values(map).some(isVideo);
  $: anyAnimated = Object.values(map).some((p) => isVideo(p) || isGif(p));
  $: backendLabel =
    imgBackend === "" && videoBackend === "" ? "none" : imgBackend + (videoBackend !== "" ? " + " + videoBackend : "");

  onMount(async () => {
    try {
      const conf = parseWallpapersConf(await api.readConfig("hypr/generated/wallpapers.conf"));
      map = conf.map;
      mode = conf.mode;
      mute = conf.mute;
    } catch {}
    try {
      outputs = (await api.monitors()).filter((m) => !m.disabled).map((m) => m.name);
    } catch {}
    try {
      const out = await api.wallpaperBackend();
      const m = out.match(/img=(\S+)\s+video=(\S+)/);
      imgBackend = m && m[1] !== "none" ? m[1] : "";
      videoBackend = m && m[2] !== "none" ? m[2] : "";
    } catch {}
    try {
      dir = await api.defaultWallpaperDir();
      files = await api.listWallpapers(dir);
    } catch {}
  });

  async function write() {
    await api.writeConfig("hypr/generated/wallpapers.conf", wallpapersConfText(mode, mute, map));
    try {
      const out = (await api.wallpaperReapply()).trim();
      if (out.indexOf("error:") >= 0)
        errorMsg.set(out.split("\n").filter((l) => l.indexOf("error:") >= 0)[0].replace(/^error:\s*/, ""));
      else flashApplied(out === "" ? "Wallpaper applied" : out.split("\n")[0].replace(/^note:\s*/, ""));
    } catch (e) {
      errorMsg.set(String(e));
    }
  }

  function assign(path) {
    if (isVideo(path) && videoBackend === "") {
      errorMsg.set("Video wallpapers need mpvpaper — install it with: sudo pacman -S mpvpaper");
      return;
    }
    if (isGif(path) && imgBackend === "swaybg")
      errorMsg.set("GIFs will be static with swaybg — install swww (sudo pacman -S awww) for animation");
    if (target === "*") map = { "*": path }; // "all displays" replaces per-monitor picks
    else map = { ...map, [target]: path };
    write();
  }

  async function browse() {
    const sel = await openDialog({
      multiple: false,
      filters: [
        { name: "Wallpapers", extensions: ["jpg", "jpeg", "png", "webp", "gif", "mp4", "webm", "mkv", "mov", "m4v"] }
      ]
    });
    if (typeof sel === "string") {
      assign(sel);
      const parent = sel.replace(/\/[^/]*$/, "");
      if (parent && parent !== dir) {
        dir = parent;
        try {
          files = await api.listWallpapers(dir);
        } catch {}
      }
    }
  }

  $: currentFor = (t) => map[t] || map["*"] || "";
  import Page from "./ui/Page.svelte";
  import Group from "./ui/Group.svelte";
  import Icon from "./ui/Icon.svelte";
  import Alert from "./ui/Alert.svelte";
</script>

<Page title="Wallpaper" desc="The picture or video behind your windows, for every display or one at a time.">
  <svelte:fragment slot="actions">
    <button class="ewe-btn ewe-btn--primary" on:click={browse}>Choose file…</button>
  </svelte:fragment>

  <Group>
    {#if outputs.length > 1}
      <SelectRow
        label="Set for"
        options={[{ label: "All displays", value: "*" }, ...outputs.map((o) => ({ label: o, value: o }))]}
        value={target}
        picked={(v) => (target = v)}
      />
    {/if}
    <SelectRow
      label="Fit"
      options={[
        { label: "Fill", value: "fill" },
        { label: "Fit", value: "fit" },
        { label: "Center", value: "center" },
        { label: "Tile", value: "tile" }
      ]}
      value={mode}
      picked={(v) => {
        mode = v;
        if (Object.keys(map).length) write();
      }}
    />
    <ToggleRow
      title="Mute video wallpapers"
      dim={!anyVideo}
      on={mute}
      toggled={() => {
        mute = !mute;
        write();
      }}
    />
  </Group>

  {#if files.length}
    <Group title={dir} well={false}>
      <div class="ewe-walls wall-grid" role="radiogroup" aria-label="Wallpapers in {dir}">
        {#each files as f (f)}
          {@const cur = currentFor(target) === f}
          {@const fname = f.split("/").pop()}
          <button
            class="ewe-wallitem"
            class:is-selected={cur}
            role="radio"
            aria-checked={cur}
            aria-label={fname}
            title={fname}
            on:click={() => assign(f)}
          >
            <span class="ewe-wallitem__img">
              {#if isVideo(f)}
                <span class="wall-fallback"><Icon name="film" size="xl" /></span>
              {:else if broken[f]}
                <!-- A thumbnail the asset protocol refused. WebKit's own
                     fallback is a bare "?" glyph, which says nothing and looks
                     like a corrupt file — name the thing instead, so a scope
                     miss is legible rather than mysterious. -->
                <span class="wall-fallback">
                  <span class="text-xs font-medium">{fname}</span>
                  <span class="text-xs text-dim">No preview</span>
                </span>
              {:else}
                <img
                  src={convertFileSrc(f)}
                  alt=""
                  loading="lazy"
                  on:error={() => (broken = { ...broken, [f]: true })}
                />
              {/if}
              {#if isVideo(f) || isGif(f)}
                <span class="ewe-badge ewe-badge--solid ewe-badge--accent wall-kind"><span class="ewe-badge__label">{isVideo(f) ? "Video" : "GIF"}</span></span>
              {/if}
              {#if cur}
                <span class="ewe-wallitem__check"><Icon name="check" /></span>
              {/if}
            </span>
          </button>
        {/each}
      </div>
    </Group>
  {:else if dir}
    <div class="ewe-list">
      <div class="ewe-empty">
        <span class="ewe-empty__icon"><Icon name="image" /></span>
        <div class="ewe-empty__title">No pictures or videos here</div>
        <div class="ewe-empty__desc">{dir} has nothing to show. Choose a file to use one from anywhere.</div>
      </div>
    </div>
  {/if}

  <p class="note">
    Applied now and again every time you sign in (wallpaper.sh); a newly connected display gets its
    wallpaper by itself. Pictures use {imgBackend || "no backend"}, GIFs animate with swww, and
    videos loop with mpvpaper. Backend: {backendLabel}.
  </p>
  {#if anyAnimated}
    <Alert tone="warning">Animated wallpapers keep the graphics card decoding all the time, which uses noticeably more battery.</Alert>
  {/if}
</Page>
