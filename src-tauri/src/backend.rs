//! Everything the panes need from the system, kept deliberately narrow.
//!
//! The frontend owns ALL generation logic (Lua/conf text, profile shapes) —
//! ported line-for-line from the shell's Settings.qml so both writers emit
//! identical files. This module only:
//!   * reads/writes files under an explicit allowlist (atomically),
//!   * runs `hyprctl` with validated argument shapes,
//!   * runs a fixed set of named probes/actions (no free-form shell).
//!
//! Nothing here is privileged. The one sysfs write (charge ceiling) relies on
//! the udev rule install.sh ships; when it is absent the attribute simply is
//! not writable and the UI says so.

use std::path::PathBuf;
use std::process::Stdio;

use serde_json::{json, Value};
use tokio::process::Command;

use crate::util::estr;

fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/".into()))
}

// ── RFC-001: route state-file writes through ewe-conf ───────────────────────
// The one file (~/.config/ewe/ewe.conf) has ONE writer, ewe-conf; this app
// persists through it and the runtime JSONs the shell reads become build
// artifacts. Unmatched paths (generated lua/conf, app caches) keep the direct
// atomic write until RFC Phase 4 moves the generators too. When ewe-conf is
// absent (a pre-0.9 DE) we fall back to the direct write, so this app keeps
// working against older desktops.

pub(crate) fn ewe_conf_bin_pub() -> Option<PathBuf> {
    ewe_conf_bin()
}

/// Direct `ewe-conf set` for the domains whose generators moved INTO
/// ewe-conf (RFC-001 Phase 4 — desktop.input first): the pane sends its
/// state object, ewe-conf derives every artifact. Key shape is validated
/// so the frontend can't reach arbitrary keys by accident.
#[tauri::command]
pub async fn set_conf(key: String, value: Value) -> Result<(), String> {
    if !key
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_')
    {
        return Err(format!("bad conf key: {key}"));
    }
    let Some(bin) = ewe_conf_bin() else {
        return Err("ewe-conf not installed".into());
    };
    let out = Command::new(bin)
        .args(["set", "--no-hooks", &key])
        .arg(value.to_string())
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(estr)?;
    if !out.status.success() {
        return Err(format!(
            "ewe-conf set {key} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    crate::shell::poke_sync();
    Ok(())
}

fn ewe_conf_bin() -> Option<PathBuf> {
    let farm = home().join(".config/quickshell/../../bin/ewe-conf");
    if farm.exists() {
        return Some(farm);
    }
    let usr = PathBuf::from("/usr/bin/ewe-conf");
    if usr.exists() {
        return Some(usr);
    }
    None
}

/// rel → (ewe.conf key, JSON pluck) for the state files ewe-conf owns.
fn conf_route(rel: &str) -> Option<(&'static str, Option<&'static str>)> {
    match rel {
        "quickshell/pinned-apps.json" => Some(("apps.pinned", None)),
        "quickshell/places.json" => Some(("apps.places", None)),
        "quickshell/startup-apps.json" => Some(("apps.startup", Some("apps"))),
        "quickshell/animations.json" => Some(("desktop.animations.detail", None)),
        "quickshell/display-profiles.json" => Some(("desktop.displays", None)),
        "quickshell/window-rules.json" => Some(("desktop.window_rules", Some("rules"))),
        _ => None,
    }
}

fn parse_wallpapers(text: &str) -> Value {
    let mut outputs = serde_json::Map::new();
    let mut w = serde_json::Map::new();
    w.insert("mode".into(), "fill".into());
    w.insert("mute".into(), Value::Bool(true));
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        match k {
            "mode" => {
                w.insert("mode".into(), v.into());
            }
            "mute" => {
                w.insert("mute".into(), Value::Bool(v != "0" && v != "false"));
            }
            "backend" => {
                w.insert("backend".into(), v.into());
            }
            "*" => {
                w.insert("default".into(), v.into());
            }
            _ => {
                outputs.insert(k.into(), v.into());
            }
        }
    }
    w.insert("outputs".into(), Value::Object(outputs));
    Value::Object(w)
}

async fn write_via_ewe_conf(rel: &str, content: &str) -> Option<Result<(), String>> {
    let bin_probe = ewe_conf_bin();
    if rel == "hypr/generated/wallpapers.conf" {
        let bin = bin_probe?;
        let out = Command::new(bin)
            .args(["set", "--no-hooks", "desktop.wallpapers"])
            .arg(parse_wallpapers(content).to_string())
            .stdin(Stdio::null())
            .output()
            .await;
        return match out {
            Ok(o) if o.status.success() => Some(Ok(())),
            Ok(o) => Some(Err(format!(
                "ewe-conf set desktop.wallpapers failed: {}",
                String::from_utf8_lossy(&o.stderr)
            ))),
            Err(_) => None,
        };
    }
    let (key, pluck) = conf_route(rel)?;
    let bin = bin_probe?;
    let mut val: Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(e) => return Some(Err(format!("{rel}: not JSON: {e}"))),
    };
    if let Some(field) = pluck {
        val = val.get(field).cloned().unwrap_or(Value::Array(vec![]));
    }
    let out = Command::new(bin)
        .args(["set", "--no-hooks", key])
        .arg(val.to_string())
        .stdin(Stdio::null())
        .output()
        .await;
    match out {
        Ok(o) if o.status.success() => Some(Ok(())),
        Ok(o) => Some(Err(format!(
            "ewe-conf set {key} failed: {}",
            String::from_utf8_lossy(&o.stderr)
        ))),
        // exec failure (binary vanished between the check and the call):
        // let the caller fall back to the direct write
        Err(_) => None,
    }
}

// ── config file access (allowlisted, atomic) ────────────────────────────────
// Paths are relative to ~/.config. The allowlist is prefix+suffix based: the
// generated dirs plus the shell's own JSON state. Nothing outside ever.

fn is_allowed_read(rel: &str) -> bool {
    rel == "hypr/SHORTCUTS.md"
        || rel.starts_with("hypr/generated/")
        || (rel.starts_with("quickshell/") && rel.ends_with(".json"))
}

fn is_allowed_write(rel: &str) -> bool {
    rel.starts_with("hypr/generated/") || (rel.starts_with("quickshell/") && rel.ends_with(".json"))
}

fn config_path(rel: &str) -> Result<PathBuf, String> {
    if rel.contains("..") || rel.starts_with('/') {
        return Err("invalid path".into());
    }
    Ok(home().join(".config").join(rel))
}

#[tauri::command]
pub async fn read_config(rel: String) -> Result<String, String> {
    if !is_allowed_read(&rel) {
        return Err(format!("read not allowed: {rel}"));
    }
    // A missing file is "nothing configured yet", same as the shell treats it.
    Ok(std::fs::read_to_string(config_path(&rel)?).unwrap_or_default())
}

#[tauri::command]
pub async fn write_config(rel: String, content: String) -> Result<(), String> {
    if !is_allowed_write(&rel) {
        return Err(format!("write not allowed: {rel}"));
    }
    // RFC-001: state files ewe-conf owns persist through it instead
    if let Some(res) = write_via_ewe_conf(&rel, &content).await {
        res?;
        crate::shell::poke_sync();
        return Ok(());
    }
    let path = config_path(&rel)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(estr)?;
    }
    // temp + rename: the shell either sees the old file or the new one.
    // The shell's own atomic writer (HyprMon.atomicWrite) emits content + "\n"
    // via its heredoc; match it exactly so alternating writers never churn the
    // file by a trailing byte.
    let mut body = content;
    body.push('\n');
    let tmp = path.with_extension("hs-tmp");
    std::fs::write(&tmp, body.as_bytes()).map_err(estr)?;
    std::fs::rename(&tmp, &path).map_err(estr)?;
    crate::shell::poke_sync();
    Ok(())
}

