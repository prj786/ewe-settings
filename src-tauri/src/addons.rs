//! Add-on awareness (ewe 0.25).
//!
//! Since 0.25 several shell features — the dock, music, Places, the phone,
//! mail, cast, VPN/SSH tiles, Insomnia, the system monitor — are opt-in
//! add-ons: plugins vendored in the ewe payload and installed from Komble.
//! Settings has panes for some of them (Layout → Dock, User → Mail), so it
//! asks `ewe-plugin list --json` what is installed and shows a "this is an
//! add-on" note instead of controls that would move nothing.
//!
//! One probe, cached for a few seconds: the python tool costs ~100 ms and
//! three panes ask at once. Defensive by design — on an ewe older than 0.25
//! (no `ewe-plugin`, or one without the `available` list) every add-on is
//! treated as installed, so nothing disappears from Settings on an old shell.
//!
//! Fields relied on from `list --json` (bin/ewe-plugin, the plugin API v3
//! contract): `plugins[].{id,name,description,kinds,barWidget,installed,
//! enabled,valid}`, `available[]` (presence = a 0.25+ tool) and `removed[]`.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tokio::process::Command;

use crate::util::estr;

static CACHE: Mutex<Option<(Instant, Value)>> = Mutex::new(None);
const TTL: Duration = Duration::from_secs(5);

/// `{legacy, why?, installed: {id: enabled}, plugins: [...], available: [...],
/// removed: [...]}` — `installed` lists valid, installed plugins only (an
/// invalid one loads nothing, so for the panes it is absent); `legacy` is the
/// old-ewe escape hatch described above.
#[tauri::command]
pub async fn addons_state(refresh: Option<bool>) -> Result<Value, String> {
    if !refresh.unwrap_or(false) {
        let cached = CACHE.lock().ok().and_then(|g| g.clone());
        if let Some((at, v)) = cached {
            if at.elapsed() < TTL {
                return Ok(v);
            }
        }
    }
    let v = probe().await;
    if let Ok(mut g) = CACHE.lock() {
        *g = Some((Instant::now(), v.clone()));
    }
    Ok(v)
}

fn legacy(why: &str) -> Value {
    eprintln!("addons: {why} — treating every add-on as installed");
    json!({
        "legacy": true,
        "why": why,
        "installed": {},
        "plugins": [],
        "available": [],
        "removed": [],
    })
}

async fn probe() -> Value {
    let Some(bin) = crate::backend::ewe_tool("ewe-plugin") else {
        return legacy("ewe-plugin is not installed (ewe before 0.21)");
    };
    let run = Command::new(bin)
        .args(["list", "--json"])
        .stdin(Stdio::null())
        .output();
    let out = match tokio::time::timeout(Duration::from_secs(15), run).await {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => return legacy(&format!("ewe-plugin failed to start: {e}")),
        Err(_) => return legacy("ewe-plugin list timed out"),
    };
    let Ok(reply) = serde_json::from_slice::<Value>(&out.stdout) else {
        return legacy("ewe-plugin list --json: unreadable reply");
    };
    // `available` arrived with the add-ons (0.25). Without it the tool predates
    // them and the dock, mail, … are still built into the shell.
    let Some(available) = reply.get("available").and_then(Value::as_array) else {
        return legacy("ewe-plugin predates add-ons (no `available` list)");
    };
    let plugins: Vec<Value> = reply
        .get("plugins")
        .and_then(Value::as_array)
        .map(|ps| {
            ps.iter()
                .filter(|p| p.get("installed").and_then(Value::as_bool).unwrap_or(false))
                .map(|p| {
                    json!({
                        "id": p.get("id").cloned().unwrap_or(Value::Null),
                        "name": p.get("name").cloned().unwrap_or(Value::Null),
                        "description": p.get("description").cloned().unwrap_or(Value::Null),
                        "kinds": p.get("kinds").cloned().unwrap_or_else(|| json!([])),
                        "barWidget": p.get("barWidget").cloned().unwrap_or(Value::Null),
                        "enabled": p.get("enabled").and_then(Value::as_bool).unwrap_or(false),
                        "valid": p.get("valid").and_then(Value::as_bool).unwrap_or(true),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let mut installed = serde_json::Map::new();
    for p in &plugins {
        let (Some(id), Some(valid), Some(enabled)) = (
            p.get("id").and_then(Value::as_str),
            p.get("valid").and_then(Value::as_bool),
            p.get("enabled").and_then(Value::as_bool),
        ) else {
            continue;
        };
        if valid {
            installed.insert(id.to_string(), Value::Bool(enabled));
        }
    }
    json!({
        "legacy": false,
        "installed": installed,
        "plugins": plugins,
        "available": available,
        "removed": reply.get("removed").cloned().unwrap_or_else(|| json!([])),
    })
}

/// Komble is the plugin catalog (`komble --addons`, the flag installed callers use). Argv only, detached,
/// nothing inherited from this process — the same shape as opening ewe-sync.
fn komble_bin() -> Option<PathBuf> {
    let p = PathBuf::from("/usr/bin/komble");
    p.exists().then_some(p)
}

/// A plugin id fit for an argv (`ewe.dock`): the same rule as ewe-plugin's.
fn plugin_id_ok(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id.contains('.')
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'))
}

/// Open Komble on its Plugins page — with one plugin's Options dialog open
/// when `options` names it (`komble --options=<id>`; an older Komble ignores
/// the flag and opens on its last page). `Ok(false)` when Komble is not
/// installed; the pane says so instead of failing silently.
#[tauri::command]
pub async fn open_addons(options: Option<String>) -> Result<bool, String> {
    let Some(bin) = komble_bin() else {
        return Ok(false);
    };
    let arg = match options.as_deref() {
        Some(id) if plugin_id_ok(id) => format!("--options={id}"),
        Some(_) => return Err("bad plugin id".into()),
        None => "--addons".into(),
    };
    Command::new(bin)
        .arg(arg)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(estr)?;
    Ok(true)
}
