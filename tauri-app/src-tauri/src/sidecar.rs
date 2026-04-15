// Thin wrapper around tauri-plugin-shell's sidecar API. Centralises
// the binary name + argument plumbing so call sites stay one-liners
// and we don't repeat the spawn boilerplate.

use anyhow::{anyhow, Result};
use tauri::AppHandle;
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

/// Name registered in `tauri.conf.json` under `bundle.externalBin`.
/// Tauri picks the right per-platform suffix at runtime.
pub const UV_SIDECAR_NAME: &str = "binaries/uv";

/// Run `uv <args>` to completion, collecting stdout + stderr.
/// Returns the combined output and the exit code.
pub async fn run_uv(app: &AppHandle, args: &[&str]) -> Result<UvOutput> {
    let cmd = app
        .shell()
        .sidecar(UV_SIDECAR_NAME)
        .map_err(|e| anyhow!("uv sidecar not found: {e}"))?
        .args(args);

    let (mut rx, _child) = cmd.spawn().map_err(|e| anyhow!("spawn uv: {e}"))?;

    let mut stdout = String::new();
    let mut stderr = String::new();
    let mut exit_code: Option<i32> = None;

    while let Some(ev) = rx.recv().await {
        match ev {
            CommandEvent::Stdout(line) => {
                stdout.push_str(&String::from_utf8_lossy(&line));
                stdout.push('\n');
            }
            CommandEvent::Stderr(line) => {
                stderr.push_str(&String::from_utf8_lossy(&line));
                stderr.push('\n');
            }
            CommandEvent::Terminated(payload) => {
                exit_code = payload.code;
            }
            _ => {}
        }
    }

    Ok(UvOutput { stdout, stderr, exit_code: exit_code.unwrap_or(-1) })
}

#[derive(Debug, Clone)]
pub struct UvOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl UvOutput {
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uv_output_success_helper() {
        assert!(UvOutput { stdout: String::new(), stderr: String::new(), exit_code: 0 }.success());
        assert!(!UvOutput { stdout: String::new(), stderr: String::new(), exit_code: 1 }.success());
    }
}