#[tauri::command]
pub async fn remove_config(rel: String) -> Result<(), String> {
    if !is_allowed_write(&rel) {
        return Err(format!("remove not allowed: {rel}"));
    }
    let _ = std::fs::remove_file(config_path(&rel)?);
    crate::shell::poke_sync();
    Ok(())
}

// ── hyprctl ─────────────────────────────────────────────────────────────────

async fn run_out(bin: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new(bin)
        .args(args)
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(estr)?;
    let mut s = String::from_utf8_lossy(&out.stdout).to_string();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    Ok(s)
}

/// Read-only hyprctl queries the panes need. First argument gates the shape.
#[tauri::command]
pub async fn hyprctl_query(args: Vec<String>) -> Result<String, String> {
    let ok = match args.first().map(String::as_str) {
        Some("monitors") | Some("devices") | Some("getoption") => true,
        // --batch is only ever a series of getoption calls
        Some("--batch") => args
            .get(1)
            .map(|b| b.split(';').all(|c| c.trim().starts_with("getoption ")))
            .unwrap_or(false),
        _ => false,
    };
    if !ok {
        return Err("hyprctl arguments not allowed".into());
    }
    let v: Vec<&str> = args.iter().map(String::as_str).collect();
    run_out("hyprctl", &v).await
}

/// The one eval runner (mirrors HyprMon.runEvals): each statement is a single
/// `hl.…` Lua line; any non-`ok` reply is an error the UI must surface.
#[tauri::command]
pub async fn run_evals(stmts: Vec<String>) -> Result<(), String> {
    let mut errs: Vec<String> = Vec::new();
    for s in &stmts {
        let t = s.trim();
        if !t.starts_with("hl.") || t.contains('\n') {
            return Err("only single-line hl.* statements are allowed".into());
        }
        let out = run_out("hyprctl", &["eval", t]).await?;
        if !out.trim_start().starts_with("ok") {
            errs.push(
                out.lines()
                    .next()
                    .unwrap_or("hyprctl eval failed")
                    .to_string(),
            );
        }
    }
    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs.remove(0))
    }
}

/// The escape hatch for a black screen: wake dpms, then the frontend re-applies
/// the saved profile. The dispatch arg must be Lua on this Hyprland.
#[tauri::command]
pub async fn dpms_on() -> Result<(), String> {
    let _ = run_out("hyprctl", &["dispatch", "hl.dsp.dpms(\"on\")"]).await?;
    Ok(())
}

/// Full config reload — needed when a change can only land via re-sourcing the
/// files (a window rule cannot be withdrawn at runtime, only added).
#[tauri::command]
pub async fn hyprctl_reload() -> Result<(), String> {
    let _ = run_out("hyprctl", &["reload"]).await?;
    Ok(())
}

