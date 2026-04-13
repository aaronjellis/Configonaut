// Marketplace catalog module — ported from Sources/Marketplace/*.swift.
//
// This collapses four Swift files into one Rust module since the frontend
// only ever talks to us through a handful of Tauri commands and we don't need
// the @MainActor/ObservableObject machinery on this side:
//
//   CatalogModels.swift          → `Catalog*` structs below
//   CatalogStore.swift           → `bootstrap_catalog` / `refresh_catalog`
//   ConfigManager+Catalog.swift  → `install_from_catalog` + link persistence
//   SecretValidator.swift        → `looks_like_placeholder` / `missing_secrets`
//
// Catalog JSON shape is identical to the Swift decoder — same camelCase field
// names on the wire — so the existing `Resources/catalog-baseline.json` parses
// unchanged, and remote catalogs keep working across both app builds.

use std::collections::HashSet;
use std::fs;
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::{anyhow, Context};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::config;
use crate::models::{AppMode, AppResult, ServerSource};
use crate::paths;

// ---------------------------------------------------------------------------
// Catalog data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub generated_at: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub categories: Vec<CatalogCategory>,
    #[serde(default)]
    pub servers: Vec<CatalogServer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogCategory {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogPublisher {
    pub name: String,
    /// `official`, `vendor`, or `community` — matches the Swift enum as a string
    /// so we don't have to maintain a parallel Rust enum + migration.
    #[serde(rename = "type", default = "default_publisher_type")]
    pub kind: String,
    #[serde(default)]
    pub verified: bool,
}

fn default_publisher_type() -> String {
    "community".to_string()
}

fn default_transport() -> String {
    "stdio".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogServer {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub publisher: CatalogPublisher,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub repository: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub popularity: i64,
    pub config: CatalogConfig,
    /// `"stdio"` or `"remote"`. Older catalogs may omit this — we default to
    /// `"stdio"` because that's the common case and the one that actually has
    /// runtime prerequisites.
    #[serde(default = "default_transport")]
    pub transport: String,
    /// Runtime prerequisites, e.g. `["node"]`, `["python", "uv"]`, `["docker"]`.
    /// Empty means no special runtime is needed.
    #[serde(default)]
    pub requirements: Vec<String>,
    #[serde(default)]
    pub setup_notes: Option<String>,
    /// Optional because older catalogs may omit this entirely — the baseline
    /// shipped with the app always includes at least an empty array.
    #[serde(default)]
    pub env_vars: Option<Vec<CatalogEnvVar>>,
    /// Which feed this server was loaded from. Not present in the raw catalog
    /// JSON — injected during the merge step so the UI can group by source.
    #[serde(default)]
    pub feed_origin: Option<String>,
}

impl CatalogServer {
    pub fn required_env_vars(&self) -> Vec<&CatalogEnvVar> {
        self.env_vars
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .filter(|v| v.required)
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogEnvVar {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub secret: bool,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub help_url: Option<String>,
}

/// Config block — either stdio (command + args + env) or http (url + headers).
/// Both field sets are optional because any given server uses one or the other,
/// and we mirror whichever keys the catalog author wrote.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogConfig {
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Option<Vec<String>>,
    #[serde(default)]
    pub env: Option<Map<String, Value>>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub headers: Option<Map<String, Value>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    Stdio,
    Http,
}

impl CatalogConfig {
    pub fn transport(&self) -> Transport {
        if self.url.is_some() {
            Transport::Http
        } else {
            Transport::Stdio
        }
    }

    /// Flatten the structured config into the `[String: Any]`-shaped map we
    /// actually persist inside `mcpServers["name"]`. Only keys the catalog
    /// author specified are included — empty arrays stay empty, missing
    /// headers stay missing — so diffs against the user's current config
    /// stay minimal.
    pub fn to_config_dict(&self) -> Map<String, Value> {
        let mut m = Map::new();
        match self.transport() {
            Transport::Stdio => {
                if let Some(cmd) = &self.command {
                    m.insert("command".into(), Value::String(cmd.clone()));
                }
                if let Some(args) = &self.args {
                    m.insert(
                        "args".into(),
                        Value::Array(args.iter().cloned().map(Value::String).collect()),
                    );
                }
                if let Some(env) = &self.env {
                    m.insert("env".into(), Value::Object(env.clone()));
                }
            }
            Transport::Http => {
                if let Some(url) = &self.url {
                    m.insert("url".into(), Value::String(url.clone()));
                }
                if let Some(headers) = &self.headers {
                    m.insert("headers".into(), Value::Object(headers.clone()));
                }
            }
        }
        m
    }
}

// ---------------------------------------------------------------------------
// Catalog store — bootstrap + remote refresh
// ---------------------------------------------------------------------------

/// The Swift app reads this from UserDefaults so power users can point at a
/// staging catalog. We don't carry UserDefaults on this side, so we fall back
/// to the production repo. If that ever needs to change, swap in an env var
/// read here.
const REMOTE_URL: &str =
    "https://raw.githubusercontent.com/aaronjellis/configonaut-catalog/main/catalog.json";

/// Embedded fallback catalog. `cargo build` inlines the JSON into the binary
/// so a brand-new install with no network can still show the Marketplace tab.
const BASELINE_CATALOG_JSON: &str =
    include_str!("../resources/catalog-baseline.json");

/// Load the catalog using the cache-first strategy from Swift's `CatalogStore`:
///
///   1. If `~/…/Configonaut/catalog-cache.json` exists and parses, use that.
///   2. Otherwise fall back to the embedded baseline.
///
/// The async `refresh_catalog` command can then replace the cache at any time
/// without blocking the initial load.
pub fn bootstrap_catalog() -> AppResult<Catalog> {
    if let Some(cached) = load_cached_catalog() {
        return Ok(cached);
    }
    serde_json::from_str::<Catalog>(BASELINE_CATALOG_JSON)
        .map_err(|e| anyhow!("baseline catalog parse failed: {e}").into())
}

fn load_cached_catalog() -> Option<Catalog> {
    let path = paths::catalog_cache_file();
    if !path.exists() {
        return None;
    }
    let raw = fs::read_to_string(&path).ok()?;
    serde_json::from_str::<Catalog>(&raw).ok()
}

/// Fetch the latest catalog from GitHub and persist it to the local cache.
///
/// Mirrors `CatalogStore.refreshRemote`:
///   • 10-second timeout
///   • Reject non-2xx responses
///   • Validate the payload parses before writing the cache (so a bad push
///     upstream can't corrupt a working install)
///   • Write the cache file atomically via a sibling `.tmp`
pub async fn refresh_catalog_remote() -> AppResult<Catalog> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("build reqwest client")?;

    let resp = client
        .get(REMOTE_URL)
        .send()
        .await
        .context("fetch catalog")?;
    if !resp.status().is_success() {
        return Err(anyhow!("catalog fetch HTTP {}", resp.status()).into());
    }
    let text = resp.text().await.context("read catalog body")?;

    // Parse before we touch disk — if it's bogus we want to surface the error
    // and leave the existing cache intact.
    let catalog: Catalog = serde_json::from_str(&text)
        .context("parse remote catalog JSON")?;

    write_cache_atomic(&text)?;
    Ok(catalog)
}

fn write_cache_atomic(raw: &str) -> AppResult<()> {
    let path = paths::catalog_cache_file();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = match path.parent() {
        Some(parent) => parent.join(".catalog-cache.json.tmp"),
        None => path.with_extension("tmp"),
    };
    fs::write(&tmp, raw)?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Custom catalog feeds
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedEntry {
    pub id: String,
    pub label: String,
    pub url: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedConfig {
    pub feeds: Vec<FeedEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedStatus {
    pub id: String,
    pub label: String,
    pub url: String,
    pub enabled: bool,
    pub server_count: usize,
    pub error: Option<String>,
    pub using_cache: bool,
}

fn generate_feed_id(url: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

// ── Feed config persistence ─────────────────────────────────────────────

pub fn load_feed_config() -> FeedConfig {
    let path = paths::feeds_config_file();
    if !path.exists() {
        return FeedConfig { feeds: vec![] };
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or(FeedConfig { feeds: vec![] })
}

fn save_feed_config(config: &FeedConfig) -> AppResult<()> {
    let path = paths::feeds_config_file();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(config)?;
    fs::write(&path, json)?;
    Ok(())
}

// ── Feed CRUD ───────────────────────────────────────────────────────────

pub fn add_feed(label: String, url: String) -> AppResult<FeedEntry> {
    let mut config = load_feed_config();
    if config.feeds.iter().any(|f| f.url == url) {
        return Err(anyhow!("A feed with this URL already exists.").into());
    }
    let entry = FeedEntry {
        id: generate_feed_id(&url),
        label,
        url,
        enabled: true,
    };
    config.feeds.push(entry.clone());
    save_feed_config(&config)?;
    Ok(entry)
}

pub fn remove_feed(feed_id: &str) -> AppResult<()> {
    let mut config = load_feed_config();
    config.feeds.retain(|f| f.id != feed_id);
    save_feed_config(&config)?;
    let _ = fs::remove_file(paths::feed_cache_file(feed_id));
    Ok(())
}

pub fn toggle_feed(feed_id: &str, enabled: bool) -> AppResult<()> {
    let mut config = load_feed_config();
    if let Some(entry) = config.feeds.iter_mut().find(|f| f.id == feed_id) {
        entry.enabled = enabled;
    }
    save_feed_config(&config)?;
    Ok(())
}

// ── Per-feed cache ──────────────────────────────────────────────────────

fn load_feed_cache(feed_id: &str) -> Option<Catalog> {
    let path = paths::feed_cache_file(feed_id);
    if !path.exists() {
        return None;
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
}

fn write_feed_cache(feed_id: &str, raw: &str) -> AppResult<()> {
    let path = paths::feed_cache_file(feed_id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, raw)?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

// ── Fetch a single feed ─────────────────────────────────────────────────

/// Returns (catalog, error, used_cache).
async fn fetch_single_feed(entry: &FeedEntry) -> (Option<Catalog>, Option<String>, bool) {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => return (load_feed_cache(&entry.id), Some(e.to_string()), true),
    };

    match client.get(&entry.url).send().await {
        Ok(resp) if resp.status().is_success() => match resp.text().await {
            Ok(text) => match serde_json::from_str::<Catalog>(&text) {
                Ok(catalog) => {
                    let _ = write_feed_cache(&entry.id, &text);
                    (Some(catalog), None, false)
                }
                Err(e) => (
                    load_feed_cache(&entry.id),
                    Some(format!("parse error: {e}")),
                    true,
                ),
            },
            Err(e) => (load_feed_cache(&entry.id), Some(e.to_string()), true),
        },
        Ok(resp) => {
            let status = resp.status();
            (
                load_feed_cache(&entry.id),
                Some(format!("HTTP {status}")),
                true,
            )
        }
        Err(e) => (load_feed_cache(&entry.id), Some(e.to_string()), true),
    }
}

// ── Merge multiple catalogs ─────────────────────────────────────────────

/// Custom feed servers come first; built-in last. First occurrence of a
/// server ID wins (dedup).
fn merge_catalogs(
    feed_catalogs: Vec<(String, Catalog)>,
    builtin: Catalog,
) -> Catalog {
    let mut seen_ids = HashSet::new();
    let mut merged_servers = Vec::new();
    let mut all_categories = Vec::new();
    let mut seen_cat_ids = HashSet::new();

    let mut all: Vec<(String, Catalog)> = feed_catalogs;
    all.push(("Built-in".into(), builtin));

    for (feed_label, catalog) in all {
        for cat in catalog.categories {
            if seen_cat_ids.insert(cat.id.clone()) {
                all_categories.push(cat);
            }
        }
        for mut server in catalog.servers {
            if seen_ids.insert(server.id.clone()) {
                server.feed_origin = Some(feed_label.clone());
                merged_servers.push(server);
            }
        }
    }

    Catalog {
        version: String::new(),
        generated_at: String::new(),
        source: None,
        categories: all_categories,
        servers: merged_servers,
    }
}

// ── Top-level feed-aware bootstrap + refresh ────────────────────────────

/// Synchronous startup load: cached custom feeds + cached/baseline built-in.
pub fn bootstrap_catalog_with_feeds() -> AppResult<(Catalog, Vec<FeedStatus>)> {
    let config = load_feed_config();
    let mut feed_catalogs = Vec::new();
    let mut statuses = Vec::new();

    for entry in &config.feeds {
        if !entry.enabled {
            statuses.push(FeedStatus {
                id: entry.id.clone(),
                label: entry.label.clone(),
                url: entry.url.clone(),
                enabled: false,
                server_count: 0,
                error: None,
                using_cache: false,
            });
            continue;
        }
        if let Some(cached) = load_feed_cache(&entry.id) {
            let count = cached.servers.len();
            feed_catalogs.push((entry.label.clone(), cached));
            statuses.push(FeedStatus {
                id: entry.id.clone(),
                label: entry.label.clone(),
                url: entry.url.clone(),
                enabled: true,
                server_count: count,
                error: None,
                using_cache: true,
            });
        }
    }

    let builtin = bootstrap_catalog()?;
    let merged = merge_catalogs(feed_catalogs, builtin);
    Ok((merged, statuses))
}

/// Async: fetch all enabled feeds + built-in, merge, return unified catalog
/// plus per-feed status reports.
pub async fn refresh_all_feeds() -> AppResult<(Catalog, Vec<FeedStatus>)> {
    let config = load_feed_config();
    let mut statuses = Vec::new();
    let mut feed_catalogs = Vec::new();

    for entry in &config.feeds {
        if !entry.enabled {
            statuses.push(FeedStatus {
                id: entry.id.clone(),
                label: entry.label.clone(),
                url: entry.url.clone(),
                enabled: false,
                server_count: 0,
                error: None,
                using_cache: false,
            });
            continue;
        }
        let (catalog_opt, error, used_cache) = fetch_single_feed(entry).await;
        if let Some(catalog) = catalog_opt {
            let count = catalog.servers.len();
            feed_catalogs.push((entry.label.clone(), catalog));
            statuses.push(FeedStatus {
                id: entry.id.clone(),
                label: entry.label.clone(),
                url: entry.url.clone(),
                enabled: true,
                server_count: count,
                error,
                using_cache: used_cache,
            });
        } else {
            statuses.push(FeedStatus {
                id: entry.id.clone(),
                label: entry.label.clone(),
                url: entry.url.clone(),
                enabled: true,
                server_count: 0,
                error: error.or_else(|| Some("Feed unavailable".into())),
                using_cache: false,
            });
        }
    }

    let builtin = match refresh_catalog_remote().await {
        Ok(c) => c,
        Err(_) => bootstrap_catalog()?,
    };

    let merged = merge_catalogs(feed_catalogs, builtin);
    Ok((merged, statuses))
}

// ---------------------------------------------------------------------------
// Catalog links — per-mode map of server-name → catalog-id
// ---------------------------------------------------------------------------

/// Load the catalog_links_<mode>.json sidecar. Returns an empty map if the
/// file doesn't exist yet (which is the norm on a fresh install).
pub fn load_catalog_links(mode: AppMode) -> AppResult<Map<String, Value>> {
    let path = paths::catalog_links_file(mode);
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

fn save_catalog_links(mode: AppMode, links: &Map<String, Value>) -> AppResult<()> {
    let path = paths::catalog_links_file(mode);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&Value::Object(links.clone()))?;
    fs::write(&path, json)?;
    Ok(())
}

fn record_catalog_link(
    mode: AppMode,
    server_name: &str,
    catalog_id: &str,
) -> AppResult<()> {
    let mut links = load_catalog_links(mode)?;
    links.insert(
        server_name.to_string(),
        Value::String(catalog_id.to_string()),
    );
    save_catalog_links(mode, &links)
}

// ---------------------------------------------------------------------------
// Install flow
// ---------------------------------------------------------------------------

/// Pick a name that doesn't already exist in the active or stored sets.
/// Matches `ConfigManager.resolveUniqueName`: preferred name first, then
/// `-2`, `-3`, … until a free slot appears.
fn resolve_unique_name(mode: AppMode, base: &str) -> AppResult<String> {
    let listing = config::list_servers(mode)?;
    let existing: HashSet<String> = listing
        .active_servers
        .iter()
        .map(|s| s.name.clone())
        .chain(listing.stored_servers.iter().map(|s| s.name.clone()))
        .collect();
    if !existing.contains(base) {
        return Ok(base.to_string());
    }
    for i in 2..1000 {
        let candidate = format!("{base}-{i}");
        if !existing.contains(&candidate) {
            return Ok(candidate);
        }
    }
    Err(anyhow!("could not find a unique name for {base}").into())
}

/// Install a catalog entry into either the active or stored (inactive) list.
///
/// If `custom_config` is provided, it's what actually gets persisted — the
/// Marketplace tab's inline JSON editor passes this when the user has tweaked
/// the default template. Otherwise we fall back to `server.config.to_config_dict()`.
///
/// Returns the final server name (may differ from `custom_name` / the catalog
/// id if a collision was resolved).
pub fn install_from_catalog(
    mode: AppMode,
    catalog_id: &str,
    target: ServerSource,
    custom_config: Option<Map<String, Value>>,
    custom_name: Option<String>,
) -> AppResult<String> {
    let (catalog, _) = bootstrap_catalog_with_feeds()?;
    let server = catalog
        .servers
        .iter()
        .find(|s| s.id == catalog_id)
        .ok_or_else(|| anyhow!("catalog id not found: {catalog_id}"))?;

    let base_name = custom_name
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| server.id.clone());

    let unique_name = resolve_unique_name(mode, &base_name)?;

    let config_map = custom_config.unwrap_or_else(|| server.config.to_config_dict());
    let entries = vec![(unique_name.clone(), Value::Object(config_map))];

    match target {
        ServerSource::Active => config::add_to_active(mode, entries)?,
        ServerSource::Stored => config::add_to_stored(mode, entries)?,
    }

    record_catalog_link(mode, &unique_name, &server.id)?;
    Ok(unique_name)
}

// ---------------------------------------------------------------------------
// Secret validator — placeholder heuristics
// ---------------------------------------------------------------------------

fn placeholder_patterns() -> &'static [Regex] {
    static PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        [
            // <YOUR_TOKEN>, <token>, <changeme>
            r"^<[^>]+>$",
            // ${SOMETHING}
            r"^\$\{[^}]+\}$",
            // "your-api-key", "YOUR_TOKEN", "your_key_here"
            r"(?i)\byour[_\- ]?(?:api[_\- ]?)?(?:key|token|secret|pat)\b",
            // "xxxxxx..." or "xxxx-xxxx-xxxx"
            r"^x{4,}([_\- ]?x{2,})*$",
            // "paste-here", "paste_your_key_here", "replace_me"
            r"(?i)\b(paste|replace|insert|fill)[_\- ]?(here|me|token|key)?\b",
            // "changeme", "change-me"
            r"(?i)^change[_\- ]?me$",
            // Bearer-prefix with obvious placeholder inside.
            r"(?i)bearer\s+<[^>]+>",
            r"(?i)bearer\s+your[_\- ]?(?:api[_\- ]?)?(?:key|token)",
            // Common "todo" style placeholders.
            r"(?i)^(todo|tbd|fixme)$",
        ]
        .iter()
        .map(|p| Regex::new(p).expect("placeholder pattern should compile"))
        .collect()
    })
}

/// Best-effort detection of unfilled secrets. Matches
/// `SecretValidator.looksLikePlaceholder` in the Swift port — same patterns,
/// same "err on the side of 'looks real'" bias.
pub fn looks_like_placeholder(raw: &str, hint: Option<&str>) -> bool {
    let value = raw.trim();
    if value.is_empty() {
        return true;
    }
    if value.starts_with('$') || value.starts_with("${") {
        return true;
    }
    for re in placeholder_patterns() {
        if re.is_match(value) {
            return true;
        }
    }
    if let Some(h) = hint {
        if !h.is_empty() && h == value {
            return true;
        }
    }
    false
}

/// Given the `mcpServers["name"]` config dict for an installed server and the
/// catalog entry it was linked to, return the names of required env vars that
/// still hold a placeholder / empty value. An empty list means "ready to run".
pub fn missing_secrets(config_block: &Value, server: &CatalogServer) -> Vec<String> {
    let mut missing: Vec<String> = Vec::new();
    let transport = server.config.transport();

    let empty = Map::new();
    let env_dict = config_block
        .get("env")
        .and_then(|v| v.as_object())
        .unwrap_or(&empty);
    let headers_dict = config_block
        .get("headers")
        .and_then(|v| v.as_object())
        .unwrap_or(&empty);
    let url_string = config_block
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    for env_var in server.required_env_vars() {
        let name = &env_var.name;
        let hint = env_var.placeholder.as_deref();
        match transport {
            Transport::Stdio => {
                // Expect the token in env[name].
                let raw = env_dict
                    .get(name)
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if looks_like_placeholder(raw, hint) {
                    missing.push(name.clone());
                }
            }
            Transport::Http => {
                // For remote servers, the "env var" is usually baked into a
                // header (e.g. `Authorization: Bearer <TOKEN>`) or into the
                // URL itself.
                let header_joined: String = headers_dict
                    .values()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                let haystack = format!("{header_joined} {url_string}");
                if haystack.trim().is_empty()
                    || looks_like_placeholder(&haystack, hint)
                {
                    missing.push(name.clone());
                    continue;
                }
                // Also flag if the haystack still literally contains `<NAME>`.
                if haystack.contains(&format!("<{name}>"))
                    || haystack.contains(&format!("${{{name}}}"))
                {
                    missing.push(name.clone());
                }
            }
        }
    }
    missing
}
