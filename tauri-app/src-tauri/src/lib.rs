// Entry point for the Tauri backend. The app is split into a handful of
// focused modules:
//
//   paths     — OS-aware path discovery for Claude + Configonaut files
//   models    — serde types shared with the React frontend
//   config    — stateless MCP server / backup read/write helpers
//   catalog   — marketplace catalog + install-from-catalog flow
//   commands  — #[tauri::command]-annotated thin wrappers over the above
//
// `run()` wires everything together and hands control to Tauri's builder.

mod catalog;
mod claude_code;
mod commands;
mod config;
mod models;
mod paths;

use tauri::Manager;

// How long the splash window stays up before we tear it down and show
// the main window. Short enough that nobody feels nagged, long enough
// that the neon spinner actually reads as "loading" instead of a flash.
const SPLASH_DURATION_MS: u64 = 5000;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Two-window boot: `splash` is created visible by tauri.conf.json
            // and `main` starts hidden. After SPLASH_DURATION_MS we close
            // the splash and reveal main. We do the wait on a background
            // thread so the Tauri event loop keeps running and the splash
            // animation stays smooth; the actual window mutations hop back
            // to the main thread via `run_on_main_thread` because window
            // handles are not Send-safe on all platforms.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(
                    SPLASH_DURATION_MS,
                ));
                // Clone the handle for the inner closure so the outer
                // `handle` is still free to serve as the receiver for
                // `run_on_main_thread` — we can't both move `handle`
                // into the closure and call a method on it in the same
                // expression.
                let handle_inner = handle.clone();
                let _ = handle.run_on_main_thread(move || {
                    if let Some(splash) = handle_inner.get_webview_window("splash") {
                        let _ = splash.close();
                    }
                    if let Some(main) = handle_inner.get_webview_window("main") {
                        let _ = main.show();
                        let _ = main.set_focus();
                    }
                });
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // MCP servers
            commands::list_servers,
            commands::parse_server_input,
            commands::add_servers_to_active,
            commands::add_servers_to_stored,
            commands::move_server_to_stored,
            commands::move_server_to_active,
            commands::delete_server,
            commands::update_server_config,
            // Backups
            commands::list_backups,
            commands::restore_backup,
            commands::delete_backup,
            commands::force_backup,
            commands::read_backup_content,
            // Marketplace
            commands::get_catalog,
            commands::refresh_catalog,
            commands::install_from_catalog,
            commands::get_catalog_links,
            commands::missing_secrets_for_server,
            // Paths
            commands::get_config_path,
            commands::get_storage_dir,
            commands::get_claude_code_settings_path,
            commands::get_commands_dir,
            commands::get_skills_dir,
            commands::restart_claude_desktop,
            // Hooks
            commands::list_hooks,
            commands::get_hook_rule_json,
            commands::toggle_hook,
            commands::update_hook_rule,
            // Agents
            commands::list_agents,
            commands::create_agent,
            commands::delete_agent,
            // Skills
            commands::list_skills,
            commands::toggle_skill,
            commands::create_skill,
            // Shared: plugin toggle + raw file I/O
            commands::toggle_plugin,
            commands::read_claude_file,
            commands::write_claude_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
