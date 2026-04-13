// Tauri command surface — the thin bridge between the React frontend
// (via `invoke()`) and our stateless Rust helpers in `config.rs`.
//
// Every command here follows the same pattern: take an `AppMode` (and
// whatever other parameters are needed), call into `config` / `paths`, and
// return either a serializable payload or `AppResult<()>`. Errors bubble up
// through `AppError`, which serializes to a plain string on the JS side.
//
// Keeping commands 1-to-1 with user-facing actions makes it easy to audit
// exactly what the frontend can do to the user's disk, and to add new
// capabilities without touching unrelated code.

use std::collections::HashMap;

use serde_json::{Map, Value};

use crate::catalog::{self, Catalog};
use crate::claude_code;
use crate::config;
use crate::models::{
    AgentEntry, AppMode, AppResult, BackupFile, HookRule, ServerListing, ServerSource,
    SkillEntry, SkillSource,
};
use crate::paths;

// ---------------------------------------------------------------------------
// MCP Servers
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_servers(mode: AppMode) -> AppResult<ServerListing> {
    config::list_servers(mode)
}

/// Parse a free-form JSON snippet the way the Swift `parseInput` function
/// did: accept either `{"mcpServers": { "name": {...} }}`, a bare
/// `{"name": {...}}`, or a single server body `{ "command": ... }` (in
/// which case we require a separate name).
///
/// Matches the Swift `classify` error messages so users see the same
/// guidance they did in the native app:
///
///   • "Nothing to parse."          — empty input
///   • "Invalid JSON. Check for …"  — parse failure
///   • "mcpServers is empty."       — wrapper present but inner map empty
///   • "No valid server configs found." — bare map with no object values
///   • "this JSON is a single server body — provide a name" — bare `{command: …}`
///     with no fallback name
///
/// In addition to shape-checking, we also validate each parsed entry has
/// at least one of `command`/`url` so a bare `{"foo": {}}` can't sneak
/// through as a "server" that Claude will error on at startup.
#[tauri::command]
pub fn parse_server_input(
    raw_json: String,
    fallback_name: Option<String>,
) -> AppResult<Vec<(String, Value)>> {
    let trimmed = raw_json.trim();
    if trimmed.is_empty() {
        return Err(anyhow::anyhow!("Nothing to parse.").into());
    }
    let parsed: Value = serde_json::from_str(trimmed).map_err(|_| {
        anyhow::anyhow!(
            "Invalid JSON. Check for trailing commas, missing quotes, or extra braces."
        )
    })?;

    let root = match parsed {
        Value::Object(m) => m,
        _ => {
            return Err(
                anyhow::anyhow!("top-level must be a JSON object").into(),
            );
        }
    };

    // Case 1: { "mcpServers": { ... } }
    if let Some(Value::Object(inner)) = root.get("mcpServers") {
        let entries: Vec<(String, Value)> = inner
            .iter()
            .filter(|(_, v)| v.is_object())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        if entries.is_empty() {
            return Err(anyhow::anyhow!("mcpServers is empty.").into());
        }
        validate_server_entries(&entries)?;
        return Ok(entries);
    }

    // Case 2: looks like a single server body (has `command` or `url` or `type`)
    let looks_like_single_body = root.contains_key("command")
        || root.contains_key("url")
        || root.contains_key("type");
    if looks_like_single_body {
        let Some(name) = fallback_name.filter(|s| !s.trim().is_empty()) else {
            return Err(
                anyhow::anyhow!("this JSON is a single server body — provide a name").into(),
            );
        };
        let entries = vec![(name, Value::Object(root))];
        validate_server_entries(&entries)?;
        return Ok(entries);
    }

    // Case 3: bare map of name → config — keep only object-valued children,
    // mirroring Swift's `compactMap { $0 as? [String: Any] }`.
    let entries: Vec<(String, Value)> = root
        .into_iter()
        .filter(|(_, v)| v.is_object())
        .collect();
    if entries.is_empty() {
        return Err(anyhow::anyhow!("No valid server configs found.").into());
    }
    validate_server_entries(&entries)?;
    Ok(entries)
}

