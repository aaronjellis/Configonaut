// Core ConfigManager logic, ported from the Swift original.
//
// The Swift version was class-based with `@Published` state; here we're
// stateless — each command reads the relevant file from disk, mutates, writes
// back, and lets the frontend re-fetch. This matches Tauri's command model
// and avoids having to thread a shared state across every handler.
//
// Three JSON files are touched, depending on mode:
//
//   Claude Desktop mode:
//     active  → ~/Library/Application Support/Claude/claude_desktop_config.json
//     stored  → ~/Library/Application Support/Configonaut/stored_servers_desktop.json
//
//   Claude Code mode:
//     active  → ~/.claude.json                        (mcpServers key — `claude mcp add`)
//     stored  → ~/Library/Application Support/Configonaut/stored_servers_cli.json
//
// "Active" means the entry is present in Claude's actual config. "Stored" is
// Configonaut's parking lot for configured-but-off servers — same shape, just
// persisted in a sidecar so turning a server off doesn't lose its env vars.

use std::fs;
use std::path::Path;

use anyhow::{anyhow, Context};
use chrono::Local;
use serde_json::{Map, Value};

use crate::models::{AppMode, AppResult, ProjectMcpGroup, ServerEntry, ServerListing, ServerSource};
use crate::paths;

// ---------------------------------------------------------------------------
// Read path — snapshot the full MCP listing for a mode
// ---------------------------------------------------------------------------

pub fn list_servers(mode: AppMode) -> AppResult<ServerListing> {
    let config_path = paths::config_file(mode);
    let active_map = load_mcp_servers_from_config(&config_path)?;
    let stored_map = load_stored_map(mode)?;

    // Stored entries shouldn't shadow active ones — if the same name exists
    // in both, the active version is canonical and the stored copy is hidden.
    let active_names: std::collections::HashSet<String> =
        active_map.keys().cloned().collect();

    let mut active_servers: Vec<ServerEntry> = active_map
        .into_iter()
        .map(|(name, config)| ServerEntry {
            name,
            config_json: pretty_json(&config),
        })
        .collect();
    active_servers.sort_by(|a, b| a.name.cmp(&b.name));

    let mut stored_servers: Vec<ServerEntry> = stored_map
        .into_iter()
        .filter(|(name, _)| !active_names.contains(name))
        .map(|(name, config)| ServerEntry {
            name,
            config_json: pretty_json(&config),
        })
        .collect();
    stored_servers.sort_by(|a, b| a.name.cmp(&b.name));

    // CLI mode: also extract project-scoped MCPs from the `projects` key.
    let project_groups = if mode == AppMode::Cli {
        load_project_mcp_groups(&config_path)?
    } else {
        Vec::new()
    };

    Ok(ServerListing {
        active_servers,
        stored_servers,
        config_path: config_path.to_string_lossy().into_owned(),
        needs_restart: false, // The frontend tracks this locally after mutations.
        project_groups,
    })
}

/// Extract project-scoped MCP servers from `~/.claude.json`.
/// Structure: `{ "projects": { "/path/to/project": { "mcpServers": { ... } } } }`
fn load_project_mcp_groups(config_path: &Path) -> AppResult<Vec<ProjectMcpGroup>> {
    let root = load_config_root(config_path)?;
    let Some(Value::Object(projects)) = root.get("projects") else {
        return Ok(Vec::new());
    };

    let mut groups = Vec::new();
    for (project_path, project_data) in projects {
        let Value::Object(proj_obj) = project_data else { continue };
        let Some(Value::Object(mcp_servers)) = proj_obj.get("mcpServers") else { continue };
        if mcp_servers.is_empty() {
            continue;
        }

        let mut servers: Vec<ServerEntry> = mcp_servers
            .iter()
            .map(|(name, config)| ServerEntry {
                name: name.clone(),
                config_json: pretty_json(config),
            })
            .collect();
        servers.sort_by(|a, b| a.name.cmp(&b.name));

        groups.push(ProjectMcpGroup {
            project_path: project_path.clone(),
            servers,
        });
    }
    groups.sort_by(|a, b| a.project_path.cmp(&b.project_path));
    Ok(groups)
}

