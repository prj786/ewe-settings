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

/// Where hypr-shell is checked out. The repo is the source of truth for the
/// version, and for updates. Overridable because ~/hypr-shell is a convention,
/// not a guarantee — and guessing with `find` across a home directory is worse
/// than simply not offering the feature.
fn repo_dir() -> PathBuf {
    match std::env::var("HYPR_SHELL_REPO") {
        Ok(p) if !p.trim().is_empty() => PathBuf::from(p),
        _ => home().join("hypr-shell"),
    }
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

/// The version shown in the footer. It is hypr-shell's, not this app's: Settings
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

    std::fs::create_dir_all(qs_dir()).map_err(estr)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(&cur).map_err(estr)?).map_err(estr)?;
    // rename is atomic on the same filesystem: the shell either sees the old
    // file or the new one, never a half-written one.
    std::fs::rename(&tmp, &path).map_err(estr)?;

    reload_shell().await?;
    Ok(cur)
}

/// Ask the running shell to re-read its user state. Not an error when it fails:
/// the shell may simply not be running (Settings launched from a TTY), and the
/// write above is still valid — it will be picked up at next start.
pub async fn reload_shell() -> Result<(), String> {
    let _ = Command::new("qs")
        .args(["ipc", "call", "settings", "reload"])
        .output()
        .await;
    Ok(())
}

#[tauri::command]
pub async fn shell_running() -> Result<bool, String> {
    let out = Command::new("qs")
        .args(["ipc", "call", "settings", "ping"])
        .output()
        .await;
    Ok(matches!(out, Ok(o) if o.status.success()))
}
