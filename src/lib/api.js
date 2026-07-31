import { invoke } from "@tauri-apps/api/core";

// Settings never mutates shell state directly — it writes the json the shell
// reads and asks it to reload. See src-tauri/src/shell.rs for why.
export const readPrefs = () => invoke("read_prefs");
export const writePrefs = (patch) => invoke("write_prefs", { patch });
export const shellVersion = () => invoke("shell_version");
export const shellRunning = () => invoke("shell_running");