/// Which connectors can do adaptive sync at all, from
/// /sys/class/drm/card*-<connector>/vrr_capable. Hyprland silently ignores a
/// vrr rule the hardware can't honour, so the UI needs this to dim the toggle
/// instead of claiming "Applied" for a no-op. Missing node → not capable.
#[tauri::command]
pub async fn vrr_caps() -> Result<serde_json::Value, String> {
    let mut map = serde_json::Map::new();
    if let Ok(rd) = std::fs::read_dir("/sys/class/drm") {
        for e in rd.flatten() {
            let fname = e.file_name().to_string_lossy().to_string();
            let Some((_, conn)) = fname.split_once('-') else {
                continue;
            };
            let cap = std::fs::read_to_string(e.path().join("vrr_capable"))
                .map(|s| s.trim() == "1")
                .unwrap_or(false);
            map.insert(conn.to_string(), serde_json::Value::Bool(cap));
        }
    }
    Ok(serde_json::Value::Object(map))
}

/// The system xkb registry (/usr/share/X11/xkb/rules/base.lst): every layout
/// and every variant xkeyboard-config ships — the same list other distros'
/// settings UIs offer (~250 layouts, ~900 variants). Returned in the UI's
/// {c, n} idiom; the curated KB_PRESETS array stays only as a fallback for
/// the day the file is missing.
#[tauri::command]
pub async fn xkb_registry() -> Result<serde_json::Value, String> {
    let text =
        std::fs::read_to_string("/usr/share/X11/xkb/rules/base.lst").map_err(|e| e.to_string())?;
    let mut layouts: Vec<(String, String, Vec<serde_json::Value>)> = Vec::new();
    let mut index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut section = String::new();
    for line in text.lines() {
        let t = line.trim();
        if let Some(s) = t.strip_prefix('!') {
            section = s.trim().to_string();
            continue;
        }
        if t.is_empty() {
            continue;
        }
        match section.as_str() {
            "layout" => {
                if let Some((code, name)) = t.split_once(char::is_whitespace) {
                    index.insert(code.to_string(), layouts.len());
                    layouts.push((code.to_string(), name.trim().to_string(), Vec::new()));
                }
            }
            "variant" => {
                // "  intl            us: English (US, intl., with dead keys)"
                if let Some((code, rest)) = t.split_once(char::is_whitespace) {
                    if let Some((layout, desc)) = rest.trim().split_once(':') {
                        if let Some(&i) = index.get(layout.trim()) {
                            layouts[i].2.push(serde_json::json!({
                                "c": code, "n": desc.trim()
                            }));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    let arr: Vec<serde_json::Value> = layouts
        .into_iter()
        .map(|(c, n, variants)| serde_json::json!({ "c": c, "n": n, "variants": variants }))
        .collect();
    Ok(serde_json::Value::Array(arr))
}

// ── Time & Place ────────────────────────────────────────────────────────────
// The auto-timezone dispatcher (system/networkmanager/60-ewe-auto-timezone)
// honours two root-owned flag files: /etc/ewe/manual-timezone ("a human chose
// this zone, never touch it") and /etc/ewe/no-auto-timezone (kill switch).
// This pane is the ONLY writer of the latch; zone/NTP changes go through
// timedatectl, whose polkit policy prompts via the DE's own agent when needed.

#[tauri::command]
pub async fn time_info() -> Result<Value, String> {
    let show = run_out("timedatectl", &["show"]).await?;
    let mut tz = String::new();
    let mut ntp = false;
    let mut synced = false;
    for line in show.lines() {
        if let Some(v) = line.strip_prefix("Timezone=") {
            tz = v.into();
        } else if let Some(v) = line.strip_prefix("NTP=") {
            ntp = v == "yes";
        } else if let Some(v) = line.strip_prefix("NTPSynchronized=") {
            synced = v == "yes";
        }
    }
    Ok(json!({
        "timezone": tz,
        "ntp": ntp,
        "ntpSynced": synced,
        "auto": !std::path::Path::new("/etc/ewe/manual-timezone").exists(),
        "killSwitch": std::path::Path::new("/etc/ewe/no-auto-timezone").exists(),
    }))
}

#[tauri::command]
pub async fn list_timezones() -> Result<Vec<String>, String> {
    Ok(run_out("timedatectl", &["list-timezones"])
        .await?
        .lines()
        .map(String::from)
        .collect())
}

/// A manual pick sets the zone AND drops the latch, so auto-detect never
/// fights the human's choice (the dispatcher checks the latch first).
#[tauri::command]
pub async fn set_timezone(tz: String) -> Result<(), String> {
    let ok_charset = tz
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "/_+-".contains(c));
    if !ok_charset || !std::path::Path::new(&format!("/usr/share/zoneinfo/{tz}")).exists() {
        return Err(format!("unknown timezone: {tz}"));
    }
    run_out("timedatectl", &["set-timezone", &tz]).await?;
    let _ = run_out(
        "pkexec",
        &[
            "sh",
            "-c",
            "mkdir -p /etc/ewe && touch /etc/ewe/manual-timezone",
        ],
    )
    .await;
    Ok(())
}

/// Auto on: remove the latch, clear the flap-throttle stamp, and give the
/// dispatcher an immediate shot instead of waiting for the next network
/// event. Auto off: latch the current zone as the human's choice.
#[tauri::command]
pub async fn set_auto_timezone(on: bool) -> Result<(), String> {
    let script = if on {
        "rm -f /etc/ewe/manual-timezone /run/ewe-auto-timezone.stamp; \
         [ -x /etc/NetworkManager/dispatcher.d/60-ewe-auto-timezone ] && \
         exec /etc/NetworkManager/dispatcher.d/60-ewe-auto-timezone settings up || true"
    } else {
        "mkdir -p /etc/ewe && touch /etc/ewe/manual-timezone"
    };
    run_out("pkexec", &["sh", "-c", script]).await?;
    Ok(())
}

#[tauri::command]
pub async fn set_ntp(on: bool) -> Result<(), String> {
    run_out(
        "timedatectl",
        &["set-ntp", if on { "true" } else { "false" }],
    )
    .await?;
    Ok(())
}

// ── named actions (fixed scripts, no free-form shell from the frontend) ─────

async fn run_sh(script: &str) -> Result<String, String> {
    let out = Command::new("sh")
        .args(["-c", script])
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(estr)?;
    let mut s = String::from_utf8_lossy(&out.stdout).to_string();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    Ok(s)
}

/// wallpaper.sh is the single authority on backends; ask it.
#[tauri::command]
pub async fn wallpaper_backend() -> Result<String, String> {
    run_sh("\"$HOME/.config/hypr/scripts/wallpaper.sh\" --backend 2>/dev/null").await
}

/// Re-apply wallpapers; error:/note: lines come back for the banner.
#[tauri::command]
pub async fn wallpaper_reapply() -> Result<String, String> {
    run_sh("\"$HOME/.config/hypr/scripts/wallpaper.sh\" --reapply 2>&1").await
}

/// Restart hypridle on the generated config (Screensaver pane). The sleep lets
/// the atomic temp+rename land before hypridle reads the file.
#[tauri::command]
pub async fn restart_hypridle() -> Result<(), String> {
    let _ = run_sh(
        "pkill -x hypridle; sleep 0.6; setsid hypridle -c \"$HOME/.config/hypr/generated/hypridle.conf\" >/dev/null 2>&1 &",
    )
    .await;
    Ok(())
}

/// Per-window keyboard-layout daemon control. The [k] pattern trick keeps
/// pgrep/pkill from matching the wrapping `sh -c` itself.
#[tauri::command]
pub async fn per_window_kb(action: String) -> Result<bool, String> {
    match action.as_str() {
        "status" => {}
        "on" => {
            let _ = run_sh("rm -f \"$HOME/.config/hypr/generated/kb-per-window.disabled\"; pgrep -f \"[k]b-per-window.py\" >/dev/null || setsid python3 \"$HOME/.config/hypr/scripts/kb-per-window.py\" >/dev/null 2>&1 &").await;
        }
        "off" => {
            let _ = run_sh("mkdir -p \"$HOME/.config/hypr/generated\"; touch \"$HOME/.config/hypr/generated/kb-per-window.disabled\"; pkill -f \"[k]b-per-window.py\"").await;
        }
        "restart" => {
            let _ = run_sh("pgrep -f \"[k]b-per-window.py\" >/dev/null && { pkill -f \"[k]b-per-window.py\"; sleep 0.3; setsid python3 \"$HOME/.config/hypr/scripts/kb-per-window.py\" >/dev/null 2>&1 & }; true").await;
        }
        _ => return Err("unknown action".into()),
    }
    if action == "on" || action == "off" {
        crate::shell::poke_sync(); // the disabled-flag file travels in the sync bundle
    }
    // settle, then report the live truth
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let out = run_sh("pgrep -f \"[k]b-per-window.py\" >/dev/null && echo yes || echo no").await?;
    Ok(out.trim() == "yes")
}

/// GTK/Qt/icon/cursor theming in one pass — colorscheme.sh is the single
/// authority (same script the shell runs at startup).
#[tauri::command]
pub async fn apply_colorscheme(scheme: String, accent: String) -> Result<(), String> {
    if scheme != "dark" && scheme != "light" {
        return Err("scheme must be dark|light".into());
    }
    let acc = accent.trim_start_matches('#').to_string();
    if !acc.chars().all(|c| c.is_ascii_hexdigit()) || (acc.len() != 6 && acc.len() != 8) {
        return Err("accent must be a hex colour".into());
    }
    let _ = run_sh(&format!(
        "\"$HOME/.config/quickshell/scripts/colorscheme.sh\" {scheme} {acc} >/dev/null 2>&1 &"
    ))
    .await;
    crate::shell::poke_sync();
    Ok(())
}

// ── wallpapers ──────────────────────────────────────────────────────────────

const WP_EXTS: &[&str] = &[
    "jpg", "jpeg", "png", "webp", "gif", "mp4", "webm", "mkv", "mov", "m4v",
];

#[tauri::command]
pub async fn default_wallpaper_dir() -> Result<String, String> {
    let h = home();
    // the shipped ewe set first (config-farm resolution finds the checkout's
    // copy; /usr/share/ewe covers packaged installs), then the user's folders
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(farm) = std::fs::canonicalize(h.join(".config/quickshell")) {
        if let Some(root) = farm.parent().and_then(|p| p.parent()) {
            dirs.push(root.join("system/branding/wallpapers"));
        }
    }
    dirs.push(PathBuf::from("/usr/share/ewe/system/branding/wallpapers"));
    dirs.extend([h.join("Pictures/Wallpapers"), h.join("Pictures"), h.clone()]);
    for d in dirs {
        if d.is_dir() {
            return Ok(d.to_string_lossy().to_string());
        }
    }
    Ok(h.to_string_lossy().to_string())
}

/// Image/video files directly inside `dir` (no recursion — same as the shell).
#[tauri::command]
pub async fn list_wallpapers(dir: String) -> Result<Vec<String>, String> {
    let p = PathBuf::from(&dir);
    if !p.is_absolute() || !p.starts_with(home()) {
        return Err("directory must be inside your home".into());
    }
    let mut out: Vec<String> = Vec::new();
    let rd = match std::fs::read_dir(&p) {
        Ok(r) => r,
        Err(_) => return Ok(out),
    };
    for e in rd.flatten() {
        let path = e.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if WP_EXTS.contains(&ext.as_str()) {
            out.push(path.to_string_lossy().to_string());
        }
    }
    out.sort();
    Ok(out)
}

// ── startup pane: installed applications ────────────────────────────────────

fn parse_desktop_file(path: &std::path::Path) -> Option<Value> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut name = String::new();
    let mut exec = String::new();
    let mut icon = String::new();
    let mut comment = String::new();
    let mut wm_class = String::new();
    let mut in_entry = false;
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            in_entry = l == "[Desktop Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }
        if let Some((k, v)) = l.split_once('=') {
            match k {
                "NoDisplay" | "Hidden" if v.trim() == "true" => return None,
                "Type" if v.trim() != "Application" => return None,
                "Name" if name.is_empty() => name = v.trim().to_string(),
                "Exec" if exec.is_empty() => exec = v.trim().to_string(),
                "Icon" if icon.is_empty() => icon = v.trim().to_string(),
                "Comment" if comment.is_empty() => comment = v.trim().to_string(),
                // window class for rule matching (Window Rules pane); absent
                // for most Wayland apps, whose app-id equals the desktop id
                "StartupWMClass" if wm_class.is_empty() => wm_class = v.trim().to_string(),
                _ => {}
            }
        }
    }
    if name.is_empty() || exec.is_empty() {
        return None;
    }
    let id = path.file_stem()?.to_str()?.to_string();
    Some(
        json!({ "id": id, "name": name, "exec": exec, "icon": icon, "comment": comment, "wmClass": wm_class }),
    )
}

/// Launchable applications, for the Startup pane's picker.
#[tauri::command]
pub async fn desktop_apps() -> Result<Vec<Value>, String> {
    let mut dirs = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        home().join(".local/share/applications"),
    ];
    if let Ok(x) = std::env::var("XDG_DATA_DIRS") {
        for d in x.split(':') {
            let p = PathBuf::from(d).join("applications");
            if !dirs.contains(&p) {
                dirs.push(p);
            }
        }
    }
    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<Value> = Vec::new();
    for dir in dirs {
        let rd = match std::fs::read_dir(&dir) {
            Ok(r) => r,
            Err(_) => continue,
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) != Some("desktop") {
                continue;
            }
            let id = p
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            if id.is_empty() || !seen.insert(id) {
                continue; // earlier dir wins, same as the desktop's lookup order
            }
            if let Some(v) = parse_desktop_file(&p) {
                out.push(v);
            }
        }
    }
    out.sort_by(|a, b| {
        a["name"]
            .as_str()
            .unwrap_or("")
            .to_lowercase()
            .cmp(&b["name"].as_str().unwrap_or("").to_lowercase())
    });
    Ok(out)
}

// ── default apps (xdg-mime / gio) ───────────────────────────────────────────

fn valid_desktop_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 256
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '@' | '+'))
}

