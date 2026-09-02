mod backend;
mod shell;
mod util;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
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
            backend::wallpaper_backend,
            backend::wallpaper_reapply,
            backend::restart_hypridle,
            backend::per_window_kb,
            backend::apply_colorscheme,
            backend::default_wallpaper_dir,
            backend::list_wallpapers,
            backend::desktop_apps,
            backend::mime_default,
            backend::mime_apps,
            backend::set_mime_default,
            backend::diagnostics,
            backend::power_info,
            backend::set_power_profile,
            backend::set_charge_limit,
            backend::set_kbd_backlight,
            backend::qs_ipc,
            backend::net_status,
            backend::net_connectivity,
            backend::wifi_set,
            backend::wifi_connect,
            backend::connection_set,
            backend::save_avatar,
            backend::avatar_from_url,
            backend::set_real_name,
            backend::user_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Settings");
}