/// Shape-check each parsed entry: the config block must be a JSON object
/// with at least one of `command` (stdio) or `url` (http) so we don't hand
/// Claude a body it will choke on at startup. Errors list *every* offender
/// so the user can fix a whole paste in one go instead of playing whack-a-mole.
fn validate_server_entries(entries: &[(String, Value)]) -> AppResult<()> {
    let mut bad: Vec<String> = Vec::new();
    for (name, config) in entries {
        let Value::Object(map) = config else {
            bad.push(format!("\"{name}\" is not a JSON object"));
            continue;
        };
        let has_command = map
            .get("command")
            .and_then(|v| v.as_str())
            .is_some_and(|s| !s.trim().is_empty());
        let has_url = map
            .get("url")
            .and_then(|v| v.as_str())
            .is_some_and(|s| !s.trim().is_empty());
        if !has_command && !has_url {
            bad.push(format!(
                "\"{name}\" is missing both `command` and `url`"
            ));
        }
    }
    if !bad.is_empty() {
        return Err(anyhow::anyhow!(bad.join("; ")).into());
    }
    Ok(())
}

#[tauri::command]
pub fn add_servers_to_active(
    mode: AppMode,
    entries: Vec<(String, Value)>,
) -> AppResult<()> {
    config::add_to_active(mode, entries)
}

#[tauri::command]
pub fn add_servers_to_stored(
    mode: AppMode,
    entries: Vec<(String, Value)>,
) -> AppResult<()> {
    config::add_to_stored(mode, entries)
}

#[tauri::command]
pub fn move_server_to_stored(mode: AppMode, name: String) -> AppResult<()> {
    config::move_to_stored(mode, &name)
}

/// Turn a stored server on, but only if it's actually ready to run.
///
/// Mirrors the Swift `moveToActiveIfReady(name:catalog:)` gate: when the
/// server was originally installed from the catalog, we know which env
/// Promote a stored server to active. If the validator detects possibly-
/// unfilled env vars, a warning string is returned but the server is
/// **still turned on**. We can't reliably distinguish a real token from
/// a placeholder, so we warn and let the user decide.
///
/// Servers that have no catalog link (e.g. pasted manually) are
/// promoted unconditionally.
#[tauri::command]
pub fn move_server_to_active(mode: AppMode, name: String) -> AppResult<String> {
    let missing = stored_server_missing_secrets(mode, &name)?;
    config::move_to_active(mode, &name)?;
    if !missing.is_empty() {
        Ok(format!(
            "Warning: {} may still be placeholder values. The server was turned on — double-check if it doesn't connect.",
            missing.join(", ")
        ))
    } else {
        Ok(String::new())
    }
}

/// Look up the stored config for `name`, find its catalog link (if any),
/// and return the list of required env vars whose current value still
/// looks like a placeholder. An empty vec means either the server has no
/// catalog link at all, or every required secret is filled in.
fn stored_server_missing_secrets(
    mode: AppMode,
    name: &str,
) -> AppResult<Vec<String>> {
    // Catalog link — if none, we can't gate and just return "ready".
    let links = catalog::load_catalog_links(mode)?;
    let Some(catalog_id) = links.get(name).and_then(|v| v.as_str()) else {
        return Ok(Vec::new());
    };

    // Find the stored server's config block. We only care about stored
    // servers here because that's where the "Turn On" flow lives — active
    // servers are already ready by definition.
    let listing = config::list_servers(mode)?;
    let Some(stored) = listing
        .stored_servers
        .iter()
        .find(|s| s.name == name)
    else {
        return Ok(Vec::new());
    };
    let config_value: Value = serde_json::from_str(&stored.config_json)
        .unwrap_or(Value::Object(Map::new()));

    // Bootstrap the catalog to find the matching server entry. If the
    // catalog id was removed upstream (e.g. rename), fall through to
    // "ready" rather than hard-blocking — the user can still edit the
    // JSON directly.
    let catalog = catalog::bootstrap_catalog()?;
    let Some(server) = catalog.servers.iter().find(|s| s.id == catalog_id)
    else {
        return Ok(Vec::new());
    };

    Ok(catalog::missing_secrets(&config_value, server))
}