// ---------------------------------------------------------------------------
// Write path — mutations
// ---------------------------------------------------------------------------

/// Install one or more servers into the active Claude config.
pub fn add_to_active(
    mode: AppMode,
    entries: Vec<(String, Value)>,
) -> AppResult<()> {
    let config_path = paths::config_file(mode);
    let mut root = load_config_root(&config_path)?;
    let mcp = ensure_mcp_map(&mut root);
    for (name, config) in entries {
        mcp.insert(name, config);
    }
    backup_config(mode)?;
    save_config_root(&config_path, &root)?;
    Ok(())
}

/// Park one or more servers in Configonaut's inactive sidecar file.
pub fn add_to_stored(
    mode: AppMode,
    entries: Vec<(String, Value)>,
) -> AppResult<()> {
    let mut stored = load_stored_map(mode)?;
    for (name, config) in entries {
        stored.insert(name, config);
    }
    save_stored_map(mode, &stored)?;
    Ok(())
}

/// Turn off a server: pull it from the live Claude config and park it in
/// the stored sidecar. Keeps the env vars intact so flipping it back on is
/// a no-op.
pub fn move_to_stored(mode: AppMode, name: &str) -> AppResult<()> {
    let config_path = paths::config_file(mode);
    let mut root = load_config_root(&config_path)?;
    let mcp = ensure_mcp_map(&mut root);
    let Some(config) = mcp.shift_remove(name) else {
        return Err(anyhow!("server {name} is not active").into());
    };

    let mut stored = load_stored_map(mode)?;
    stored.insert(name.to_string(), config);

    backup_config(mode)?;
    save_config_root(&config_path, &root)?;
    save_stored_map(mode, &stored)?;
    Ok(())
}

/// Turn on a server: pull it from the stored sidecar and drop it into the
/// live Claude config.
pub fn move_to_active(mode: AppMode, name: &str) -> AppResult<()> {
    let mut stored = load_stored_map(mode)?;
    let Some(config) = stored.shift_remove(name) else {
        return Err(anyhow!("server {name} is not in the stored list").into());
    };

    let config_path = paths::config_file(mode);
    let mut root = load_config_root(&config_path)?;
    let mcp = ensure_mcp_map(&mut root);
    mcp.insert(name.to_string(), config);

    backup_config(mode)?;
    save_config_root(&config_path, &root)?;
    save_stored_map(mode, &stored)?;
    Ok(())
}

/// Permanently delete a server from either the active config or the stored
/// sidecar (or both).
pub fn delete_server(
    mode: AppMode,
    name: &str,
    source: ServerSource,
) -> AppResult<()> {
    match source {
        ServerSource::Active => {
            let config_path = paths::config_file(mode);
            let mut root = load_config_root(&config_path)?;
            let mcp = ensure_mcp_map(&mut root);
            mcp.shift_remove(name);
            backup_config(mode)?;
            save_config_root(&config_path, &root)?;
        }
        ServerSource::Stored => {
            let mut stored = load_stored_map(mode)?;
            stored.shift_remove(name);
            save_stored_map(mode, &stored)?;
        }
    }
    Ok(())
}

/// Replace the JSON config for a single server in place.
/// Auto-unwraps `{ "mcpServers": { ... } }` wrappers that users commonly paste.
pub fn update_server_config(
    mode: AppMode,
    name: &str,
    source: ServerSource,
    new_json: &str,
) -> AppResult<()> {
    let new_value: Value = serde_json::from_str(new_json)
        .context("config JSON is invalid")?;

    // Unwrap `{ "mcpServers": { "name": { ... } } }` if the user pasted the
    // full Claude config snippet instead of just the server body.
    let unwrapped = unwrap_mcp_wrapper(&new_value, name);

    match source {
        ServerSource::Active => {
            let config_path = paths::config_file(mode);
            let mut root = load_config_root(&config_path)?;
            let mcp = ensure_mcp_map(&mut root);
            mcp.insert(name.to_string(), unwrapped);
            backup_config(mode)?;
            save_config_root(&config_path, &root)?;
        }
        ServerSource::Stored => {
            let mut stored = load_stored_map(mode)?;
            stored.insert(name.to_string(), unwrapped);
            save_stored_map(mode, &stored)?;
        }
    }
    Ok(())
}

