// Auto-install runner. See docs/superpowers/specs/2026-04-14-mcp-auto-install-design.md.

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tauri::AppHandle;
use tauri::Emitter;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::process::Command as TokioCommand;

use crate::catalog::{CatalogConfig, CatalogPrerequisite, CatalogServer, ConfigField,
                     ConfigFieldKind, ConfigFieldType, InstallStep, RuntimeName};
use crate::sidecar;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    pub installed: bool,
    pub version: Option<String>,
    /// `"system"` (found on PATH) or `"sidecar"` (bundled with the app).
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum InstallAction {
    /// Runtime is ready to use; no user action required.
    Ready,
    /// Open this URL in the user's browser; runtime install happens externally.
    OpenUrl { url: String },
    /// Configonaut can download and install this runtime in-app.
    Download,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InstallErrorKind {
    Network,
    DockerDaemonDown,
    DiskFull,
    Interrupted,
    Generic(i32),
}

/// Classify `(stderr, exit_code)` into a known error variant. Used to
/// produce a user-friendly message in install_server.
pub fn classify_install_error(stderr: &str, exit_code: i32) -> InstallErrorKind {
    let s = stderr.to_lowercase();
    if s.contains("network is unreachable")
        || s.contains("could not resolve host")
        || s.contains("connection refused")
        || s.contains("temporary failure in name resolution")
    {
        return InstallErrorKind::Network;
    }
    if s.contains("cannot connect to the docker daemon") {
        return InstallErrorKind::DockerDaemonDown;
    }
    if s.contains("no space left on device") {
        return InstallErrorKind::DiskFull;
    }
    if exit_code == 130 || s.contains("interrupted") || s.contains("signal") {
        return InstallErrorKind::Interrupted;
    }
    InstallErrorKind::Generic(exit_code)
}

impl InstallErrorKind {
    pub fn user_message(&self) -> String {
        match self {
            Self::Network => "Couldn't reach the registry. Check your connection and retry.".into(),
            Self::DockerDaemonDown => "Docker is installed but the daemon isn't running. Start Docker Desktop and retry.".into(),
            Self::DiskFull => "Out of disk space during install.".into(),
            Self::Interrupted => "Install was interrupted.".into(),
            Self::Generic(code) => format!("Install failed (exit code {code}). See log below."),
        }
    }
}

/// Hardcoded install URLs for runtimes we don't manage. NEVER read from
/// catalog data — that would let a malicious feed phish the user.
pub fn runtime_install_url(name: RuntimeName) -> Option<&'static str> {
    match name {
        RuntimeName::Node => Some("https://nodejs.org"),
        RuntimeName::Docker => Some("https://www.docker.com/products/docker-desktop"),
        RuntimeName::Uv => None, // bundled
    }
}

/// Build the `(program, args)` pair to detect a runtime.
pub fn check_command_for(name: RuntimeName) -> (&'static str, Vec<&'static str>) {
    let probe = if cfg!(target_os = "windows") { "where" } else { "which" };
    let bin = match name {
        RuntimeName::Node => "node",
        RuntimeName::Docker => "docker",
        RuntimeName::Uv => "uv",
    };
    (probe, vec![bin])
}

/// Build the `(program, args)` pair to query a runtime's version.
fn version_command_for(name: RuntimeName) -> (&'static str, Vec<&'static str>) {
    match name {
        RuntimeName::Node => ("node", vec!["-v"]),
        RuntimeName::Docker => ("docker", vec!["-v"]), // parse_runtime_version is calibrated to "docker -v" format
        RuntimeName::Uv => ("uv", vec!["-V"]),
    }
}

/// Pull the version string out of the runtime's `--version` output.
pub fn parse_runtime_version(name: RuntimeName, raw: &str) -> Option<String> {
    let line = raw.lines().next()?;
    match name {
        RuntimeName::Node => line.trim().strip_prefix('v').map(str::to_string),
        RuntimeName::Docker => {
            line.split_whitespace().nth(2).map(|s| s.trim_end_matches(',').to_string())
        }
        RuntimeName::Uv => {
            line.split_whitespace().nth(1).map(str::to_string)
        }
    }
}

pub fn install_runtime_for(name: RuntimeName) -> InstallAction {
    match name {
        RuntimeName::Uv => InstallAction::Ready,
        RuntimeName::Node => InstallAction::Download,
        RuntimeName::Docker => {
            match runtime_install_url(name) {
                None => InstallAction::Ready,
                Some(url) => InstallAction::OpenUrl { url: url.to_string() },
            }
        }
    }
}

#[tauri::command]
pub async fn install_runtime(name: RuntimeName) -> Result<InstallAction, String> {
    Ok(install_runtime_for(name))
}

// ---------------------------------------------------------------------------
// Managed runtimes — Node.js download + install
// ---------------------------------------------------------------------------

pub fn managed_runtimes_dir() -> PathBuf {
    crate::paths::storage_dir().join("runtimes")
}

pub fn managed_node_dir() -> PathBuf {
    managed_runtimes_dir().join("node")
}

pub fn managed_node_bin() -> Option<PathBuf> {
    let dir = managed_node_dir();
    let bin = if cfg!(target_os = "windows") {
        dir.join("node.exe")
    } else {
        dir.join("bin").join("node")
    };
    if bin.exists() { Some(bin) } else { None }
}

