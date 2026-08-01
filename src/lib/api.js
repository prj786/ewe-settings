import { invoke } from "@tauri-apps/api/core";

// Settings never mutates shell state directly — it writes the json/lua/conf
// files the shell reads and asks it to reload. See src-tauri/src/shell.rs.
export const readPrefs = () => invoke("read_prefs");
export const writePrefs = (patch) => invoke("write_prefs", { patch });
export const shellVersion = () => invoke("shell_version");
export const shellRunning = () => invoke("shell_running");
export const pokeShell = () => invoke("poke_shell");

// config files (allowlisted, relative to ~/.config; atomic writes)
export const readConfig = (rel) => invoke("read_config", { rel });
export const writeConfig = (rel, content) => invoke("write_config", { rel, content });
export const removeConfig = (rel) => invoke("remove_config", { rel });

// hyprctl
export const hyprctl = (args) => invoke("hyprctl_query", { args });
export const monitors = async () => JSON.parse(await hyprctl(["monitors", "all", "-j"]));
export const devices = async () => JSON.parse(await hyprctl(["devices", "-j"]));
export const runEvals = (stmts) => invoke("run_evals", { stmts });
export const dpmsOn = () => invoke("dpms_on");
export const reloadHyprland = () => invoke("hyprctl_reload");

// wallpaper
export const wallpaperBackend = () => invoke("wallpaper_backend");
export const wallpaperReapply = () => invoke("wallpaper_reapply");
export const defaultWallpaperDir = () => invoke("default_wallpaper_dir");
export const listWallpapers = (dir) => invoke("list_wallpapers", { dir });

// screensaver / input daemons
export const restartHypridle = () => invoke("restart_hypridle");
export const perWindowKb = (action) => invoke("per_window_kb", { action });

// theming
export const applyColorscheme = (scheme, accent) =>
  invoke("apply_colorscheme", { scheme, accent });

// startup / default apps
export const desktopApps = () => invoke("desktop_apps");
export const mimeDefault = (mime) => invoke("mime_default", { mime });
export const mimeApps = (mime) => invoke("mime_apps", { mime });
export const setMimeDefault = (desktopId, mimes, setBrowser) =>
  invoke("set_mime_default", { desktopId, mimes, setBrowser });

// shell ipc (allowlisted verbs: google account, saver preview)
export const qsIpc = (target, func, arg) => invoke("qs_ipc", { target, func, arg });

// networking
export const netStatus = () => invoke("net_status");
export const wifiSet = (on) => invoke("wifi_set", { on });
export const wifiConnect = (ssid, password) => invoke("wifi_connect", { ssid, password });
export const connectionSet = (name, up) => invoke("connection_set", { name, up });

// user account
export const saveAvatar = (pngBase64) => invoke("save_avatar", { pngBase64 });
export const avatarFromUrl = (url) => invoke("avatar_from_url", { url });
export const setRealName = (name) => invoke("set_real_name", { name });
export const userInfo = () => invoke("user_info");

// system / power
export const diagnostics = () => invoke("diagnostics");
export const powerInfo = () => invoke("power_info");
export const setPowerProfile = (profile) => invoke("set_power_profile", { profile });
export const setChargeLimit = (pct) => invoke("set_charge_limit", { pct });
export const setKbdBacklight = (value) => invoke("set_kbd_backlight", { value });