/// If `value` looks like `{ "mcpServers": { ... } }`, extract the inner
/// server config. Tries matching by `name` first, then falls back to the
/// single entry if there's only one.
fn unwrap_mcp_wrapper(value: &Value, name: &str) -> Value {
    let Some(obj) = value.as_object() else { return value.clone() };
    let Some(Value::Object(inner)) = obj.get("mcpServers") else { return value.clone() };

    // Only unwrap if "mcpServers" is the sole top-level key (it's a wrapper, not a real config).
    if obj.len() != 1 { return value.clone(); }

    if let Some(server_config) = inner.get(name) {
        return server_config.clone();
    }
    if inner.len() == 1 {
        if let Some((_, server_config)) = inner.iter().next() {
            return server_config.clone();
        }
    }
    value.clone()
}

// ---------------------------------------------------------------------------
// Internal I/O primitives
// ---------------------------------------------------------------------------

/// Load the root JSON object from Claude's config file. Returns an empty
/// object if the file is missing or empty — Claude treats a missing file the
/// same as an empty config, so we match that.
fn load_config_root(path: &Path) -> AppResult<Map<String, Value>> {
    if !path.exists() {
        return Ok(Map::new());
    }
    let raw = fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))?;
    if raw.trim().is_empty() {
        return Ok(Map::new());
    }
    let parsed: Value = serde_json::from_str(&raw)
        .with_context(|| format!("parse {}", path.display()))?;
    match parsed {
        Value::Object(map) => Ok(map),
        _ => Err(anyhow!("config root is not a JSON object").into()),
    }
}

/// Save the root JSON object to Claude's config file with pretty formatting,
/// atomically (write to temp, then rename) so a crash mid-write can't corrupt
/// the user's setup.
fn save_config_root(path: &Path, root: &Map<String, Value>) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&Value::Object(root.clone()))?;
    write_atomic(path, json.as_bytes())
}

/// Extract (or insert) the `mcpServers` sub-object. We never want Claude's
/// other config keys (`theme`, `globalShortcut`, etc.) clobbered, so we
/// mutate in place.
fn ensure_mcp_map(root: &mut Map<String, Value>) -> &mut Map<String, Value> {
    root.entry("mcpServers".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    match root.get_mut("mcpServers").unwrap() {
        Value::Object(m) => m,
        other => {
            // If the file had `mcpServers` set to something weird (array,
            // null, string), blow it away rather than panic.
            *other = Value::Object(Map::new());
            match other {
                Value::Object(m) => m,
                _ => unreachable!(),
            }
        }
    }
}

/// Read just the mcpServers sub-object from a config file, flattened into a
/// `name -> config` map. Empty map if the file or key is missing.
fn load_mcp_servers_from_config(path: &Path) -> AppResult<Map<String, Value>> {
    let root = load_config_root(path)?;
    match root.get("mcpServers") {
        Some(Value::Object(m)) => Ok(m.clone()),
        _ => Ok(Map::new()),
    }
}

/// Load the stored (inactive) servers sidecar for a mode. The sidecar is a
/// flat `{ "name": { config } }` object — no outer mcpServers wrapper.
fn load_stored_map(mode: AppMode) -> AppResult<Map<String, Value>> {
    let path = paths::stored_servers_file(mode);
    if !path.exists() {
        return Ok(Map::new());
    }
    let raw = fs::read_to_string(&path)?;
    if raw.trim().is_empty() {
        return Ok(Map::new());
    }
    match serde_json::from_str::<Value>(&raw)? {
        Value::Object(m) => Ok(m),
        _ => Ok(Map::new()),
    }
}

fn save_stored_map(mode: AppMode, stored: &Map<String, Value>) -> AppResult<()> {
    let path = paths::stored_servers_file(mode);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&Value::Object(stored.clone()))?;
    write_atomic(&path, json.as_bytes())
}