/// Returns the directory containing the managed node/npx binaries, if
/// installed. Callers can prepend this to PATH so Claude finds npx.
pub fn managed_node_bin_dir() -> Option<PathBuf> {
    let dir = managed_node_dir();
    let bin_dir = if cfg!(target_os = "windows") {
        dir.clone()
    } else {
        dir.join("bin")
    };
    let node = if cfg!(target_os = "windows") {
        bin_dir.join("node.exe")
    } else {
        bin_dir.join("node")
    };
    if node.exists() { Some(bin_dir) } else { None }
}

const RUNTIME_INSTALL_EVENT: &str = "runtime-install-progress";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RuntimeInstallProgress {
    Downloading { percent: f64, downloaded_bytes: u64, total_bytes: u64 },
    Extracting,
    Verifying,
    Done { version: String },
    Error { message: String },
}

fn node_download_url(version: &str) -> String {
    // Always use .tar.gz — Windows bsdtar doesn't support
    // --strip-components on .zip archives, and Node publishes
    // .tar.gz for all platforms including Windows.
    let os = if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "windows") {
        "win"
    } else {
        "linux"
    };
    let arch = if cfg!(target_arch = "aarch64") { "arm64" } else { "x64" };
    format!("https://nodejs.org/dist/{version}/node-{version}-{os}-{arch}.tar.gz")
}

#[derive(Debug, Deserialize)]
struct NodeVersionEntry {
    version: String,
    lts: serde_json::Value,
}

async fn fetch_latest_lts_version() -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let resp = client
        .get("https://nodejs.org/dist/index.json")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Node.js version list: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("nodejs.org returned HTTP {}", resp.status()));
    }

    let entries: Vec<NodeVersionEntry> = resp.json().await
        .map_err(|e| format!("Failed to parse version list: {e}"))?;

    entries
        .iter()
        .find(|e| e.lts.is_string())
        .map(|e| e.version.clone())
        .ok_or_else(|| "No LTS version found in Node.js release index".into())
}

static NODE_DOWNLOAD_IN_PROGRESS: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

struct DownloadGuard;
impl Drop for DownloadGuard {
    fn drop(&mut self) {
        NODE_DOWNLOAD_IN_PROGRESS.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

#[tauri::command]
pub async fn download_node(app: AppHandle) -> Result<(), String> {
    if NODE_DOWNLOAD_IN_PROGRESS.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return Err("A Node.js download is already in progress.".into());
    }
    let _guard = DownloadGuard;

    let emit = |p: RuntimeInstallProgress| { app.emit(RUNTIME_INSTALL_EVENT, p).ok(); };

    emit(RuntimeInstallProgress::Downloading {
        percent: 0.0, downloaded_bytes: 0, total_bytes: 0,
    });

    let version = fetch_latest_lts_version().await?;
    let url = node_download_url(&version);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| {
            let msg = format!("HTTP client error: {e}");
            emit(RuntimeInstallProgress::Error { message: msg.clone() });
            msg
        })?;

    let resp = client.get(&url).send().await.map_err(|e| {
        let msg = format!("Download failed: {e}");
        emit(RuntimeInstallProgress::Error { message: msg.clone() });
        msg
    })?;

    if !resp.status().is_success() {
        let msg = format!("Download failed: HTTP {}", resp.status());
        emit(RuntimeInstallProgress::Error { message: msg.clone() });
        return Err(msg);
    }

    let total = resp.content_length().unwrap_or(0);
    let dest_dir = managed_node_dir();
    let archive_path = managed_runtimes_dir().join("node-download.tar.gz");

    if let Some(parent) = archive_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            let msg = format!("Failed to create runtimes directory: {e}");
            emit(RuntimeInstallProgress::Error { message: msg.clone() });
            msg
        })?;
    }

    // Stream the download with progress updates.
    let mut file = tokio::fs::File::create(&archive_path).await.map_err(|e| {
        let msg = format!("Failed to create archive file: {e}");
        emit(RuntimeInstallProgress::Error { message: msg.clone() });
        msg
    })?;

    let mut downloaded: u64 = 0;
    let mut last_pct: u64 = 0;
    let mut resp = resp;
    while let Some(chunk) = resp.chunk().await.map_err(|e| {
        let msg = format!("Download interrupted: {e}");
        emit(RuntimeInstallProgress::Error { message: msg.clone() });
        let _ = std::fs::remove_file(&archive_path);
        msg
    })? {
        file.write_all(&chunk).await.map_err(|e| {
            let msg = format!("Write error: {e}");
            emit(RuntimeInstallProgress::Error { message: msg.clone() });
            let _ = std::fs::remove_file(&archive_path);
            msg
        })?;
        downloaded += chunk.len() as u64;
        let pct = if total > 0 { (downloaded * 100) / total } else { 0 };
        if pct != last_pct || downloaded == total {
            last_pct = pct;
            emit(RuntimeInstallProgress::Downloading {
                percent: if total > 0 { (downloaded as f64 / total as f64) * 100.0 } else { 0.0 },
                downloaded_bytes: downloaded,
                total_bytes: total,
            });
        }
    }
    drop(file);

    // Extract the archive.
    emit(RuntimeInstallProgress::Extracting);

    if dest_dir.exists() {
        std::fs::remove_dir_all(&dest_dir).ok();
    }
    std::fs::create_dir_all(&dest_dir).map_err(|e| {
        let msg = format!("Failed to create node directory: {e}");
        emit(RuntimeInstallProgress::Error { message: msg.clone() });
        msg
    })?;

    let archive_str = archive_path.to_string_lossy().to_string();
    let dest_str = dest_dir.to_string_lossy().to_string();

    let extract_status = TokioCommand::new("tar")
        .args(["xzf", &archive_str, "-C", &dest_str, "--strip-components=1"])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .status()
        .await;

    match extract_status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            let msg = format!("Extraction failed (exit code {})", s.code().unwrap_or(-1));
            emit(RuntimeInstallProgress::Error { message: msg.clone() });
            let _ = std::fs::remove_dir_all(&dest_dir);
            let _ = std::fs::remove_file(&archive_path);
            return Err(msg);
        }
        Err(e) => {
            let msg = format!("Failed to run tar: {e}");
            emit(RuntimeInstallProgress::Error { message: msg.clone() });
            let _ = std::fs::remove_dir_all(&dest_dir);
            let _ = std::fs::remove_file(&archive_path);
            return Err(msg);
        }
    }

    // Clean up the archive.
    let _ = std::fs::remove_file(&archive_path);

    // Verify the install by running `node -v`.
    emit(RuntimeInstallProgress::Verifying);

    let node_bin = managed_node_bin().ok_or_else(|| {
        let msg = "Node binary not found after extraction".to_string();
        emit(RuntimeInstallProgress::Error { message: msg.clone() });
        msg
    })?;

    let verify = TokioCommand::new(&node_bin).arg("-v").output().await;
    match verify {
        Ok(out) if out.status.success() => {
            let raw = String::from_utf8_lossy(&out.stdout);
            let ver = parse_runtime_version(RuntimeName::Node, &raw)
                .unwrap_or_else(|| "unknown".into());
            emit(RuntimeInstallProgress::Done { version: ver });
            Ok(())
        }
        Ok(out) => {
            let msg = format!(
                "Node installed but verification failed (exit {})",
                out.status.code().unwrap_or(-1)
            );
            emit(RuntimeInstallProgress::Error { message: msg.clone() });
            Err(msg)
        }
        Err(e) => {
            let msg = format!("Could not run installed node: {e}");
            emit(RuntimeInstallProgress::Error { message: msg.clone() });
            Err(msg)
        }
    }
}