fn valid_mime(m: &str) -> bool {
    !m.is_empty()
        && m.len() <= 128
        && m.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '_' | '-' | '+'))
}

#[tauri::command]
pub async fn mime_default(mime: String) -> Result<String, String> {
    if !valid_mime(&mime) {
        return Err("invalid mime".into());
    }
    Ok(run_out("xdg-mime", &["query", "default", &mime])
        .await?
        .trim()
        .to_string())
}

/// Apps that can handle a mime type — `gio mime` recommended, falling back to
/// registered (same as the shell's probe).
#[tauri::command]
pub async fn mime_apps(mime: String) -> Result<Vec<String>, String> {
    if !valid_mime(&mime) {
        return Err("invalid mime".into());
    }
    let text = run_out("gio", &["mime", &mime]).await.unwrap_or_default();
    let mut rec: Vec<String> = Vec::new();
    let mut reg: Vec<String> = Vec::new();
    let mut mode = 0;
    for line in text.lines() {
        if line.starts_with("Recommended applications:") {
            mode = 1;
            continue;
        }
        if line.starts_with("Registered applications:") {
            mode = 2;
            continue;
        }
        if !line.starts_with('\t') && !line.starts_with(' ') {
            mode = 0;
            continue;
        }
        let id = line.trim().to_string();
        if id.is_empty() {
            continue;
        }
        if mode == 1 {
            rec.push(id);
        } else if mode == 2 {
            reg.push(id);
        }
    }
    Ok(if rec.is_empty() { reg } else { rec })
}

