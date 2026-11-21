mod addons;
mod backend;
mod language;
mod shell;
mod util;

use tauri::{Emitter, Manager};

/// `--page <id>` / `--page=<id>` from a command line: the page the shell asks
/// for (desktop menu "Change wallpaper…" → `wallpaper`, the bar's network
/// menu → `network` …). The page ids are lib/pages.js's; an unknown one is
/// ignored by the front end.
fn page_arg(args: &[String]) -> Option<String> {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--page" {
            return it.next().filter(|p| !p.is_empty()).cloned();
        }
        if let Some(p) = a.strip_prefix("--page=") {
            return (!p.is_empty()).then(|| p.to_string());
        }
    }
    None
}

/// The page this process was started on (`ewe-settings --page wallpaper`);
/// the front end reads it once, at mount.
#[tauri::command]
fn start_page() -> Option<String> {
    page_arg(&std::env::args().collect::<Vec<_>>())
}

pub fn run() {
    tauri::Builder::default()
        // a second `ewe-settings --page X` while Settings is open: bring the
        // window forward AND go to X — until 0.23.1 the page was dropped (and
        // the first start ignored it too), so "Change wallpaper…" showed
        // Appearance
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                if let Some(page) = page_arg(&args) {
                    let _ = w.emit("open-page", page);
                }
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            start_page,
            shell::read_prefs,
            shell::write_prefs,
            shell::shell_version,
            shell::shell_running,
            shell::poke_shell,
            backend::read_config,
            backend::write_config,
            backend::set_conf,
            backend::remove_config,
            backend::hyprctl_query,
            backend::run_evals,
            backend::dpms_on,
            backend::hyprctl_reload,
            backend::vrr_caps,
            backend::xkb_registry,
            backend::time_info,
            backend::list_timezones,
            backend::set_timezone,
            backend::set_auto_timezone,
            backend::set_ntp,
            backend::locale_info,
            backend::set_locale,
            backend::install_packages,
            backend::ime,
            language::spell_info,
            language::komble_search,
            language::formats_info,
            language::set_formats,
            language::language_packs,
            backend::wallpaper_backend,
            backend::wallpaper_reapply,
            backend::restart_hypridle,
            backend::restart_shell,
            backend::per_window_kb,
            backend::apply_colorscheme,
            backend::default_wallpaper_dir,
            backend::list_wallpapers,
            backend::desktop_apps,
            backend::app_names,
            backend::mime_default,
            backend::mime_apps,
            backend::set_mime_default,
            backend::diagnostics,
            backend::power_info,
            backend::set_power_profile,
            backend::game_status,
            backend::game_set,
            backend::set_charge_limit,
            backend::set_kbd_backlight,
            backend::qs_ipc,
            backend::theme_tokens,
            backend::theme_scheme,
            backend::blur_available,
            backend::google_client_info,
            backend::net_status,
            backend::net_connectivity,
            backend::sync_app_installed,
            backend::open_sync_app,
            backend::wifi_set,
            backend::wifi_connect,
            backend::wired_set,
            backend::connection_set,
            backend::bt_status,
            backend::bt_power,
            backend::bt_discoverable,
            backend::bt_scan,
            backend::bt_pair,
            backend::bt_connect,
            backend::bt_disconnect,
            backend::bt_trust,
            backend::bt_forget,
            backend::vpn_info,
            backend::vpn_set_secrets,
            backend::vpn_add_l2tp,
            backend::vpn_import,
            backend::net_connections,
            backend::net_conn_details,
            backend::net_conn_apply,
            backend::net_conn_forget,
            backend::hostname_get,
            backend::hostname_set,
            backend::save_avatar,
            backend::avatar_from_url,
            backend::set_real_name,
            backend::user_info,
            addons::addons_state,
            addons::open_addons,
        ])
        .run(tauri::generate_context!())
        .expect("error while running ewe Settings");
}

#[cfg(test)]
mod tests {
    use super::page_arg;

    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn page_arg_reads_both_spellings() {
        assert_eq!(
            page_arg(&v(&["ewe-settings", "--page", "wallpaper"])).as_deref(),
            Some("wallpaper")
        );
        assert_eq!(
            page_arg(&v(&["ewe-settings", "--page=displays"])).as_deref(),
            Some("displays")
        );
    }

    #[test]
    fn page_arg_none_when_missing_or_empty() {
        assert_eq!(page_arg(&v(&["ewe-settings"])), None);
        assert_eq!(page_arg(&v(&["ewe-settings", "--page"])), None);
        assert_eq!(page_arg(&v(&["ewe-settings", "--page="])), None);
    }
}