#[tauri::command]
pub async fn check_runtime(app: AppHandle, name: RuntimeName) -> Result<RuntimeStatus, String> {
    if matches!(name, RuntimeName::Uv) {
        let out = sidecar::run_uv(&app, &["-V"]).await.map_err(|e| e.to_string())?;
        if out.success() {
            return Ok(RuntimeStatus {
                installed: true,
                version: parse_runtime_version(RuntimeName::Uv, &out.stdout),
                source: Some("sidecar".into()),
            });
        }
        return Ok(RuntimeStatus { installed: false, version: None, source: None });
    }

    let (probe, args) = check_command_for(name);
    let probe_status = Command::new(probe).args(&args).output();
    let installed = matches!(&probe_status, Ok(o) if o.status.success() && !o.stdout.is_empty());

    if installed {
        let (vprog, vargs) = version_command_for(name);
        let version = Command::new(vprog).args(&vargs).output().ok().and_then(|o| {
            let raw = String::from_utf8_lossy(&o.stdout);
            let alt = String::from_utf8_lossy(&o.stderr);
            let combined = if raw.trim().is_empty() { alt.into_owned() } else { raw.into_owned() };
            parse_runtime_version(name, &combined)
        });
        return Ok(RuntimeStatus { installed: true, version, source: Some("system".into()) });
    }

    // System node not found — check for a Configonaut-managed install.
    if matches!(name, RuntimeName::Node) {
        if let Some(bin) = managed_node_bin() {
            let version = Command::new(&bin).arg("-v").output().ok().and_then(|o| {
                let raw = String::from_utf8_lossy(&o.stdout);
                parse_runtime_version(RuntimeName::Node, &raw)
            });
            return Ok(RuntimeStatus { installed: true, version, source: Some("managed".into()) });
        }
    }

    Ok(RuntimeStatus { installed: false, version: None, source: None })
}

/// Render the final config JSON for a server, substituting template
/// markers `{{name}}` with values from the user's form input.
pub fn render_config_block(
    cfg: &CatalogConfig,
    schema: &[ConfigField],
    values: &BTreeMap<String, Value>,
) -> Result<Map<String, Value>, String> {
    for f in schema {
        if f.required && !values.contains_key(&f.name) {
            return Err(format!("Missing required field: {}", f.name));
        }
    }

    let mut out = Map::new();

    if let Some(cmd) = &cfg.command {
        out.insert("command".into(), json!(cmd));
    }

    if let Some(args) = &cfg.args {
        let mut rendered: Vec<Value> = Vec::with_capacity(args.len());
        for arg in args {
            if let Some(field) = schema_field_for_marker(arg, schema) {
                if matches!(field.kind, ConfigFieldKind::ArgSpread) {
                    if let Some(Value::Array(items)) = values.get(&field.name) {
                        rendered.extend(items.iter().cloned());
                    }
                    continue;
                }
                if matches!(field.kind, ConfigFieldKind::Arg) {
                    if let Some(v) = values.get(&field.name) {
                        rendered.push(v.clone());
                    }
                    // optional Arg field with no value: drop the slot (same semantics as empty ArgSpread)
                    continue;
                }
            }
            rendered.push(json!(substitute_substrings(arg, schema, values)));
        }
        out.insert("args".into(), Value::Array(rendered));
    }

    let mut env: Map<String, Value> = cfg.env.clone().unwrap_or_default();
    for (k, v) in env.clone() {
        if let Value::String(s) = v {
            env.insert(k, json!(substitute_substrings(&s, schema, values)));
        }
    }
    for f in schema {
        if matches!(f.kind, ConfigFieldKind::Env) {
            if let Some(v) = values.get(&f.name) {
                env.insert(f.name.clone(), v.clone());
            }
        }
    }
    if !env.is_empty() {
        out.insert("env".into(), Value::Object(env));
    }

    if let Some(url) = &cfg.url {
        out.insert("url".into(), json!(substitute_substrings(url, schema, values)));
    }
    if let Some(headers) = &cfg.headers {
        out.insert("headers".into(), Value::Object(headers.clone()));
    }

    Ok(out)
}