#[tauri::command]
pub async fn set_mime_default(
    desktop_id: String,
    mimes: Vec<String>,
    set_browser: bool,
) -> Result<(), String> {
    if !valid_desktop_id(&desktop_id) {
        return Err("invalid desktop id".into());
    }
    for m in &mimes {
        if !valid_mime(m) {
            return Err("invalid mime".into());
        }
        let _ = run_out("xdg-mime", &["default", &desktop_id, m]).await?;
    }
    if set_browser {
        let _ = run_out("xdg-settings", &["set", "default-web-browser", &desktop_id]).await?;
    }
    crate::shell::poke_sync(); // mimeapps.list travels in the sync bundle
    Ok(())
}

// ── system diagnostics (System pane) ────────────────────────────────────────

#[tauri::command]
pub async fn diagnostics() -> Result<Value, String> {
    let text = run_sh(concat!(
        "echo \"gsession=$(systemctl --user is-active graphical-session.target 2>/dev/null)\";",
        "echo \"portal=$(systemctl --user is-active xdg-desktop-portal 2>/dev/null)\";",
        "echo \"portal_hypr=$(systemctl --user is-active xdg-desktop-portal-hyprland 2>/dev/null)\";",
        "echo \"portal_gtk=$(systemctl --user is-active xdg-desktop-portal-gtk 2>/dev/null)\";",
        "echo \"browser=$(xdg-settings get default-web-browser 2>/dev/null)\";",
        "echo \"gpu=$(lspci -k 2>/dev/null | grep -A3 -iE 'VGA|Display|3D' | grep -i 'driver in use' | head -1 | sed 's/.*use: //')\";",
        "echo \"mem=$(free -m 2>/dev/null | awk '/Mem:/{print $3\"/\"$2\" MB\"}')\";",
        "echo \"disk=$(df -h / 2>/dev/null | awk 'NR==2{print $3\"/\"$2\" (\"$5\")\"}')\";",
        "echo \"kernel=$(uname -r)\";",
        "echo \"hypr=$(hyprctl version 2>/dev/null | head -1)\";",
        "echo \"hypridle=$(command -v hypridle >/dev/null && echo yes || echo no)\"",
    ))
    .await?;
    let mut map = serde_json::Map::new();
    for line in text.lines() {
        if let Some((k, v)) = line.split_once('=') {
            map.insert(k.to_string(), Value::String(v.to_string()));
        }
    }
    Ok(Value::Object(map))
}