/// Write bytes to `path` atomically: stage them in a sibling temp file, then
/// rename over the destination. On POSIX this is a single syscall; on Windows
/// `fs::rename` across existing files works fine since Rust 1.5+.
fn write_atomic(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let tmp = match path.parent() {
        Some(parent) => parent.join(format!(
            ".{}.tmp",
            path.file_name().and_then(|s| s.to_str()).unwrap_or("config")
        )),
        None => path.with_extension("tmp"),
    };
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

/// Pretty-print a JSON value the way the Swift version did — sorted keys,
/// two-space indent. We sort via a recursive rewrite so `serde_json::to_string_pretty`
/// gets deterministic output.
pub fn pretty_json(value: &Value) -> String {
    serde_json::to_string_pretty(&sort_keys(value)).unwrap_or_else(|_| "{}".into())
}

fn sort_keys(value: &Value) -> Value {
    match value {
        Value::Object(m) => {
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort();
            let mut out = Map::new();
            for k in keys {
                out.insert(k.clone(), sort_keys(&m[k]));
            }
            Value::Object(out)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(sort_keys).collect()),
        other => other.clone(),
    }
}

// ---------------------------------------------------------------------------
// Backups
// ---------------------------------------------------------------------------

/// Copy the current active config to the backup directory with a timestamp.
/// Called automatically before any mutation. Keeps only the 30 most recent
/// backups per mode.
pub fn backup_config(mode: AppMode) -> AppResult<()> {
    let src = paths::config_file(mode);
    if !src.exists() {
        return Ok(());
    }
    let dir = paths::backup_dir(mode);
    fs::create_dir_all(&dir)?;

    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let dest = dir.join(format!("config_{timestamp}.json"));

    let data = fs::read(&src)?;
    fs::write(&dest, data)?;

    // Keep the last 30 backups.
    let mut files: Vec<_> = fs::read_dir(&dir)?
        .flatten()
        .filter(|e| {
            e.path()
                .extension()
                .map(|x| x == "json")
                .unwrap_or(false)
        })
        .collect();
    files.sort_by_key(|e| e.file_name());
    while files.len() > 30 {
        let oldest = files.remove(0);
        let _ = fs::remove_file(oldest.path());
    }
    Ok(())
}

/// Scan the backup directory for a mode and return its contents sorted newest-first.
pub fn list_backups(mode: AppMode) -> AppResult<Vec<crate::models::BackupFile>> {
    let dir = paths::backup_dir(mode);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(&dir)?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let metadata = match fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let created_at = metadata
            .created()
            .or_else(|_| metadata.modified())
            .map(|t| {
                let dt: chrono::DateTime<chrono::Utc> = t.into();
                dt.to_rfc3339()
            })
            .unwrap_or_else(|_| String::new());

        files.push(crate::models::BackupFile {
            file_name: path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string(),
            path: path.to_string_lossy().into_owned(),
            created_at,
            size_bytes: metadata.len(),
        });
    }
    // Newest first (filenames are timestamped so string sort descending works).
    files.sort_by(|a, b| b.file_name.cmp(&a.file_name));
    Ok(files)
}

pub fn restore_backup(mode: AppMode, backup_path: &str) -> AppResult<()> {
    let backup_path = Path::new(backup_path);
    if !backup_path.exists() {
        return Err(anyhow!("backup file not found").into());
    }
    // Validate the backup is real JSON before clobbering the live config.
    let raw = fs::read_to_string(backup_path)?;
    let _: Value = serde_json::from_str(&raw)
        .context("backup file is not valid JSON")?;

    // Take a snapshot of the current config before overwriting.
    backup_config(mode)?;

    let dest = paths::config_file(mode);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    write_atomic(&dest, raw.as_bytes())?;
    Ok(())
}

pub fn delete_backup(backup_path: &str) -> AppResult<()> {
    let p = Path::new(backup_path);
    if p.exists() {
        fs::remove_file(p)?;
    }
    Ok(())
}