fn schema_field_for_marker<'a>(s: &str, schema: &'a [ConfigField]) -> Option<&'a ConfigField> {
    let trimmed = s.trim();
    if !(trimmed.starts_with("{{") && trimmed.ends_with("}}")) {
        return None;
    }
    let name = trimmed[2..trimmed.len() - 2].trim();
    schema.iter().find(|f| f.name == name)
}

fn substitute_substrings(
    raw: &str,
    schema: &[ConfigField],
    values: &BTreeMap<String, Value>,
) -> String {
    let mut out = raw.to_string();
    for f in schema {
        let marker = format!("{{{{{}}}}}", f.name);
        if !out.contains(&marker) {
            continue;
        }
        let replacement = match values.get(&f.name) {
            Some(Value::String(s)) => s.clone(),
            Some(v) => v.to_string(),
            None => String::new(),
        };
        out = out.replace(&marker, &replacement);
    }
    out
}

// ---------------------------------------------------------------------------
// inspect_install — returns schema for the Setup UI
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrerequisiteEntry {
    pub r#type: RuntimeName,
    pub status: Option<RuntimeStatus>,
    pub install_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallSchema {
    pub prerequisites: Vec<PrerequisiteEntry>,
    pub config_fields: Vec<ConfigField>,
    pub install_step_count: usize,
    pub has_unknown_install_step: bool,
    pub post_install_notes: Vec<crate::catalog::PostInstallNote>,
}

fn effective_config_fields(server: &CatalogServer) -> Vec<ConfigField> {
    if !server.config_fields.is_empty() {
        return server.config_fields.clone();
    }
    server.env_vars.as_deref().unwrap_or(&[]).iter().map(|ev| ConfigField {
        name: ev.name.clone(),
        kind: ConfigFieldKind::Env,
        r#type: if ev.secret { ConfigFieldType::Secret } else { ConfigFieldType::String },
        label: ev.name.clone(),
        description: ev.description.clone(),
        required: ev.required,
        placeholder: ev.placeholder.clone(),
        default: None,
        help_url: ev.help_url.clone(),
    }).collect()
}

pub fn build_inspect_schema(server: &CatalogServer) -> InstallSchema {
    let prerequisites = server.prerequisites.iter().map(|p: &CatalogPrerequisite| {
        PrerequisiteEntry {
            r#type: p.r#type,
            status: None,
            install_url: runtime_install_url(p.r#type).map(str::to_string),
        }
    }).collect();

    let has_unknown = server.install.iter().any(|s| matches!(s, InstallStep::Unknown));

    let config_fields = effective_config_fields(server);

    InstallSchema {
        prerequisites,
        config_fields,
        install_step_count: server.install.len(),
        has_unknown_install_step: has_unknown,
        post_install_notes: server.post_install_notes.clone(),
    }
}

#[tauri::command]
pub async fn inspect_install(server_id: String) -> Result<InstallSchema, String> {
    let (catalog, _) = crate::catalog::bootstrap_catalog_with_feeds()
        .map_err(|e| format!("Failed to read catalog: {e}"))?;
    let server = catalog.servers.iter().find(|s| s.id == server_id)
        .ok_or_else(|| format!("Server '{server_id}' not found in catalog."))?;
    Ok(build_inspect_schema(server))
}

// ---------------------------------------------------------------------------
// install_server — runs install steps, streams progress events, writes config
// ---------------------------------------------------------------------------

pub fn warmup_command_for(step: &InstallStep) -> (&'static str, Vec<String>) {
    match step {
        InstallStep::NpmWarmup { package } => {
            ("npx", vec!["-y".into(), package.clone(), "--help".into()])
        }
        InstallStep::UvxWarmup { package } => {
            ("uvx", vec![package.clone(), "--help".into()])
        }
        InstallStep::DockerPull { image } => ("docker", vec!["pull".into(), image.clone()]),
        InstallStep::None | InstallStep::Unknown => ("true", vec![]),
    }
}

fn label_for(step: &InstallStep) -> String {
    match step {
        InstallStep::NpmWarmup { package } => format!("Pre-fetching {package}..."),
        InstallStep::UvxWarmup { package } => format!("Pre-fetching {package} via uv..."),
        InstallStep::DockerPull { image } => format!("Pulling {image}..."),
        InstallStep::None => "No install step needed".into(),
        InstallStep::Unknown => "Skipping unknown install step".into(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum InstallProgress {
    Step { step: String, label: String },
    Log { line: String },
    Error { step: String, message: String, can_retry: bool },
}

const PROGRESS_EVENT: &str = "install-progress";

/// If the server uses `npx` (or `node`) as its command and only the
/// Configonaut-managed Node is available, prepend the managed bin dir to
/// PATH in the server config's `env` block so Claude Desktop/Code can
/// find the binary when spawning the MCP process.
pub fn inject_managed_node_path(config: &mut Map<String, Value>) {
    if !config_needs_node(config) {
        return;
    }

    // Only inject if there's no system node and we have a managed install.
    let probe = if cfg!(target_os = "windows") { "where" } else { "which" };
    let has_system = Command::new(probe)
        .arg("node")
        .output()
        .map(|o| o.status.success() && !o.stdout.is_empty())
        .unwrap_or(false);

    if has_system {
        return;
    }

    let Some(bin_dir) = managed_node_bin_dir() else { return };
    let current_path = std::env::var("PATH").unwrap_or_default();
    do_inject_node_path(config, &bin_dir.to_string_lossy(), &current_path);
}

fn config_needs_node(config: &Map<String, Value>) -> bool {
    match config.get("command") {
        Some(Value::String(cmd)) if matches!(cmd.as_str(), "npx" | "node" | "npx.cmd" | "node.exe") => true,
        // Defense in depth: see through a `cmd /c <program> ...` wrapper
        // (what adapt_config_for_windows produces) in case this ever runs
        // after the Windows adapter instead of before it.
        Some(Value::String(cmd)) if cmd == "cmd" => match config.get("args") {
            Some(Value::Array(args)) => args
                .get(1)
                .and_then(Value::as_str)
                .map(|inner| matches!(inner, "npx" | "node" | "npx.cmd" | "node.exe"))
                .unwrap_or(false),
            _ => false,
        },
        _ => false,
    }
}

/// Inner implementation, split out so tests can exercise it without
/// filesystem or system-PATH side effects.
fn do_inject_node_path(config: &mut Map<String, Value>, bin_dir: &str, current_path: &str) {
    let env = config
        .entry("env")
        .or_insert_with(|| Value::Object(Map::new()));

    if let Value::Object(env_map) = env {
        let separator = if cfg!(target_os = "windows") { ";" } else { ":" };

        if let Some(Value::String(existing)) = env_map.get("PATH") {
            if !existing.contains(bin_dir) {
                env_map.insert(
                    "PATH".into(),
                    Value::String(format!("{bin_dir}{separator}{existing}")),
                );
            }
        } else {
            env_map.insert(
                "PATH".into(),
                Value::String(format!("{bin_dir}{separator}{current_path}")),
            );
        }
    }
}

#[tauri::command]
pub async fn install_server(
    app: tauri::AppHandle,
    mode: crate::models::AppMode,
    server_id: String,
    field_values: BTreeMap<String, Value>,
) -> Result<String, String> {
    let (catalog, _) = crate::catalog::bootstrap_catalog_with_feeds()
        .map_err(|e| format!("Failed to read catalog: {e}"))?;
    let server = catalog.servers.iter().find(|s| s.id == server_id)
        .ok_or_else(|| format!("Server '{server_id}' not found in catalog."))?
        .clone();

    app.emit(PROGRESS_EVENT, InstallProgress::Step {
        step: "configure".into(),
        label: "Validating configuration".into(),
    }).ok();
    let rendered = render_config_block(&server.config, &effective_config_fields(&server), &field_values)?;

    for step in &server.install {
        let label = label_for(step);
        app.emit(PROGRESS_EVENT, InstallProgress::Step {
            step: "install".into(),
            label,
        }).ok();

        let (program, args) = warmup_command_for(step);
        // Skip no-op steps
        if program == "true" {
            continue;
        }

        let mut command = TokioCommand::new(program);
        command.args(&args).stdout(Stdio::piped()).stderr(Stdio::piped());

        // npx is the only program we manage ourselves — if the user just
        // installed Node via the in-app downloader, the managed bin dir
        // is NOT on the Tauri process's inherited PATH, so a bare
        // `TokioCommand::new("npx")` would ENOENT. Mirror the same path
        // injection inject_managed_node_path does for server configs.
        if program == "npx" || program == "node" {
            if let Some(bin_dir) = managed_node_bin_dir() {
                let sep = if cfg!(target_os = "windows") { ";" } else { ":" };
                let current_path = std::env::var("PATH").unwrap_or_default();
                let bin_dir_str = bin_dir.to_string_lossy();
                if !current_path.split(sep).any(|p| p == bin_dir_str) {
                    command.env("PATH", format!("{bin_dir_str}{sep}{current_path}"));
                }
            }
        }

        let mut child = command
            .spawn()
            .map_err(|e| {
                let msg = format!("Could not start {program}: {e}");
                app.emit(PROGRESS_EVENT, InstallProgress::Error {
                    step: "install".into(),
                    message: msg.clone(),
                    can_retry: false,
                }).ok();
                msg
            })?;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let app_a = app.clone();
        let app_b = app.clone();

        let stdout_task = tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                app_a.emit(PROGRESS_EVENT, InstallProgress::Log { line }).ok();
            }
        });

        let stderr_task = tokio::spawn(async move {
            let mut buf = String::new();
            let mut reader = tokio::io::BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                buf.push_str(&line);
                buf.push('\n');
                app_b.emit(PROGRESS_EVENT, InstallProgress::Log { line }).ok();
            }
            buf
        });

        let status = child.wait().await.map_err(|e| e.to_string())?;
        stdout_task.await.ok();
        let stderr_buf = stderr_task.await.unwrap_or_default();

        if !status.success() {
            let kind = classify_install_error(&stderr_buf, status.code().unwrap_or(-1));
            let message = kind.user_message();
            app.emit(PROGRESS_EVENT, InstallProgress::Error {
                step: "install".into(),
                message: message.clone(),
                can_retry: matches!(kind, InstallErrorKind::Network | InstallErrorKind::DockerDaemonDown | InstallErrorKind::DiskFull),
            }).ok();
            return Err(message);
        }
    }

    // Write to Claude config — now literally shares finalize_install with
    // install_from_catalog, instead of just following the same path.
    app.emit(PROGRESS_EVENT, InstallProgress::Step {
        step: "configure".into(),
        label: "Writing configuration".into(),
    }).ok();

    // Always active: render_config_block already rejects missing required
    // fields, so there is nothing left to park as "stored" here.
    let name = crate::catalog::finalize_install(
        mode,
        &server.id,
        rendered,
        &server.id,
        crate::models::ServerSource::Active,
    )
    .map_err(|e| e.to_string())?;

    app.emit(PROGRESS_EVENT, InstallProgress::Step {
        step: "done".into(),
        label: "Done".into(),
    }).ok();

    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_network_error_from_curl_message() {
        let kind = classify_install_error("curl: Could not resolve host: registry.npmjs.org", 6);
        assert_eq!(kind, InstallErrorKind::Network);
    }

    #[test]
    fn classify_docker_daemon_down() {
        let kind = classify_install_error("Cannot connect to the Docker daemon", 1);
        assert_eq!(kind, InstallErrorKind::DockerDaemonDown);
    }

    #[test]
    fn classify_disk_full() {
        let kind = classify_install_error("write: No space left on device", 28);
        assert_eq!(kind, InstallErrorKind::DiskFull);
    }

    #[test]
    fn classify_interrupted_by_exit_code() {
        let kind = classify_install_error("", 130);
        assert_eq!(kind, InstallErrorKind::Interrupted);
    }

    #[test]
    fn classify_generic_falls_through() {
        let kind = classify_install_error("something weird", 42);
        assert_eq!(kind, InstallErrorKind::Generic(42));
    }

    #[test]
    fn runtime_install_urls_are_hardcoded() {
        assert!(runtime_install_url(RuntimeName::Node).unwrap().starts_with("https://"));
        assert!(runtime_install_url(RuntimeName::Docker).unwrap().starts_with("https://"));
        assert!(runtime_install_url(RuntimeName::Uv).is_none());
    }

    #[test]
    fn user_messages_are_non_empty() {
        for kind in [
            InstallErrorKind::Network,
            InstallErrorKind::DockerDaemonDown,
            InstallErrorKind::DiskFull,
            InstallErrorKind::Interrupted,
            InstallErrorKind::Generic(7),
        ] {
            assert!(!kind.user_message().is_empty());
        }
    }

    #[test]
    fn build_check_command_for_node() {
        let (program, args) = check_command_for(RuntimeName::Node);
        if cfg!(target_os = "windows") {
            assert_eq!(program, "where");
        } else {
            assert_eq!(program, "which");
        }
        assert_eq!(args, vec!["node"]);
    }

    #[test]
    fn build_check_command_for_docker() {
        let (_, args) = check_command_for(RuntimeName::Docker);
        assert_eq!(args, vec!["docker"]);
    }

    #[test]
    fn parse_node_version_from_v_output() {
        assert_eq!(parse_runtime_version(RuntimeName::Node, "v20.11.1\n"), Some("20.11.1".into()));
    }

    #[test]
    fn parse_docker_version_from_dashv() {
        let v = parse_runtime_version(RuntimeName::Docker, "Docker version 24.0.7, build afdd53b\n");
        assert_eq!(v, Some("24.0.7".into()));
    }

    #[test]
    fn install_runtime_uv_returns_ready() {
        let action = install_runtime_for(RuntimeName::Uv);
        assert!(matches!(action, InstallAction::Ready));
    }

    #[test]
    fn install_runtime_node_returns_download() {
        let action = install_runtime_for(RuntimeName::Node);
        assert!(matches!(action, InstallAction::Download));
    }

    #[test]
    fn install_runtime_docker_returns_open_url() {
        let action = install_runtime_for(RuntimeName::Docker);
        assert!(matches!(action, InstallAction::OpenUrl { .. }));
    }

    use serde_json::json;
    use std::collections::BTreeMap;

    fn fields(pairs: &[(&str, serde_json::Value)]) -> BTreeMap<String, serde_json::Value> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
    }

    #[test]
    fn render_substring_substitution_in_env() {
        use crate::catalog::{CatalogConfig, ConfigField, ConfigFieldKind, ConfigFieldType};
        let cfg = CatalogConfig {
            command: Some("npx".into()),
            args: Some(vec!["-y".into(), "@scope/foo".into()]),
            env: Some(serde_json::Map::from_iter([(
                "API_KEY".to_string(), json!("Bearer {{api_key}}"),
            )])),
            url: None,
            headers: None,
        };
        let schema = vec![ConfigField {
            name: "api_key".into(),
            kind: ConfigFieldKind::Env,
            r#type: ConfigFieldType::Secret,
            label: "API key".into(),
            description: None, required: true, placeholder: None, default: None, help_url: None,
        }];
        let values = fields(&[("api_key", json!("sk-test-123"))]);
        let rendered = render_config_block(&cfg, &schema, &values).unwrap();
        assert_eq!(rendered["env"]["API_KEY"], json!("Bearer sk-test-123"));
    }

    #[test]
    fn render_arg_spread_expands_into_args() {
        use crate::catalog::{CatalogConfig, ConfigField, ConfigFieldKind, ConfigFieldType};
        let cfg = CatalogConfig {
            command: Some("npx".into()),
            args: Some(vec!["-y".into(), "@scope/foo".into(), "{{paths}}".into()]),
            env: None, url: None, headers: None,
        };
        let schema = vec![ConfigField {
            name: "paths".into(),
            kind: ConfigFieldKind::ArgSpread,
            r#type: ConfigFieldType::PathArray,
            label: "Paths".into(),
            description: None, required: true, placeholder: None, default: None, help_url: None,
        }];
        let values = fields(&[("paths", json!(["/a", "/b"]))]);
        let rendered = render_config_block(&cfg, &schema, &values).unwrap();
        assert_eq!(rendered["args"], json!(["-y", "@scope/foo", "/a", "/b"]));
    }

    #[test]
    fn render_empty_arg_spread_drops_marker() {
        use crate::catalog::{CatalogConfig, ConfigField, ConfigFieldKind, ConfigFieldType};
        let cfg = CatalogConfig {
            command: Some("npx".into()),
            args: Some(vec!["-y".into(), "{{paths}}".into()]),
            env: None, url: None, headers: None,
        };
        let schema = vec![ConfigField {
            name: "paths".into(), kind: ConfigFieldKind::ArgSpread,
            r#type: ConfigFieldType::PathArray, label: "Paths".into(),
            description: None, required: false, placeholder: None, default: None, help_url: None,
        }];
        let values = fields(&[("paths", json!([]))]);
        let rendered = render_config_block(&cfg, &schema, &values).unwrap();
        assert_eq!(rendered["args"], json!(["-y"]));
    }

    #[test]
    fn render_missing_required_field_returns_err() {
        use crate::catalog::{CatalogConfig, ConfigField, ConfigFieldKind, ConfigFieldType};
        let cfg = CatalogConfig {
            command: Some("npx".into()), args: None, env: None, url: None, headers: None,
        };
        let schema = vec![ConfigField {
            name: "api_key".into(), kind: ConfigFieldKind::Env,
            r#type: ConfigFieldType::Secret, label: "API key".into(),
            description: None, required: true, placeholder: None, default: None, help_url: None,
        }];
        let values = fields(&[]);
        assert!(render_config_block(&cfg, &schema, &values).is_err());
    }

    #[test]
    fn build_warmup_command_for_npm() {
        let (program, args) = warmup_command_for(&InstallStep::NpmWarmup {
            package: "@scope/foo".into(),
        });
        assert_eq!(program, "npx");
        assert_eq!(args, vec!["-y", "@scope/foo", "--help"]);
    }

    #[test]
    fn build_warmup_command_for_uvx() {
        let (program, args) = warmup_command_for(&InstallStep::UvxWarmup {
            package: "mcp-server-foo".into(),
        });
        assert_eq!(program, "uvx");
        assert_eq!(args, vec!["mcp-server-foo", "--help"]);
    }

    #[test]
    fn build_warmup_command_for_docker_pull() {
        let (program, args) = warmup_command_for(&InstallStep::DockerPull {
            image: "ghcr.io/foo/bar:latest".into(),
        });
        assert_eq!(program, "docker");
        assert_eq!(args, vec!["pull", "ghcr.io/foo/bar:latest"]);
    }

    #[test]
    fn inspect_combines_prereqs_and_fields() {
        use crate::catalog::{CatalogServer, CatalogPrerequisite, CatalogPublisher, CatalogConfig,
                             ConfigField, ConfigFieldKind, ConfigFieldType, InstallStep};

        let server = CatalogServer {
            id: "x".into(), name: "X".into(), description: String::new(),
            category: String::new(), tags: vec![],
            publisher: CatalogPublisher { name: "a".into(), kind: "official".into(), verified: false },
            homepage: None, repository: None, license: None, popularity: 0,
            config: CatalogConfig { command: Some("npx".into()), args: None, env: None, url: None, headers: None },
            transport: "stdio".into(),
            requirements: vec![], setup_notes: None, env_vars: None, feed_origin: None,
            prerequisites: vec![CatalogPrerequisite { r#type: RuntimeName::Node }],
            install: vec![InstallStep::NpmWarmup { package: "x".into() }],
            config_fields: vec![ConfigField {
                name: "k".into(), kind: ConfigFieldKind::Env, r#type: ConfigFieldType::Secret,
                label: "K".into(), description: None, required: true,
                placeholder: None, default: None, help_url: None,
            }],
            post_install_notes: vec![],
        };

        let schema = build_inspect_schema(&server);
        assert_eq!(schema.prerequisites.len(), 1);
        assert_eq!(schema.config_fields.len(), 1);
        assert_eq!(schema.install_step_count, 1);
    }

    #[test]
    fn render_absent_optional_arg_drops_slot() {
        use crate::catalog::{CatalogConfig, ConfigField, ConfigFieldKind, ConfigFieldType};
        let cfg = CatalogConfig {
            command: Some("npx".into()),
            args: Some(vec!["-y".into(), "{{port}}".into()]),
            env: None, url: None, headers: None,
        };
        let schema = vec![ConfigField {
            name: "port".into(), kind: ConfigFieldKind::Arg,
            r#type: ConfigFieldType::Number, label: "Port".into(),
            description: None, required: false, placeholder: None, default: None, help_url: None,
        }];
        // No "port" value — optional field absent.
        let values = std::collections::BTreeMap::new();
        let rendered = render_config_block(&cfg, &schema, &values).unwrap();
        // The marker slot is dropped; only the static flags remain.
        assert_eq!(rendered["args"], serde_json::json!(["-y"]));
    }

    #[test]
    fn node_download_url_contains_version_and_platform() {
        let url = node_download_url("v22.13.1");
        assert!(url.starts_with("https://nodejs.org/dist/v22.13.1/node-v22.13.1-"));
        assert!(url.ends_with(".tar.gz"));
        assert!(url.contains("-x64.") || url.contains("-arm64."));
    }

    #[test]
    fn managed_runtimes_dir_is_under_storage() {
        let dir = managed_runtimes_dir();
        assert!(dir.ends_with("runtimes"));
    }

    #[test]
    fn managed_node_dir_is_under_runtimes() {
        let dir = managed_node_dir();
        assert!(dir.ends_with("node"));
        assert!(dir.starts_with(managed_runtimes_dir()));
    }

    #[test]
    fn managed_node_bin_returns_none_when_not_installed() {
        assert!(managed_node_bin().is_none());
    }

    #[test]
    fn managed_node_bin_dir_returns_none_when_not_installed() {
        assert!(managed_node_bin_dir().is_none());
    }

    #[test]
    fn inject_managed_node_path_skips_non_node_commands() {
        let mut config: Map<String, Value> = Map::new();
        config.insert("command".into(), json!("uvx"));
        inject_managed_node_path(&mut config);
        assert!(config.get("env").is_none());
    }

    #[test]
    fn inject_managed_node_path_skips_when_system_node_exists() {
        let mut config: Map<String, Value> = Map::new();
        config.insert("command".into(), json!("npx"));
        inject_managed_node_path(&mut config);
        if managed_node_bin_dir().is_none() {
            let has_path = config.get("env")
                .and_then(|e| e.as_object())
                .and_then(|m| m.get("PATH"))
                .is_some();
            assert!(!has_path);
        }
    }

    #[test]
    fn config_needs_node_recognizes_all_variants() {
        for cmd in ["npx", "node", "npx.cmd", "node.exe"] {
            let mut config: Map<String, Value> = Map::new();
            config.insert("command".into(), json!(cmd));
            assert!(config_needs_node(&config), "should match: {cmd}");
        }
        for cmd in ["uvx", "docker", "python"] {
            let mut config: Map<String, Value> = Map::new();
            config.insert("command".into(), json!(cmd));
            assert!(!config_needs_node(&config), "should not match: {cmd}");
        }
    }

    #[test]
    fn do_inject_node_path_prepends_bin_dir() {
        let mut config: Map<String, Value> = Map::new();
        config.insert("command".into(), json!("npx"));
        do_inject_node_path(&mut config, "/managed/node/bin", "/usr/local/bin:/usr/bin");
        let path = config["env"]["PATH"].as_str().unwrap();
        assert!(path.starts_with("/managed/node/bin"));
        assert!(path.contains("/usr/local/bin"));
        assert!(path.contains("/usr/bin"));
    }

    #[test]
    fn do_inject_node_path_prepends_to_existing_path_entry() {
        let mut config: Map<String, Value> = Map::new();
        config.insert("command".into(), json!("npx"));
        let mut env = Map::new();
        env.insert("PATH".into(), json!("/custom/bin:/other/bin"));
        config.insert("env".into(), Value::Object(env));

        do_inject_node_path(&mut config, "/managed/node/bin", "/system/path");

        let path = config["env"]["PATH"].as_str().unwrap();
        assert!(path.starts_with("/managed/node/bin"));
        assert!(path.contains("/custom/bin"));
        assert!(path.contains("/other/bin"));
        // Should NOT contain the system path — the existing entry is used.
        assert!(!path.contains("/system/path"));
    }

    #[test]
    fn do_inject_node_path_skips_if_already_present() {
        let mut config: Map<String, Value> = Map::new();
        config.insert("command".into(), json!("npx"));
        let mut env = Map::new();
        env.insert("PATH".into(), json!("/managed/node/bin:/usr/bin"));
        config.insert("env".into(), Value::Object(env));

        do_inject_node_path(&mut config, "/managed/node/bin", "/whatever");

        let path = config["env"]["PATH"].as_str().unwrap();
        // Should be unchanged — bin_dir is already present.
        assert_eq!(path, "/managed/node/bin:/usr/bin");
    }

    #[test]
    fn do_inject_node_path_preserves_existing_env_vars() {
        let mut config: Map<String, Value> = Map::new();
        config.insert("command".into(), json!("npx"));
        let mut env = Map::new();
        env.insert("API_KEY".into(), json!("secret123"));
        config.insert("env".into(), Value::Object(env));

        do_inject_node_path(&mut config, "/managed/bin", "/usr/bin");

        assert_eq!(config["env"]["API_KEY"], json!("secret123"));
        assert!(config["env"]["PATH"].as_str().unwrap().contains("/managed/bin"));
    }

    #[test]
    fn config_needs_node_sees_through_cmd_wrapper() {
        let mut config = Map::new();
        config.insert("command".into(), json!("cmd"));
        config.insert("args".into(), json!(["/c", "npx", "-y", "pkg"]));
        assert!(config_needs_node(&config));
        config.insert("args".into(), json!(["/c", "docker", "run"]));
        assert!(!config_needs_node(&config));
    }

    #[test]
    fn windows_adapt_after_inject_keeps_env_path() {
        // The composed order finalize_install uses: inject, then wrap.
        let mut config = Map::new();
        config.insert("command".into(), json!("npx"));
        config.insert("args".into(), json!(["-y", "pkg"]));
        do_inject_node_path(&mut config, "/managed/bin", "/usr/bin");
        crate::catalog::do_adapt_config_for_windows(&mut config);
        assert_eq!(config["command"], "cmd");
        assert_eq!(config["args"][1], "npx");
        assert!(config["env"]["PATH"].as_str().unwrap().starts_with("/managed/bin"));
    }

    #[test]
    fn download_guard_resets_flag() {
        // Simulate the guard's drop behavior.
        NODE_DOWNLOAD_IN_PROGRESS.store(true, std::sync::atomic::Ordering::SeqCst);
        {
            let _guard = DownloadGuard;
        }
        assert!(!NODE_DOWNLOAD_IN_PROGRESS.load(std::sync::atomic::Ordering::SeqCst));
    }
}