#[tauri::command]
pub fn delete_server(
    mode: AppMode,
    name: String,
    source: ServerSource,
) -> AppResult<()> {
    config::delete_server(mode, &name, source)
}

#[tauri::command]
pub fn update_server_config(
    mode: AppMode,
    name: String,
    source: ServerSource,
    new_json: String,
) -> AppResult<()> {
    config::update_server_config(mode, &name, source, &new_json)
}

// ---------------------------------------------------------------------------
// Backups
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_backups(mode: AppMode) -> AppResult<Vec<BackupFile>> {
    config::list_backups(mode)
}

#[tauri::command]
pub fn restore_backup(mode: AppMode, backup_path: String) -> AppResult<()> {
    config::restore_backup(mode, &backup_path)
}

#[tauri::command]
pub fn delete_backup(backup_path: String) -> AppResult<()> {
    config::delete_backup(&backup_path)
}

#[tauri::command]
pub fn force_backup(mode: AppMode) -> AppResult<()> {
    config::backup_config(mode)
}

/// Read a backup (or the live config) from disk as a UTF-8 string. Used by
/// the Backups view to render a preview and compute an added/removed diff.
///
/// Restricted to paths inside our storage directory **or** the known config
/// paths for each mode, so we can't be tricked into reading arbitrary files.
#[tauri::command]
pub fn read_backup_content(path: String) -> AppResult<String> {
    use std::path::Path;
    let p = Path::new(&path);
    let canonical = std::fs::canonicalize(p)
        .map_err(|e| anyhow::anyhow!("could not resolve path: {e}"))?;

    // Allow: inside any of our backup dirs, or one of the two live config
    // files. Everything else is rejected.
    let backup_roots = [paths::backup_dir(AppMode::Desktop), paths::backup_dir(AppMode::Cli)];
    let config_files = [paths::config_file(AppMode::Desktop), paths::config_file(AppMode::Cli)];

    let inside_backup = backup_roots.iter().any(|root| {
        std::fs::canonicalize(root)
            .map(|r| canonical.starts_with(&r))
            .unwrap_or(false)
    });
    let is_config_file = config_files.iter().any(|f| {
        std::fs::canonicalize(f)
            .map(|c| c == canonical)
            .unwrap_or(false)
    });

    if !inside_backup && !is_config_file {
        return Err(anyhow::anyhow!("path not allowed: must be a backup or config file").into());
    }

    let text = std::fs::read_to_string(&canonical)
        .map_err(|e| anyhow::anyhow!("could not read file: {e}"))?;
    Ok(text)
}

// ---------------------------------------------------------------------------
// Marketplace catalog
// ---------------------------------------------------------------------------

/// Bootstrap the catalog from cache (or embedded baseline). Returns instantly
/// — the frontend calls this on mount to populate the Marketplace tab, then
/// fires `refresh_catalog` in the background to pull the latest from GitHub.
#[tauri::command]
pub fn get_catalog() -> AppResult<Catalog> {
    catalog::bootstrap_catalog()
}

/// Pull the latest catalog from GitHub and persist it to the local cache.
/// This is `async` because reqwest is async and Tauri commands that await
/// must be too — blocking in a sync command would lock up the tokio runtime.
#[tauri::command]
pub async fn refresh_catalog() -> AppResult<Catalog> {
    catalog::refresh_catalog_remote().await
}

