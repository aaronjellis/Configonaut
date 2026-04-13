// TypeScript mirrors of the Rust types in src-tauri/src/models.rs.
// Keep these in sync by hand — the wire format uses camelCase (via
// serde's rename_all) so the field names here match what we see in JSON.

export type AppMode = "desktop" | "cli";

export type ServerSource = "active" | "stored";

export interface ServerEntry {
  name: string;
  configJson: string;
}

export interface ProjectMcpGroup {
  projectPath: string;
  servers: ServerEntry[];
}

export interface ServerListing {
  activeServers: ServerEntry[];
  storedServers: ServerEntry[];
  configPath: string;
  needsRestart: boolean;
  projectGroups: ProjectMcpGroup[];
}

export interface HookRule {
  id: string;
  event: string;
  matcher: string;
  commands: string[];
  isEnabled: boolean;
}

export interface BackupFile {
  path: string;
  fileName: string;
  createdAt: string;
  sizeBytes: number;
}

export type AgentSource = "personal" | "plugin";

export interface AgentEntry {
  name: string;
  description: string;
  tools: string[];
  model: string;
  color: string;
  pluginName: string;
  filePath: string;
  source: AgentSource;
  isPluginEnabled: boolean;
}

export type SkillSource = "command" | "skill" | "plugin";

export interface SkillEntry {
  name: string;
  description: string;
  source: SkillSource;
  filePath: string;
  isEnabled: boolean;
}

// Tuple form used when passing new entries into add commands.
// Serde deserializes Vec<(String, Value)> from a JS array of [name, config].
export type ServerTuple = [string, unknown];

// ---------------------------------------------------------------------------
// Marketplace catalog — mirrors src-tauri/src/catalog.rs. All field names
// are camelCase on the wire (serde rename_all) so these match the raw catalog
// JSON the backend returns.
// ---------------------------------------------------------------------------

export interface Catalog {
  version: string;
  generatedAt: string;
  source?: string | null;
  categories: CatalogCategory[];
  servers: CatalogServer[];
}

export interface CatalogCategory {
  id: string;
  label: string;
  icon?: string | null;
  description?: string | null;
}

export interface CatalogPublisher {
  name: string;
  /// "official" | "vendor" | "community"
  type: string;
  verified: boolean;
}

export interface CatalogServer {
  id: string;
  name: string;
  description: string;
  category: string;
  tags: string[];
  publisher: CatalogPublisher;
  homepage?: string | null;
  repository?: string | null;
  license?: string | null;
  popularity: number;
  config: CatalogConfig;
  transport: string;
  requirements: string[];
  setupNotes?: string | null;
  envVars?: CatalogEnvVar[] | null;
  feedOrigin?: string | null;
}

export interface RuntimeStatus {
  node: string | null;
  python: string | null;
  uv: string | null;
  docker: string | null;
}

export interface CatalogEnvVar {
  name: string;
  description?: string | null;
  required: boolean;
  secret: boolean;
  placeholder?: string | null;
  helpUrl?: string | null;
}

export interface CatalogConfig {
  command?: string | null;
  args?: string[] | null;
  env?: Record<string, unknown> | null;
  url?: string | null;
  headers?: Record<string, unknown> | null;
}

// ---------------------------------------------------------------------------
// Custom catalog feeds
// ---------------------------------------------------------------------------

export interface FeedEntry {
  id: string;
  label: string;
  url: string;
  enabled: boolean;
}

export interface FeedStatus {
  id: string;
  label: string;
  url: string;
  enabled: boolean;
  serverCount: number;
  error: string | null;
  usingCache: boolean;
}

// Top-level sidebar sections. Marketplace is intentionally NOT here — in the
// Swift version it lives as a tab inside the Add Server flow, not as its own
// view.
export type ViewKey = "mcp" | "hooks" | "agents" | "skills" | "backups";
