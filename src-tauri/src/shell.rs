//! The bridge to the running shell.
//!
//! Settings is a separate process from the Quickshell shell, so it cannot set
//! `Globals.*` the way the old in-shell Settings.qml did. Instead it follows the
//! contract the shell already has with itself:
//!
//!   1. write the SAME json files the shell reads (`user-theme.json`, …)
//!   2. poke the shell over `qs ipc` so it re-reads them
//!
//! That keeps one source of truth, survives a shell restart for free, and means
//! Google Drive sync needs no changes at all — those files are exactly what
//! settings-bundle.py already backs up.
//!
//! Writes are atomic (temp + rename). A settings app that can corrupt the
//! shell's config by being killed mid-write is worse than no settings app.

use std::path::PathBuf;

use serde_json::Value;
use tokio::process::Command;

use crate::util::estr;

fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/".into()))
}

fn qs_dir() -> PathBuf {
    home().join(".config/quickshell")
}

/// Where ewe is checked out / installed. The repo is the source of truth for
/// the version, and for updates. Overridable because the locations below are
/// conventions, not guarantees — and guessing with `find` across a home
/// directory is worse than simply not offering the feature. get.sh installs to
/// ~/.local/share/ewe; a developer clone is ~/hypr-shell (the pre-rename dir
/// name, kept on disk).
fn repo_dir() -> PathBuf {
    for var in ["EWE_REPO", "HYPR_SHELL_REPO"] {
        if let Ok(p) = std::env::var(var) {
            if !p.trim().is_empty() {
                return PathBuf::from(p);
            }
        }
    }
    let installed = home().join(".local/share/ewe");
    if installed.join("VERSION").is_file() {
        return installed;
    }
    home().join("hypr-shell")
}

// ── reading ─────────────────────────────────────────────────────────────────
#[tauri::command]
pub async fn read_prefs() -> Result<Value, String> {
    let p = qs_dir().join("user-theme.json");
    match std::fs::read_to_string(&p) {
        // A missing file is not an error: it means "nothing customised yet", and
        // the shell itself treats it that way.
        Err(_) => Ok(serde_json::json!({})),
        Ok(s) => serde_json::from_str(&s).map_err(estr),
    }
}

/// The version shown in the footer. It is ewe's, not this app's: Settings
/// is a part of the desktop rather than a product with its own release cycle,
/// and showing a second number would only invite "which one is the real one".
#[tauri::command]
pub async fn shell_version() -> Result<String, String> {
    let v = std::fs::read_to_string(repo_dir().join("VERSION"))
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    if !v.is_empty() {
        return Ok(v);
    }
    // Fall back to what the running shell reports, so a user who installed
    // without keeping the checkout still sees a version rather than a blank.
    let out = Command::new("qs")
        .args(["ipc", "call", "settings", "version"])
        .output()
        .await
        .map_err(estr)?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Ok(if s.is_empty() { "unknown".into() } else { s })
}

// ── writing ─────────────────────────────────────────────────────────────────
/// Merge a patch into user-theme.json and tell the shell to re-read it.
///
/// Merge rather than replace: this app does not own every key in that file, and
/// a naive overwrite would silently drop settings written by the shell or by a
/// newer version of Settings than the one running.
#[tauri::command]
pub async fn write_prefs(patch: Value) -> Result<Value, String> {
    let path = qs_dir().join("user-theme.json");
    let mut cur = match std::fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str::<Value>(&s).unwrap_or_else(|_| serde_json::json!({})),
        Err(_) => serde_json::json!({}),
    };
    let obj = cur
        .as_object_mut()
        .ok_or("user-theme.json is not an object")?;
    if let Some(p) = patch.as_object() {
        for (k, v) in p {
            obj.insert(k.clone(), v.clone());
        }
    }

    // RFC-001: persist the merged object through ewe-conf (`absorb
    // user-theme` owns the key mapping and regenerates user-theme.json).
    // Fallback to the direct write only when ewe-conf is absent (pre-0.9 DE).
    let absorbed = if let Some(bin) = crate::backend::ewe_conf_bin_pub() {
        tokio::process::Command::new(bin)
            .args(["absorb", "--no-hooks", "user-theme"])
            .arg(cur.to_string())
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false)
    } else {
        false
    };
    if !absorbed {
        std::fs::create_dir_all(qs_dir()).map_err(estr)?;
        let tmp = path.with_extension("json.tmp");
        // trailing \n matches the shell's own writer, so alternating writers
        // never churn the file by a single byte
        let mut body = serde_json::to_vec_pretty(&cur).map_err(estr)?;
        body.push(b'\n');
        std::fs::write(&tmp, body).map_err(estr)?;
        // rename is atomic: the shell either sees the old file or the new one.
        std::fs::rename(&tmp, &path).map_err(estr)?;
    }

    reload_shell().await?;
    poke_sync();
    Ok(cur)
}

/// The DE shell's qs pid — targeting it makes `qs ipc` unambiguous even when
/// another qs instance exists (nested tests). The unit is `ewe.service` since
/// the rename; `hypr-shell.service` is probed second for pre-rename installs.
pub async fn qs_call(args: &[&str]) -> std::io::Result<std::process::Output> {
    let mut pid: Option<String> = None;
    for unit in ["ewe.service", "hypr-shell.service"] {
        pid = Command::new("systemctl")
            .args(["--user", "show", "-p", "MainPID", "--value", unit])
            .output()
            .await
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|p| !p.is_empty() && p != "0");
        if pid.is_some() {
            break;
        }
    }
    let mut cmd = Command::new("qs");
    cmd.arg("ipc");
    if let Some(p) = &pid {
        cmd.args(["--pid", p]);
    }
    cmd.arg("call").args(args);
    cmd.output().await
}

/// Fire-and-forget "user state changed": the shell debounces the pokes and
/// pushes the sync bundle to Drive when things go quiet. Never awaited — a
/// save must not wait on a network round-trip.
pub fn poke_sync() {
    tokio::spawn(async {
        let _ = qs_call(&["google", "syncSoon"]).await;
    });
}

/// Ask the running shell to re-read its user state. Not an error when it fails:
/// the shell may simply not be running (Settings launched from a TTY), and the
/// write above is still valid — it will be picked up at next start.
pub async fn reload_shell() -> Result<(), String> {
    let _ = qs_call(&["settings", "reload"]).await;
    Ok(())
}

/// Exposed to the frontend for panes that write files other than
/// user-theme.json (displays, input, startup): after the write, the running
/// shell re-reads its state so hotplug/re-assert logic never acts on stale
/// in-memory copies.
#[tauri::command]
pub async fn poke_shell() -> Result<(), String> {
    reload_shell().await
}

#[tauri::command]
pub async fn shell_running() -> Result<bool, String> {
    let out = qs_call(&["settings", "ping"]).await;
    Ok(matches!(out, Ok(o) if o.status.success()))
}