/// Install a server from the catalog. `custom_config` is the user-edited JSON
/// snippet from the inline editor — if they left it alone, the frontend can
/// pass `null` and we'll fall back to the catalog template.
///
/// Returns the final server name (may differ from the catalog id if a name
/// collision was resolved by appending -2, -3, …).
#[tauri::command]
pub fn install_from_catalog(
    mode: AppMode,
    catalog_id: String,
    target: ServerSource,
    custom_config: Option<Value>,
    custom_name: Option<String>,
) -> AppResult<String> {
    let custom_map = match custom_config {
        Some(Value::Object(m)) => Some(m),
        Some(_) => {
            return Err(
                anyhow::anyhow!("custom_config must be a JSON object").into()
            );
        }
        None => None,
    };
    catalog::install_from_catalog(mode, &catalog_id, target, custom_map, custom_name)
}

/// Return the per-mode map of server-name → catalog-id so the frontend can
/// decorate the server list (checkmark "Installed" pill on catalog rows that
/// are already in the user's active/stored sets).
#[tauri::command]
pub fn get_catalog_links(mode: AppMode) -> AppResult<HashMap<String, String>> {
    let links = catalog::load_catalog_links(mode)?;
    Ok(links
        .into_iter()
        .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_string())))
        .collect())
}

/// Given a server config block and a catalog id, report which required env
/// vars still look like placeholders. Used by the MCP Servers view to gate
/// the "Turn On" button.
#[tauri::command]
pub fn missing_secrets_for_server(
    catalog_id: String,
    config_json: String,
) -> AppResult<Vec<String>> {
    let config_value: Value = serde_json::from_str(&config_json)
        .map_err(|e| anyhow::anyhow!("invalid config JSON: {e}"))?;
    // We expect the caller to pass the server's config block (i.e. the value
    // inside `mcpServers["name"]`) — which is always a JSON object. Anything
    // else gets treated as an empty object so `missing_secrets` simply reports
    // every required var as missing, which is the safest fallback.
    let block = if config_value.is_object() {
        config_value
    } else {
        Value::Object(Map::new())
    };

    let catalog = catalog::bootstrap_catalog()?;
    let server = catalog
        .servers
        .iter()
        .find(|s| s.id == catalog_id)
        .ok_or_else(|| anyhow::anyhow!("catalog id not found: {catalog_id}"))?;

    Ok(catalog::missing_secrets(&block, server))
}

// ---------------------------------------------------------------------------
// Runtime prerequisite detection
// ---------------------------------------------------------------------------

/// Probe the user's PATH for runtimes that stdio MCP servers depend on.
/// Returns a struct with the detected version string for each runtime, or
/// `None` if not found. This is intentionally fire-and-forget — we never
/// block an install, just surface a warning.
#[tauri::command]
pub fn check_runtime() -> crate::models::RuntimeStatus {
    use std::process::Command;

    fn probe(cmd: &str, args: &[&str]) -> Option<String> {
        Command::new(cmd)
            .args(args)
            .output()
            .ok()
            .and_then(|out| {
                if out.status.success() {
                    let raw = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if raw.is_empty() { None } else { Some(raw) }
                } else {
                    None
                }
            })
    }

    crate::models::RuntimeStatus {
        node: probe("node", &["--version"]),
        python: probe("python3", &["--version"])
            .or_else(|| probe("python", &["--version"])),
        uv: probe("uv", &["--version"]),
        docker: probe("docker", &["--version"]),
    }
}

// ---------------------------------------------------------------------------
// Paths (debug helpers used by the UI to display "editing → /path/to/file")
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_config_path(mode: AppMode) -> String {
    paths::config_file(mode).to_string_lossy().into_owned()
}

#[tauri::command]
pub fn get_storage_dir() -> String {
    paths::storage_dir().to_string_lossy().into_owned()
}

#[tauri::command]
pub fn get_claude_code_settings_path() -> String {
    paths::claude_code_settings()
        .to_string_lossy()
        .into_owned()
}

