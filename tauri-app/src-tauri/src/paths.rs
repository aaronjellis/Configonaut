// Path discovery for Claude Desktop, Claude Code, and Configonaut's own
// storage directory. All paths are resolved per-OS so the same commands
// work on macOS and Windows (and Linux, as a freebie).
//
// Reference layout:
//
//   macOS
//     Claude Desktop config : ~/Library/Application Support/Claude/claude_desktop_config.json
//     Claude Code settings  : ~/.claude/settings.json
//     Configonaut storage   : ~/Library/Application Support/Configonaut/
//
//   Windows
//     Claude Desktop config : %APPDATA%\Claude\claude_desktop_config.json
//     Claude Code settings  : %USERPROFILE%\.claude\settings.json
//     Configonaut storage   : %APPDATA%\Configonaut\
//
//   Linux
//     Claude Desktop config : ~/.config/Claude/claude_desktop_config.json
//     Claude Code settings  : ~/.claude/settings.json
//     Configonaut storage   : ~/.config/Configonaut/

use std::path::PathBuf;

use crate::models::AppMode;

/// Home directory for the current user. Panics only if no home can be
/// discovered at all, which should be impossible on any sane OS install.
pub fn home() -> PathBuf {
    dirs::home_dir().expect("no home directory")
}

/// Claude Desktop's config file for the current OS.
pub fn claude_desktop_config() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        home()
            .join("Library")
            .join("Application Support")
            .join("Claude")
            .join("claude_desktop_config.json")
    }
    #[cfg(target_os = "windows")]
    {
        dirs::config_dir()
            .unwrap_or_else(|| home().join("AppData").join("Roaming"))
            .join("Claude")
            .join("claude_desktop_config.json")
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        dirs::config_dir()
            .unwrap_or_else(|| home().join(".config"))
            .join("Claude")
            .join("claude_desktop_config.json")
    }
}

/// Claude Code's global settings file. Same on every OS (~/.claude/settings.json).
pub fn claude_code_settings() -> PathBuf {
    home().join(".claude").join("settings.json")
}

/// The mcp-config file for a given mode (Desktop reads/writes Claude Desktop's
/// dedicated file, CLI reads/writes ~/.claude/settings.json which holds both
/// mcpServers and hooks).
pub fn config_file(mode: AppMode) -> PathBuf {
    match mode {
        AppMode::Desktop => claude_desktop_config(),
        AppMode::Cli => claude_code_settings(),
    }
}

/// Root directory for Configonaut's own persisted state: stored (inactive)
/// servers, backups, and catalog links.
pub fn storage_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        home()
            .join("Library")
            .join("Application Support")
            .join("Configonaut")
    }
    #[cfg(target_os = "windows")]
    {
        dirs::config_dir()
            .unwrap_or_else(|| home().join("AppData").join("Roaming"))
            .join("Configonaut")
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        dirs::config_dir()
            .unwrap_or_else(|| home().join(".config"))
            .join("Configonaut")
    }
}

/// Per-mode file that holds the user's "saved for later" (inactive) servers.
pub fn stored_servers_file(mode: AppMode) -> PathBuf {
    storage_dir().join(format!("stored_servers_{}.json", mode.as_str()))
}

/// Per-mode directory where timestamped config backups are written.
pub fn backup_dir(mode: AppMode) -> PathBuf {
    storage_dir().join("backups").join(mode.as_str())
}

/// Per-mode file mapping a server name → the catalog id it was installed from.
/// Used by the "ready to enable" gate so we know which env vars a server needs.
pub fn catalog_links_file(mode: AppMode) -> PathBuf {
    storage_dir().join(format!("catalog_links_{}.json", mode.as_str()))
}

/// Cached copy of the remote marketplace catalog, refreshed on launch.
pub fn catalog_cache_file() -> PathBuf {
    storage_dir().join("catalog-cache.json")
}

// Claude Code content directories (used for agents, skills, and plugin scanning).
// These are identical across OSes since Claude Code reads them from ~/.claude
// regardless of platform.

pub fn commands_dir() -> PathBuf {
    home().join(".claude").join("commands")
}

pub fn skills_dir() -> PathBuf {
    home().join(".claude").join("skills")
}

pub fn personal_agents_dir() -> PathBuf {
    home().join(".claude").join("agents")
}

pub fn plugins_dir() -> PathBuf {
    home()
        .join(".claude")
        .join("plugins")
        .join("marketplaces")
        .join("claude-plugins-official")
        .join("plugins")
}
