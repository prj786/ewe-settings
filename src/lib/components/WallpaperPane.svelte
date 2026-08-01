<script>
  import { onMount } from "svelte";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import * as api from "../api.js";
  import { parseWallpapersConf, wallpapersConfText, isVideo, isGif } from "../hypr.js";
  import { errorMsg, flashApplied } from "../stores.js";
  import Card from "./ui/Card.svelte";
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
</script>

<div class="mx-auto max-w-3xl space-y-6 p-5 sm:p-8">
  <div class="flex flex-wrap items-center justify-between gap-2">
    <h1 class="text-lg font-semibold">Wallpaper</h1>
    <button class="btn-primary !py-1.5 text-xs" on:click={browse}>Choose file…</button>
  </div>

  <Card>
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
  </Card>

  {#if files.length}
    <section>
      <div class="section-title">{dir}</div>
      <div class="grid grid-cols-[repeat(auto-fill,minmax(140px,1fr))] gap-2.5">
        {#each files as f (f)}
          {@const cur = currentFor(target) === f}
          <button
            class="group relative aspect-video overflow-hidden rounded-lg border transition-shadow hover:shadow-md
              {cur ? 'border-transparent ring-2' : 'border-zinc-200 dark:border-zinc-700/60'}"
            style={cur ? "--tw-ring-color: var(--accent)" : ""}
            on:click={() => assign(f)}
          >
            {#if isVideo(f)}
              <div class="flex h-full w-full items-center justify-center bg-zinc-200 text-2xl dark:bg-zinc-700">🎬</div>
            {:else}
              <img src={convertFileSrc(f)} alt="" loading="lazy" class="h-full w-full object-cover" />
            {/if}
            {#if isVideo(f) || isGif(f)}
              <span class="absolute bottom-1 left-1 rounded bg-black/60 px-1.5 py-0.5 text-[9px] font-semibold text-white">
                {isVideo(f) ? "▶ video" : "GIF"}
              </span>
            {/if}
            {#if cur}
              <span class="absolute right-1 top-1 rounded-full px-1.5 text-[11px] font-bold text-white" style="background: var(--accent)">✓</span>
            {/if}
          </button>
        {/each}
      </div>
    </section>
  {:else if dir}
    <p class="text-sm text-zinc-400">No images or videos in {dir} — use “Choose file…”.</p>
  {/if}

  <p class="text-xs text-zinc-400 dark:text-zinc-500">
    Applied live and restored at every login by wallpaper.sh; a newly plugged-in monitor gets its
    wallpaper automatically. Static images → {imgBackend || "no backend"}, GIFs animate via swww,
    video plays via mpvpaper (always looped). Backend: {backendLabel}.
  </p>
  {#if anyAnimated}
    <p class="text-xs text-amber-500">
      Animated wallpapers keep the GPU decoding continuously — expect measurable battery drain.
    </p>
  {/if}
</div>
