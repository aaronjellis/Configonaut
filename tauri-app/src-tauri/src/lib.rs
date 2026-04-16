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
mod installer;
mod models;
mod paths;
pub mod sidecar;

use tauri::menu::{MenuBuilder, MenuItem, SubmenuBuilder};
use tauri::{Emitter, Manager};

// How long the splash window stays up before we tear it down and show
// the main window. Short enough that nobody feels nagged, long enough
// that the neon spinner actually reads as "loading" instead of a flash.
const SPLASH_DURATION_MS: u64 = 5000;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // ── Native application menu ─────────────────────────────
            let about_item = MenuItem::with_id(
                app,
                "about_configonaut",
                "About Configonaut",
                true,
                None::<&str>,
            )?;
            let check_updates = MenuItem::with_id(
                app,
                "check_for_updates",
                "Check for Updates\u{2026}",
                true,
                None::<&str>,
            )?;

            let edit_sub = SubmenuBuilder::new(app, "Edit")
                .undo()
                .redo()
                .separator()
                .cut()
                .copy()
                .paste()
                .select_all()
                .build()?;

            let window_sub = SubmenuBuilder::new(app, "Window")
                .minimize()
                .separator()
                .close_window()
                .build()?;

            #[cfg(target_os = "macos")]
            let menu = {
                let app_sub = SubmenuBuilder::new(app, "Configonaut")
                    .item(&about_item)
                    .item(&check_updates)
                    .separator()
                    .services()
                    .separator()
                    .hide()
                    .hide_others()
                    .show_all()
                    .separator()
                    .quit()
                    .build()?;

                MenuBuilder::new(app)
                    .item(&app_sub)
                    .item(&edit_sub)
                    .item(&window_sub)
                    .build()?
            };

            #[cfg(not(target_os = "macos"))]
            let menu = {
                let help_sub = SubmenuBuilder::new(app, "Help")
                    .item(&about_item)
                    .item(&check_updates)
                    .build()?;

                MenuBuilder::new(app)
                    .item(&edit_sub)
                    .item(&window_sub)
                    .item(&help_sub)
                    .build()?
            };

            app.set_menu(menu)?;

            app.on_menu_event(|app_handle, event| {
                match event.id().as_ref() {
                    "about_configonaut" => {
                        let _ = app_handle.emit("show-about", ());
                    }
                    "check_for_updates" => {
                        let _ = app_handle.emit("check-for-updates", ());
                    }
                    _ => {}
                }
            });

            // ── Two-window boot ─────────────────────────────────────
            // `splash` is created visible by tauri.conf.json and `main`
            // starts hidden. After SPLASH_DURATION_MS we close the splash
            // and reveal main. We do the wait on a background thread so
            // the Tauri event loop keeps running and the splash animation
            // stays smooth; the actual window mutations hop back to the
            // main thread via `run_on_main_thread` because window handles
            // are not Send-safe on all platforms.
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
            // Custom feeds
            commands::list_feeds,
            commands::add_feed,
            commands::remove_feed,
            commands::toggle_feed,
            commands::get_catalog_with_feeds,
            commands::refresh_all_feeds,
            // Auto-install
            installer::check_runtime,
            installer::install_runtime,
            installer::inspect_install,
            installer::install_server,
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
            commands::create_hook,
            commands::delete_hook,
            // Agents
            commands::list_agents,
            commands::create_agent,
            commands::delete_agent,
            // Skills
            commands::list_skills,
            commands::toggle_skill,
            commands::create_skill,
            commands::delete_skill,
            // Shared: plugin toggle + raw file I/O
            commands::toggle_plugin,
            commands::read_claude_file,
            commands::write_claude_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