/// Base directory containing user-authored slash commands.
/// We ensure it exists before returning so `revealItemInDir` from the
/// frontend can actually open it in Finder/Explorer — revealing a
/// non-existent path silently fails on macOS.
#[tauri::command]
pub fn get_commands_dir() -> AppResult<String> {
    let dir = paths::commands_dir();
    std::fs::create_dir_all(&dir)?;
    Ok(dir.to_string_lossy().into_owned())
}

/// Base directory containing user-authored skills. Same guarantees as
/// `get_commands_dir` — we `mkdir -p` so the caller can reveal it.
#[tauri::command]
pub fn get_skills_dir() -> AppResult<String> {
    let dir = paths::skills_dir();
    std::fs::create_dir_all(&dir)?;
    Ok(dir.to_string_lossy().into_owned())
}

/// Quit and relaunch Claude Desktop so config changes take effect
/// without the user having to cmd-Q and re-open manually. macOS only —
/// on other platforms this is a no-op that returns an error the UI can
/// display. The approach is:
///
/// macOS:
///   1. Ask Claude to quit via AppleScript (graceful — gives the app a
///      chance to clean up). If the app isn't running, osascript errors
///      silently and we just go straight to step 3.
///   2. Sleep briefly so the process actually exits before we try to
///      launch a fresh copy. Without the delay, `open -a` sees the
///      still-terminating instance and foregrounds it instead of
///      starting a new one.
///   3. `open -a Claude` to launch (or foreground) the app.
///
/// Windows:
///   1. `taskkill /IM Claude.exe /F` to end any running instance. The
///      /F is deliberate — Windows has no AppleScript equivalent and
///      asking for a graceful quit is more hassle than it's worth here;
///      Claude Desktop saves its config through our IPC, so there's
///      nothing in-flight to lose. This works for both the MSI and MSIX
///      builds — they both surface as `Claude.exe` in the process list.
///   2. Poll `tasklist` until Claude.exe is actually gone so we don't
///      race the new launch against a still-exiting instance (mirrors
///      the macOS `pgrep` loop for the same reason).
///   3. Relaunch. There are two install shapes in the wild:
///      - **Legacy MSI** at `%LOCALAPPDATA%\AnthropicClaude\Claude.exe`.
///        We try this first because it's cheap (a single file-exists
///        check) and can be spawned directly.
///      - **MSIX / Store** under `C:\Program Files\WindowsApps\...`. That
///        directory is ACL-locked to TrustedInstaller, so we can't spawn
///        the exe by path even if we could find it. The canonical way to
///        launch an MSIX app is via `explorer.exe shell:AppsFolder\<AUMID>`
///        — the same mechanism the Start menu uses. The App User Model ID
///        looks like `Claude_<publisherHash>!Claude`, and the publisher
///        hash varies per install, so we discover it at runtime with
///        PowerShell's `Get-StartApps`.
///      If neither lookup finds anything, we return an error pointing at
///      both paths so the user knows what was checked.
///
/// Both platforms park the command-handler thread while polling, which
/// is fine — Tauri runs each command on its own task so the UI thread
/// is unaffected.
#[tauri::command]
pub fn restart_claude_desktop() -> AppResult<()> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        use std::thread;
        use std::time::Duration;

        // Quit. Ignore the exit status — a "not running" error here is
        // exactly the case where we still want to proceed to launch.
        let _ = Command::new("osascript")
            .args(["-e", "tell application \"Claude\" to quit"])
            .status();

        // Poll until Claude is actually gone. A fixed sleep here isn't
        // reliable: a quitting app can take anywhere from 200ms (empty
        // state) to several seconds (flushing config + unregistering
        // from Launch Services) to fully exit. If we call `open -a` too
        // early, macOS sees the still-exiting instance and either
        // silently no-ops or tries to foreground the dying window —
        // which the user experiences as "it quit but never restarted".
        //
        // We check for a running Claude process with `pgrep` every
        // 100ms for up to ~4 seconds, then proceed either way. An
        // initial 200ms head-start lets the quit event actually
        // dispatch before the first check.
        thread::sleep(Duration::from_millis(200));
        for _ in 0..38 {
            let still_running = Command::new("pgrep")
                .args(["-x", "Claude"])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if !still_running {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
        // Extra beat so Launch Services has a moment to clear the old
        // registration even after the process is gone. Without this
        // `open -a` will occasionally still race and silently no-op.
        thread::sleep(Duration::from_millis(250));

        Command::new("open")
            .args(["-a", "Claude"])
            .status()
            .map_err(|e| anyhow::anyhow!("launch Claude: {}", e))?;
        Ok(())
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        use std::path::PathBuf;
        use std::process::Command;
        use std::thread;
        use std::time::Duration;

        // CREATE_NO_WINDOW (0x08000000) suppresses the transient console
        // window that would otherwise flash on screen every time we spawn
        // a console subprocess from this GUI binary. Without it, taskkill,
        // tasklist, and powershell each briefly pop a black cmd window,
        // which is disruptive and makes "Restart Claude Desktop" feel
        // broken even when it's working. The flag has no effect on GUI
        // apps (Claude.exe, explorer.exe), so we only apply it to the
        // console helpers below.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;

        // Step 1 — terminate any running instance. We don't care about
        // the exit code: a "process not found" error means Claude wasn't
        // running, which is fine.
        let _ = Command::new("taskkill")
            .creation_flags(CREATE_NO_WINDOW)
            .args(["/IM", "Claude.exe", "/F"])
            .status();

        // Step 2 — poll `tasklist` until Claude.exe is no longer listed
        // before we relaunch. `taskkill /F` *requests* termination but
        // doesn't block until the process actually exits, and launching
        // a fresh instance while the old one is still releasing file
        // locks has the same "quit but never restarted" failure mode as
        // the macOS race. We check every 100ms for up to ~4 seconds,
        // then proceed either way.
        //
        // `tasklist /FI "IMAGENAME eq Claude.exe" /NH` prints a header
        // line when nothing matches ("INFO: No tasks are running…") and
        // a process row when something does, so we decide "still
        // running" by looking for "Claude.exe" in the stdout bytes.
        thread::sleep(Duration::from_millis(200));
        for _ in 0..38 {
            let output = Command::new("tasklist")
                .creation_flags(CREATE_NO_WINDOW)
                .args(["/FI", "IMAGENAME eq Claude.exe", "/NH"])
                .output();
            let still_running = match output {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    stdout.contains("Claude.exe")
                }
                // If tasklist itself fails we fall through rather than
                // looping forever — better to attempt the launch than to
                // hang the command.
                Err(_) => false,
            };
            if !still_running {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
        // Small trailing beat so Windows finishes releasing file handles
        // on the installed executable before we spawn it again.
        thread::sleep(Duration::from_millis(250));

        // Step 3a — try the legacy MSI install first. `%LOCALAPPDATA%`
        // is where Anthropic's classic MSI-deployed desktop app lives,
        // and we can spawn that exe directly. Failing this check is not
        // an error — it just means we fall through to the MSIX lookup.
        let local_app_data = std::env::var_os("LOCALAPPDATA").ok_or_else(|| {
            anyhow::anyhow!("LOCALAPPDATA is not set — cannot locate Claude.exe")
        })?;
        let msi_exe = PathBuf::from(&local_app_data)
            .join("AnthropicClaude")
            .join("Claude.exe");

        if msi_exe.exists() {
            Command::new(&msi_exe)
                .spawn()
                .map_err(|e| anyhow::anyhow!("launch Claude (MSI): {}", e))?;
            return Ok(());
        }

        // Step 3b — fall back to the MSIX / Store install. The exe lives
        // under `C:\Program Files\WindowsApps\Claude_<version>_x64__<hash>\`
        // which is ACL-locked to TrustedInstaller, so direct spawning is
        // off the table. Instead we ask PowerShell for the App User Model
        // ID via `Get-StartApps` (it's the same query the Start menu uses)
        // and hand that off to explorer.exe.
        //
        // Get-StartApps rows look like:
        //     Name  AppID
        //     ----  -----
        //     Claude Claude_pzs8sxrjxfjjc!Claude
        //
        // We print only the AppID column to avoid parsing the table, and
        // filter by `Name -eq 'Claude'` so we don't accidentally pick up
        // some other app whose name happens to contain "Claude".
        let ps_output = Command::new("powershell.exe")
            .creation_flags(CREATE_NO_WINDOW)
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "(Get-StartApps | Where-Object { $_.Name -eq 'Claude' } | Select-Object -First 1).AppID",
            ])
            .output()
            .map_err(|e| anyhow::anyhow!("query AppsFolder for Claude: {}", e))?;

        let aumid = String::from_utf8_lossy(&ps_output.stdout).trim().to_string();

        if aumid.is_empty() {
            return Err(anyhow::anyhow!(
                "Couldn't find Claude Desktop. Checked:\n  {}\n  Windows Start menu (Get-StartApps)\nIs Claude Desktop installed?",
                msi_exe.display()
            )
            .into());
        }

        // `explorer.exe shell:AppsFolder\<AUMID>` is the documented way to
        // launch a packaged app from outside the shell. explorer.exe exits
        // immediately after dispatching to the Store activation broker, so
        // there's no long-lived child process to manage.
        Command::new("explorer.exe")
            .arg(format!("shell:AppsFolder\\{}", aumid))
            .spawn()
            .map_err(|e| anyhow::anyhow!("launch Claude (MSIX): {}", e))?;
        Ok(())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err(anyhow::anyhow!(
            "Automatic restart is only supported on macOS and Windows right now."
        )
        .into())
    }
}

