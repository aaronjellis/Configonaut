// Auto-install runner. See docs/superpowers/specs/2026-04-14-mcp-auto-install-design.md.

use serde::{Deserialize, Serialize};

use crate::catalog::RuntimeName;

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
}
