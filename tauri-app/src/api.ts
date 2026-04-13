// Typed wrappers around Tauri's `invoke()` — one function per backend
// command. Centralising the invocations here keeps the view code free of
// stringly-typed command names and gives us a single place to tweak types
// if the Rust side changes.

import { invoke } from "@tauri-apps/api/core";
import type {
  AgentEntry,
  AppMode,
  BackupFile,
  Catalog,
  HookRule,
  ServerListing,
  ServerSource,
  ServerTuple,
  SkillEntry,
  SkillSource,
} from "./types";

// ---------- MCP servers ----------

export function listServers(mode: AppMode): Promise<ServerListing> {
  return invoke<ServerListing>("list_servers", { mode });
}

export function parseServerInput(
  rawJson: string,
  fallbackName?: string
): Promise<ServerTuple[]> {
  return invoke<ServerTuple[]>("parse_server_input", {
    rawJson,
    fallbackName: fallbackName ?? null,
  });
}

export function addServersToActive(
  mode: AppMode,
  entries: ServerTuple[]
): Promise<void> {
  return invoke("add_servers_to_active", { mode, entries });
}

export function addServersToStored(
  mode: AppMode,
  entries: ServerTuple[]
): Promise<void> {
  return invoke("add_servers_to_stored", { mode, entries });
}

export function moveServerToStored(
  mode: AppMode,
  name: string
): Promise<void> {
  return invoke("move_server_to_stored", { mode, name });
}

export function moveServerToActive(
  mode: AppMode,
  name: string
): Promise<string> {
  return invoke("move_server_to_active", { mode, name });
}

export function deleteServer(
  mode: AppMode,
  name: string,
  source: ServerSource
): Promise<void> {
  return invoke("delete_server", { mode, name, source });
}

export function updateServerConfig(
  mode: AppMode,
  name: string,
  source: ServerSource,
  newJson: string
): Promise<void> {
  return invoke("update_server_config", { mode, name, source, newJson });
}

// ---------- Backups ----------

export function listBackups(mode: AppMode): Promise<BackupFile[]> {
  return invoke<BackupFile[]>("list_backups", { mode });
}

export function restoreBackup(
  mode: AppMode,
  backupPath: string
): Promise<void> {
  return invoke("restore_backup", { mode, backupPath });
}

export function deleteBackup(backupPath: string): Promise<void> {
  return invoke("delete_backup", { backupPath });
}

export function forceBackup(mode: AppMode): Promise<void> {
  return invoke("force_backup", { mode });
}

/// Read a backup file (or the live config) as raw text. The backend only
/// allows paths that live under a configured backup directory or point at
/// one of the known config files, so this can't be used to read arbitrary
/// files on disk.
export function readBackupContent(path: string): Promise<string> {
  return invoke<string>("read_backup_content", { path });
}

// ---------- Marketplace catalog ----------

export function getCatalog(): Promise<Catalog> {
  return invoke<Catalog>("get_catalog");
}

export function refreshCatalog(): Promise<Catalog> {
  return invoke<Catalog>("refresh_catalog");
}

export function installFromCatalog(
  mode: AppMode,
  catalogId: string,
  target: ServerSource,
  customConfig: Record<string, unknown> | null,
  customName: string | null
): Promise<string> {
  return invoke<string>("install_from_catalog", {
    mode,
    catalogId,
    target,
    customConfig,
    customName,
  });
}

export function getCatalogLinks(
  mode: AppMode
): Promise<Record<string, string>> {
  return invoke<Record<string, string>>("get_catalog_links", { mode });
}

export function missingSecretsForServer(
  catalogId: string,
  configJson: string
): Promise<string[]> {
  return invoke<string[]>("missing_secrets_for_server", {
    catalogId,
    configJson,
  });
}

// ---------- Paths ----------

export function getConfigPath(mode: AppMode): Promise<string> {
  return invoke<string>("get_config_path", { mode });
}

export function getStorageDir(): Promise<string> {
  return invoke<string>("get_storage_dir");
}

export function getClaudeCodeSettingsPath(): Promise<string> {
  return invoke<string>("get_claude_code_settings_path");
}

export function getCommandsDir(): Promise<string> {
  return invoke<string>("get_commands_dir");
}

export function getSkillsDir(): Promise<string> {
  return invoke<string>("get_skills_dir");
}

/// macOS only: gracefully quit and relaunch Claude Desktop so config
/// changes take effect without the user having to do it manually.
export function restartClaudeDesktop(): Promise<void> {
  return invoke("restart_claude_desktop");
}

// ---------- Hooks ----------

export function listHooks(): Promise<HookRule[]> {
  return invoke<HookRule[]>("list_hooks");
}

export function getHookRuleJson(
  event: string,
  matcher: string
): Promise<string> {
  return invoke<string>("get_hook_rule_json", { event, matcher });
}

export function toggleHook(
  event: string,
  matcher: string,
  enable: boolean
): Promise<void> {
  return invoke("toggle_hook", { event, matcher, enable });
}

export function updateHookRule(
  event: string,
  matcher: string,
  newJson: string
): Promise<void> {
  return invoke("update_hook_rule", { event, matcher, newJson });
}

// ---------- Agents ----------

export function listAgents(): Promise<AgentEntry[]> {
  return invoke<AgentEntry[]>("list_agents");
}

export function createAgent(name: string): Promise<string> {
  return invoke<string>("create_agent", { name });
}

export function deleteAgent(filePath: string): Promise<void> {
  return invoke("delete_agent", { filePath });
}

// ---------- Skills ----------

export function listSkills(): Promise<SkillEntry[]> {
  return invoke<SkillEntry[]>("list_skills");
}

export function toggleSkill(
  filePath: string,
  source: SkillSource,
  currentlyEnabled: boolean
): Promise<void> {
  return invoke("toggle_skill", { filePath, source, currentlyEnabled });
}

export function createSkill(
  name: string,
  source: SkillSource
): Promise<string> {
  return invoke<string>("create_skill", { name, source });
}

// ---------- Shared (plugins + file I/O) ----------

export function togglePlugin(pluginKey: string): Promise<void> {
  return invoke("toggle_plugin", { pluginKey });
}

export function readClaudeFile(filePath: string): Promise<string> {
  return invoke<string>("read_claude_file", { filePath });
}

export function writeClaudeFile(
  filePath: string,
  content: string
): Promise<void> {
  return invoke("write_claude_file", { filePath, content });
}