// ---------------------------------------------------------------------------
// Hooks
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_hooks() -> AppResult<Vec<HookRule>> {
    claude_code::list_hooks()
}

#[tauri::command]
pub fn get_hook_rule_json(event: String, matcher: String) -> AppResult<String> {
    claude_code::hook_rule_json(&event, &matcher)
}

#[tauri::command]
pub fn toggle_hook(event: String, matcher: String, enable: bool) -> AppResult<()> {
    claude_code::toggle_hook(&event, &matcher, enable)
}

#[tauri::command]
pub fn update_hook_rule(
    event: String,
    matcher: String,
    new_json: String,
) -> AppResult<()> {
    claude_code::update_hook_rule(&event, &matcher, &new_json)
}

// ---------------------------------------------------------------------------
// Agents
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_agents() -> AppResult<Vec<AgentEntry>> {
    claude_code::list_agents()
}

#[tauri::command]
pub fn create_agent(name: String) -> AppResult<String> {
    claude_code::create_agent(&name)
}

#[tauri::command]
pub fn delete_agent(file_path: String) -> AppResult<()> {
    claude_code::delete_agent(&file_path)
}

// ---------------------------------------------------------------------------
// Skills
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_skills() -> AppResult<Vec<SkillEntry>> {
    claude_code::list_skills()
}

#[tauri::command]
pub fn toggle_skill(
    file_path: String,
    source: SkillSource,
    currently_enabled: bool,
) -> AppResult<()> {
    claude_code::toggle_skill(&file_path, source, currently_enabled)
}

#[tauri::command]
pub fn create_skill(name: String, source: SkillSource) -> AppResult<String> {
    claude_code::create_skill(&name, source)
}

// ---------------------------------------------------------------------------
// Shared plugin toggle + raw file I/O used by Agents and Skills editors
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn toggle_plugin(plugin_key: String) -> AppResult<()> {
    claude_code::toggle_plugin(&plugin_key)
}

#[tauri::command]
pub fn read_claude_file(file_path: String) -> AppResult<String> {
    claude_code::read_claude_file(&file_path)
}

#[tauri::command]
pub fn write_claude_file(file_path: String, content: String) -> AppResult<()> {
    claude_code::write_claude_file(&file_path, &content)
}
