// Auto-install runner. See docs/superpowers/specs/2026-04-14-mcp-auto-install-design.md.

use serde::{Deserialize, Serialize};
use std::process::Command;
use tauri::AppHandle;

use crate::catalog::RuntimeName;
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
        RuntimeName::Docker => ("docker", vec!["-v"]),
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
}
