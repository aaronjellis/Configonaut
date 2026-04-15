// Auto-install runner. See docs/superpowers/specs/2026-04-14-mcp-auto-install-design.md.

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::process::{Command, Stdio};
use tauri::AppHandle;
use tauri::Emitter;
use tokio::io::AsyncBufReadExt;
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
    match runtime_install_url(name) {
        None => InstallAction::Ready,
        Some(url) => InstallAction::OpenUrl { url: url.to_string() },
    }
}

#[tauri::command]
pub async fn install_runtime(name: RuntimeName) -> Result<InstallAction, String> {
    Ok(install_runtime_for(name))
}

// NOTE: `#[tauri::command]` is intentionally omitted here to avoid a macro
// name collision with `commands::check_runtime` (the legacy all-runtimes probe).
// This command will be registered once the old stub is retired in a later task.
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

    if !installed {
        return Ok(RuntimeStatus { installed: false, version: None, source: None });
    }

    let (vprog, vargs) = version_command_for(name);
    let version = Command::new(vprog).args(&vargs).output().ok().and_then(|o| {
        let raw = String::from_utf8_lossy(&o.stdout);
        let alt = String::from_utf8_lossy(&o.stderr);
        let combined = if raw.trim().is_empty() { alt.into_owned() } else { raw.into_owned() };
        parse_runtime_version(name, &combined)
    });

    Ok(RuntimeStatus { installed: true, version, source: Some("system".into()) })
}

/// Render the final config JSON for a server, substituting template
/// markers `{{name}}` with values from the user's form input.
pub fn render_config_block(
    cfg: &CatalogConfig,
    schema: &[ConfigField],
    values: &BTreeMap<String, Value>,
) -> Result<Value, String> {
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

    Ok(Value::Object(out))
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

    // Coalesce config_fields with the legacy env_vars field for old catalog entries.
    let mut config_fields = server.config_fields.clone();
    if config_fields.is_empty() {
        if let Some(env_vars) = &server.env_vars {
            config_fields = env_vars.iter().map(|ev| ConfigField {
                name: ev.name.clone(),
                kind: ConfigFieldKind::Env,
                r#type: if ev.secret { ConfigFieldType::Secret } else { ConfigFieldType::String },
                label: ev.name.clone(),
                description: ev.description.clone(),
                required: ev.required,
                placeholder: ev.placeholder.clone(),
                default: None,
                help_url: ev.help_url.clone(),
            }).collect();
        }
    }

    InstallSchema {
        prerequisites,
        config_fields,
        install_step_count: server.install.len(),
        has_unknown_install_step: has_unknown,
    }
}

#[tauri::command]
pub async fn inspect_install(server_id: String) -> Result<InstallSchema, String> {
    let catalog = crate::catalog::bootstrap_catalog()
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

#[tauri::command]
pub async fn install_server(
    app: tauri::AppHandle,
    server_id: String,
    field_values: BTreeMap<String, Value>,
) -> Result<(), String> {
    let catalog = crate::catalog::bootstrap_catalog()
        .map_err(|e| format!("Failed to read catalog: {e}"))?;
    let server = catalog.servers.iter().find(|s| s.id == server_id)
        .ok_or_else(|| format!("Server '{server_id}' not found in catalog."))?
        .clone();

    app.emit(PROGRESS_EVENT, InstallProgress::Step {
        step: "configure".into(),
        label: "Validating configuration".into(),
    }).ok();
    let rendered = render_config_block(&server.config, &server.config_fields, &field_values)?;

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

        let mut child = TokioCommand::new(program)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
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
                can_retry: !matches!(kind, InstallErrorKind::Generic(_)),
            }).ok();
            return Err(message);
        }
    }

    // Write to Claude config using existing config API.
    app.emit(PROGRESS_EVENT, InstallProgress::Step {
        step: "configure".into(),
        label: "Writing configuration".into(),
    }).ok();

    use crate::models::AppMode;
    crate::config::add_to_active(AppMode::Desktop, vec![(server.id.clone(), rendered)])
        .map_err(|e| e.to_string())?;

    app.emit(PROGRESS_EVENT, InstallProgress::Step {
        step: "done".into(),
        label: "Done".into(),
    }).ok();

    Ok(())
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
    fn install_runtime_node_returns_open_url() {
        let action = install_runtime_for(RuntimeName::Node);
        match action {
            InstallAction::OpenUrl { url } => assert!(url.starts_with("https://nodejs.org")),
            _ => panic!("expected OpenUrl"),
        }
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
}