// ── power ───────────────────────────────────────────────────────────────────

fn read_sys(p: &std::path::Path) -> Option<String> {
    std::fs::read_to_string(p)
        .ok()
        .map(|s| s.trim().to_string())
}

fn read_sys_i64(p: &std::path::Path) -> Option<i64> {
    read_sys(p)?.parse().ok()
}

#[tauri::command]
pub async fn power_info() -> Result<Value, String> {
    let mut bat = json!(null);
    if let Ok(rd) = std::fs::read_dir("/sys/class/power_supply") {
        for e in rd.flatten() {
            let p = e.path();
            let typ = read_sys(&p.join("type")).unwrap_or_default();
            if typ != "Battery" {
                continue;
            }
            let full = read_sys_i64(&p.join("charge_full"))
                .or_else(|| read_sys_i64(&p.join("energy_full")));
            let design = read_sys_i64(&p.join("charge_full_design"))
                .or_else(|| read_sys_i64(&p.join("energy_full_design")));
            let health = match (full, design) {
                (Some(f), Some(d)) if d > 0 => json!(f as f64 / d as f64),
                _ => json!(null),
            };
            let chg_path = p.join("charge_control_end_threshold");
            let chg = read_sys_i64(&chg_path);
            let writable = chg.is_some()
                && std::fs::OpenOptions::new()
                    .append(true)
                    .open(&chg_path)
                    .is_ok();
            bat = json!({
                "capacity": read_sys_i64(&p.join("capacity")),
                "status": read_sys(&p.join("status")),
                "cycles": read_sys_i64(&p.join("cycle_count")),
                "health": health,
                "chargeLimit": chg,
                "chargeLimitWritable": writable,
                "chargePath": chg_path.to_string_lossy(),
            });
            break;
        }
    }
    let ppd = run_out("powerprofilesctl", &["get"])
        .await
        .unwrap_or_default();
    let ppd_list = run_out("powerprofilesctl", &["list"])
        .await
        .unwrap_or_default();
    let profiles: Vec<String> = ppd_list
        .lines()
        .filter_map(|l| {
            let t = l.trim().trim_start_matches("* ");
            t.strip_suffix(':').map(|s| s.to_string())
        })
        .collect();
    // keyboard backlight via brightnessctl (no logind D-Bus from here)
    let kbd = run_out("brightnessctl", &["-m", "-d", "*::kbd_backlight", "info"])
        .await
        .unwrap_or_default();
    let kf: Vec<&str> = kbd.trim().split(',').collect();
    let kbd_val = json!({
        "present": kf.len() >= 5,
        "value": kf.get(2).and_then(|s| s.parse::<i64>().ok()),
        "max": kf.get(4).and_then(|s| s.parse::<i64>().ok()),
    });
    Ok(json!({
        "battery": bat,
        "profile": ppd.trim(),
        "profiles": profiles,
        "kbdBacklight": kbd_val,
    }))
}

#[tauri::command]
pub async fn set_power_profile(profile: String) -> Result<(), String> {
    if !["power-saver", "balanced", "performance"].contains(&profile.as_str()) {
        return Err("unknown profile".into());
    }
    let out = run_out("powerprofilesctl", &["set", &profile]).await?;
    if out.to_lowercase().contains("error") {
        return Err(out);
    }
    Ok(())
}

/// Charge ceiling: the kernel attribute is group-writable only when the udev
/// rule from install.sh is present — otherwise this errors and the UI says why.
#[tauri::command]
pub async fn set_charge_limit(pct: i64) -> Result<(), String> {
    let v = pct.clamp(20, 100);
    if let Ok(rd) = std::fs::read_dir("/sys/class/power_supply") {
        for e in rd.flatten() {
            let p = e.path().join("charge_control_end_threshold");
            if p.exists() {
                return std::fs::write(&p, v.to_string()).map_err(estr);
            }
        }
    }
    Err("no charge-ceiling control on this machine".into())
}

#[tauri::command]
pub async fn set_kbd_backlight(value: i64) -> Result<(), String> {
    let _ = run_out(
        "brightnessctl",
        &["-d", "*::kbd_backlight", "s", &value.max(0).to_string()],
    )
    .await?;
    Ok(())
}

// ── qs ipc (allowlisted verbs only) ─────────────────────────────────────────
// The Google account and the screensaver preview live in the SHELL — the app
// only sends verbs and reads the status snapshot. Tokens never cross here.

#[tauri::command]
pub async fn qs_ipc(target: String, func: String, arg: Option<String>) -> Result<String, String> {
    const ALLOWED: &[(&str, &[&str])] = &[
        (
            "google",
            &[
                "signIn",
                "signOut",
                "syncNow",
                "refresh",
                "setAutoSync",
                "status",
            ],
        ),
        ("saver", &["show"]),
    ];
    let ok = ALLOWED
        .iter()
        .any(|(t, fs)| *t == target && fs.contains(&func.as_str()));
    if !ok {
        return Err(format!("ipc not allowed: {target} {func}"));
    }
    if let Some(a) = &arg {
        if a != "true" && a != "false" {
            return Err("only boolean ipc arguments are allowed".into());
        }
    }
    let mut args = vec![target.as_str(), func.as_str()];
    if let Some(a) = &arg {
        args.push(a.as_str());
    }
    let out = crate::shell::qs_call(&args).await.map_err(estr)?;
    let mut s = String::from_utf8_lossy(&out.stdout).to_string();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    Ok(s)
}

// ── networking (nmcli — argv vectors, never a shell string) ─────────────────

#[tauri::command]
pub async fn net_status() -> Result<Value, String> {
    let has_wifi = run_out("nmcli", &["-t", "-f", "TYPE", "device", "status"])
        .await
        .unwrap_or_default()
        .lines()
        .any(|l| l.trim() == "wifi");
    let wifi_on = run_out("nmcli", &["-t", "-f", "WIFI", "radio"])
        .await
        .unwrap_or_default()
        .trim()
        == "enabled";

    let mut wifi: Vec<Value> = Vec::new();
    if has_wifi && wifi_on {
        let text = run_out(
            "nmcli",
            &[
                "-t",
                "-f",
                "IN-USE,SIGNAL,SECURITY,SSID",
                "device",
                "wifi",
                "list",
            ],
        )
        .await
        .unwrap_or_default();
        let mut seen = std::collections::HashSet::new();
        for l in text.lines() {
            let p: Vec<&str> = l.splitn(4, ':').collect();
            if p.len() < 4 {
                continue;
            }
            let ssid = p[3];
            if ssid.is_empty() || !seen.insert(ssid.to_string()) {
                continue;
            }
            wifi.push(json!({
                "ssid": ssid,
                "signal": p[1].parse::<i64>().unwrap_or(0),
                "sec": p[2],
                "active": p[0] == "*",
            }));
        }
        wifi.sort_by_key(|w| {
            (
                !(w["active"].as_bool().unwrap_or(false)),
                -w["signal"].as_i64().unwrap_or(0),
            )
        });
    }

    let mut active: Vec<Value> = Vec::new();
    for l in run_out(
        "nmcli",
        &[
            "-t",
            "-f",
            "NAME,TYPE,DEVICE,STATE",
            "connection",
            "show",
            "--active",
        ],
    )
    .await
    .unwrap_or_default()
    .lines()
    {
        // NAME may itself contain ':' — nmcli escapes it, but be permissive and
        // split from the right for the three fixed fields.
        let p: Vec<&str> = l.rsplitn(4, ':').collect();
        if p.len() == 4 {
            active.push(json!({ "name": p[3], "type": p[2], "dev": p[1], "state": p[0] }));
        }
    }

    let mut vpn: Vec<Value> = Vec::new();
    for l in run_out(
        "nmcli",
        &["-t", "-f", "NAME,TYPE,ACTIVE", "connection", "show"],
    )
    .await
    .unwrap_or_default()
    .lines()
    {
        let p: Vec<&str> = l.rsplitn(3, ':').collect();
        if p.len() == 3 && (p[1].contains("vpn") || p[1].contains("wireguard")) {
            vpn.push(json!({ "name": p[2], "active": p[0] == "yes" }));
        }
    }

    let ips: Vec<String> =
        run_sh("ip -4 -o addr show scope global 2>/dev/null | awk '{print $2\": \"$4}'")
            .await
            .unwrap_or_default()
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();

    let ssh: Vec<String> = std::fs::read_to_string(home().join(".ssh/config"))
        .unwrap_or_default()
        .lines()
        .filter(|l| l.trim_start().to_lowercase().starts_with("host "))
        .flat_map(|l| {
            l.split_whitespace()
                .skip(1)
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        })
        .filter(|h| !h.contains('*') && !h.contains('?'))
        .collect();

    Ok(json!({
        "hasWifi": has_wifi, "wifiOn": wifi_on, "wifi": wifi,
        "active": active, "vpn": vpn, "ips": ips, "sshHosts": ssh,
    }))
}

#[tauri::command]
pub async fn wifi_set(on: bool) -> Result<(), String> {
    let _ = run_out("nmcli", &["radio", "wifi", if on { "on" } else { "off" }]).await?;
    Ok(())
}

#[tauri::command]
pub async fn wifi_connect(ssid: String, password: Option<String>) -> Result<String, String> {
    if ssid.is_empty() || ssid.len() > 64 {
        return Err("invalid SSID".into());
    }
    let mut args = vec!["device", "wifi", "connect", ssid.as_str()];
    let pw = password.unwrap_or_default();
    if !pw.is_empty() {
        args.push("password");
        args.push(pw.as_str());
    }
    let out = run_out("nmcli", &args).await?;
    if out.contains("Error") {
        return Err(out
            .lines()
            .next()
            .unwrap_or("connection failed")
            .to_string());
    }
    crate::shell::poke_sync(); // the new Wi-Fi profile travels in the sync bundle
    Ok(out)
}

/// Bring a saved connection (VPN/WireGuard) up or down by name.
#[tauri::command]
pub async fn connection_set(name: String, up: bool) -> Result<String, String> {
    if name.is_empty() || name.len() > 128 {
        return Err("invalid connection name".into());
    }
    let out = run_out(
        "nmcli",
        &["connection", if up { "up" } else { "down" }, name.as_str()],
    )
    .await?;
    if out.contains("Error") {
        return Err(out.lines().next().unwrap_or("failed").to_string());
    }
    Ok(out)
}

// ── user account (avatar / name / session facts) ────────────────────────────

/// ~/.face + AccountsService — the same pipeline the in-shell pane used, so
/// the greeter picks the new icon up too.
async fn install_face(tmp: &std::path::Path) -> Result<String, String> {
    let face = home().join(".face");
    std::fs::copy(tmp, &face).map_err(|e| format!("could not save ~/.face: {e}"))?;
    let uid = run_out("id", &["-u"])
        .await
        .unwrap_or_default()
        .trim()
        .to_string();
    let obj = format!("/org/freedesktop/Accounts/User{uid}");
    let out = run_out(
        "busctl",
        &[
            "call",
            "org.freedesktop.Accounts",
            &obj,
            "org.freedesktop.Accounts.User",
            "SetIconFile",
            "s",
            &face.to_string_lossy(),
        ],
    )
    .await
    .unwrap_or_default();
    crate::shell::poke_sync(); // ~/.face travels in the sync bundle
    if out.to_lowercase().contains("error") {
        return Ok("Avatar saved to ~/.face, but AccountsService rejected the update — the login screen may keep the old icon.".into());
    }
    Ok(String::new())
}

/// The frontend crops in a canvas and sends a 512² PNG as base64.
#[tauri::command]
pub async fn save_avatar(png_base64: String) -> Result<String, String> {
    if png_base64.len() > 8_000_000 {
        return Err("image too large".into());
    }
    let bytes = b64_decode(&png_base64).ok_or("invalid image data")?;
    if !bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        return Err("not a PNG".into());
    }
    let tmp = home().join(".cache/ewe-avatar.png");
    if let Some(d) = tmp.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    std::fs::write(&tmp, &bytes).map_err(estr)?;
    install_face(&tmp).await
}

/// Google profile photo → avatar. Host-validated, downloaded with curl, then
/// the same ~/.face pipeline.
#[tauri::command]
pub async fn avatar_from_url(url: String) -> Result<String, String> {
    let ok = url.starts_with("https://lh3.googleusercontent.com/")
        || url.starts_with("https://lh4.googleusercontent.com/")
        || url.starts_with("https://lh5.googleusercontent.com/")
        || url.starts_with("https://lh6.googleusercontent.com/");
    if !ok || url.len() > 1024 {
        return Err("only Google profile photo URLs are allowed here".into());
    }
    let tmp = home().join(".cache/ewe-avatar.png");
    if let Some(d) = tmp.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let out = Command::new("curl")
        .args(["-fsSL", "--max-time", "20", "-o"])
        .arg(&tmp)
        .arg(&url)
        .output()
        .await
        .map_err(estr)?;
    if !out.status.success() {
        return Err("could not download the profile photo".into());
    }
    install_face(&tmp).await
}

#[tauri::command]
pub async fn set_real_name(name: String) -> Result<(), String> {
    let n = name.trim();
    if n.is_empty() || n.len() > 128 || n.contains(':') || n.contains('\n') {
        return Err("invalid name".into());
    }
    let uid = run_out("id", &["-u"])
        .await
        .unwrap_or_default()
        .trim()
        .to_string();
    let obj = format!("/org/freedesktop/Accounts/User{uid}");
    let out = run_out(
        "busctl",
        &[
            "call",
            "org.freedesktop.Accounts",
            &obj,
            "org.freedesktop.Accounts.User",
            "SetRealName",
            "s",
            n,
        ],
    )
    .await?;
    if out.to_lowercase().contains("error") {
        return Err(out
            .lines()
            .next()
            .unwrap_or("AccountsService refused")
            .to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn user_info() -> Result<Value, String> {
    let user = std::env::var("USER").unwrap_or_default();
    let host = run_out("uname", &["-n"])
        .await
        .unwrap_or_default()
        .trim()
        .to_string();
    let real = run_out("getent", &["passwd", &user])
        .await
        .unwrap_or_default()
        .split(':')
        .nth(4)
        .unwrap_or("")
        .split(',')
        .next()
        .unwrap_or("")
        .to_string();
    let uptime = run_sh("uptime -p 2>/dev/null")
        .await
        .unwrap_or_default()
        .trim()
        .to_string();
    let has_face = home().join(".face").is_file();
    Ok(
        json!({ "user": user, "host": host, "realName": real, "uptime": uptime, "hasFace": has_face }),
    )
}

// Minimal base64 decoder (standard alphabet, padding optional) — not worth a
// crate dependency for one call site.
fn b64_decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits = 0u32;
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => (c - b'A') as u32,
            b'a'..=b'z' => (c - b'a' + 26) as u32,
            b'0'..=b'9' => (c - b'0' + 52) as u32,
            b'+' => 62,
            b'/' => 63,
            b'=' | b'\n' | b'\r' => continue,
            _ => return None,
        };
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Some(out)
}
