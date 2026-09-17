// Hooks, agents, skills, and plugin toggles — all the Claude Code
// ecosystem bits that hang off ~/.claude and settings.json.
//
// Split out from config.rs (which owns mcpServers) because none of this
// touches the MCP pipeline and the file formats are totally different —
// YAML-style frontmatter, directory scans, file moves for enable/disable.
//
// Shape of the data:
//
//   ~/.claude/settings.json
//     {
//       "hooks": { "<Event>": [ { "matcher": "...",
//                                  "hooks": [ { "command": "..." } ] } ] },
//       "enabledPlugins": { "<pluginName>@<marketplace>": bool },
//       ...
//     }
//   <storage>/disabled_hooks.json         (rules the user switched off — see below)
//   ~/.claude/agents/*.md                       (personal agents)
//   ~/.claude/commands/*.md                     (custom slash commands)
//   ~/.claude/commands/.disabled/*.md           (hidden/off)
//   ~/.claude/skills/<name>/SKILL.md            (custom skills — what we create)
//   ~/.claude/skills/<name>.md                  (legacy flat layout — still read)
//   ~/.claude/skills/.disabled/...              (hidden/off)
//   ~/.claude/plugins/installed_plugins.json    (index: "<plugin>@<marketplace>" → installPath)
//   <installPath>/agents/*.md                   (read-only plugin agents)
//   <installPath>/skills/<name>/SKILL.md        (read-only plugin skills)

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context};
use serde_json::{json, Map, Value};

use crate::models::{
    AgentEntry, AgentSource, AppResult, HookRule, LegacyMigrationResult, SkillEntry, SkillSource,
};
use crate::paths;

// ---------------------------------------------------------------------------
// Shared helpers — settings.json I/O + frontmatter parser
// ---------------------------------------------------------------------------

fn load_settings() -> AppResult<Map<String, Value>> {
    let path = paths::claude_code_settings();
    if !path.exists() {
        return Ok(Map::new());
    }
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.display()))?;
    if raw.trim().is_empty() {
        return Ok(Map::new());
    }
    let parsed: Value = serde_json::from_str(&raw)
        .with_context(|| format!("parse {}", path.display()))?;
    match parsed {
        Value::Object(m) => Ok(m),
        _ => Err(anyhow!("settings.json is not a JSON object").into()),
    }
}

/// Persist a mutated settings.json. Same atomic-write pattern as the MCP
/// config path: write a sibling temp file and rename over it so a crash
/// can't truncate the live file.
fn save_settings(settings: &Map<String, Value>) -> AppResult<()> {
    let path = paths::claude_code_settings();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    // Sort keys so the file is diff-friendly — matches the Swift version
    // which used `.sortedKeys`.
    let sorted = sort_keys(&Value::Object(settings.clone()));
    let json = serde_json::to_string_pretty(&sorted)?;
    crate::config::write_atomic(&path, json.as_bytes())
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

/// Parse a YAML-style `---` frontmatter block at the top of a markdown file.
/// We only handle simple `key: value` pairs — no nested structures, no multi-
/// line strings — because that's all the existing agent/skill files use.
/// Returns an empty map if the file doesn't start with `---`.
fn parse_frontmatter(content: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut lines = content.lines();
    let Some(first) = lines.next() else { return out };
    if first.trim() != "---" {
        return out;
    }
    for line in lines {
        let trimmed = line.trim();
        if trimmed == "---" {
            break;
        }
        // Only split on the first ": " so descriptions with colons survive.
        let Some(idx) = trimmed.find(": ") else { continue };
        let key = trimmed[..idx].trim().to_string();
        let mut value = trimmed[idx + 2..].trim().to_string();
        // Strip wrapping quotes the way the Swift version did.
        if (value.starts_with('"') && value.ends_with('"') && value.len() >= 2)
            || (value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2)
        {
            value = value[1..value.len() - 1].to_string();
        }
        out.insert(key, value);
    }
    out
}

// ---------------------------------------------------------------------------
// Hooks
// ---------------------------------------------------------------------------
//
// Two stores share one shape — `{ "<Event>": [ rule, ... ] }`:
//
//   • settings.json → "hooks"            enabled rules (what Claude Code runs)
//   • <storage>/disabled_hooks.json      rules the user switched off
//
// Claude Code has no per-rule disable flag, so "off" means "not in
// settings.json". Older Configonaut builds wrote `"disabled": true` on the
// rule instead; `migrate_legacy_disabled` moves those to the sidecar the
// first time the list is read.

fn load_disabled_hooks() -> AppResult<Map<String, Value>> {
    let path = paths::disabled_hooks_file();
    if !path.exists() {
        return Ok(Map::new());
    }
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.display()))?;
    if raw.trim().is_empty() {
        return Ok(Map::new());
    }
    match serde_json::from_str::<Value>(&raw)
        .with_context(|| format!("parse {}", path.display()))?
    {
        Value::Object(m) => Ok(m),
        _ => Ok(Map::new()),
    }
}

fn save_disabled_hooks(disabled: &Map<String, Value>) -> AppResult<()> {
    let path = paths::disabled_hooks_file();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&Value::Object(disabled.clone()))?;
    crate::config::write_atomic(&path, json.as_bytes())
}

/// Borrow settings.json's `hooks` object, creating it if absent. Errors
/// instead of clobbering an unexpected non-object value.
fn settings_hooks_mut(settings: &mut Map<String, Value>) -> AppResult<&mut Map<String, Value>> {
    let entry = settings
        .entry("hooks".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !entry.is_object() {
        return Err(anyhow!("'hooks' in settings.json is not a JSON object").into());
    }
    Ok(entry.as_object_mut().expect("hooks is an object"))
}

fn rule_matcher(rule: &Value) -> &str {
    rule.as_object()
        .and_then(|m| m.get("matcher"))
        .and_then(|v| v.as_str())
        .unwrap_or("*")
}

/// Position of the rule under `matcher` within an event's rule array.
fn rule_index(arr: &[Value], matcher: &str) -> Option<usize> {
    arr.iter().position(|r| rule_matcher(r) == matcher)
}

/// Replace a rule in place. Returns false if no rule matched.
fn replace_rule(hooks: &mut Map<String, Value>, event: &str, matcher: &str, new_rule: Value) -> bool {
    let Some(arr) = hooks.get_mut(event).and_then(|v| v.as_array_mut()) else { return false };
    match rule_index(arr, matcher) {
        Some(idx) => {
            arr[idx] = new_rule;
            true
        }
        None => false,
    }
}

/// Remove and return the rule under `event` whose matcher equals `matcher`.
/// Prunes the event key when its array becomes empty.
fn take_rule(hooks: &mut Map<String, Value>, event: &str, matcher: &str) -> Option<Value> {
    let arr = hooks.get_mut(event)?.as_array_mut()?;
    let idx = rule_index(arr, matcher)?;
    let rule = arr.remove(idx);
    if arr.is_empty() {
        hooks.shift_remove(event);
    }
    Some(rule)
}

/// Insert a rule under `event`, replacing any existing rule with the same
/// matcher in place rather than duplicating it. Errors if the event key
/// already holds something other than an array.
fn put_rule(hooks: &mut Map<String, Value>, event: &str, rule: Value) -> AppResult<()> {
    let entry = hooks
        .entry(event.to_string())
        .or_insert_with(|| Value::Array(vec![]));
    if !entry.is_array() {
        return Err(anyhow!("'hooks.{event}' is not a JSON array").into());
    }
    let arr = entry.as_array_mut().expect("event is an array");
    match rule_index(arr, rule_matcher(&rule)) {
        Some(idx) => arr[idx] = rule,
        None => arr.push(rule),
    }
    Ok(())
}

fn find_rule<'a>(hooks: &'a Map<String, Value>, event: &str, matcher: &str) -> Option<&'a Value> {
    hooks
        .get(event)?
        .as_array()?
        .iter()
        .find(|r| rule_matcher(r) == matcher)
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(n).collect::<String>())
    }
}

/// One-line description of a handler for the list view.
fn summarize_handler(h: &Value) -> Option<String> {
    let m = h.as_object()?;
    let kind = m.get("type").and_then(|v| v.as_str()).unwrap_or("command");
    let s = |k: &str| m.get(k).and_then(|v| v.as_str()).map(str::to_string);
    match kind {
        "command" => s("command"),
        "prompt" => s("prompt").map(|p| format!("prompt: {}", truncate(&p, 80))),
        "agent" => s("prompt").map(|p| format!("agent: {}", truncate(&p, 80))),
        "http" => s("url").map(|u| format!("http: {u}")),
        "mcp_tool" => Some(format!(
            "mcp_tool: {}/{}",
            s("server").unwrap_or_default(),
            s("tool").unwrap_or_default()
        )),
        other => Some(format!("{other} hook")),
    }
}

fn rule_to_hook_rule(event: &str, rule: &Value, is_enabled: bool) -> Option<HookRule> {
    let m = rule.as_object()?;
    let matcher = m
        .get("matcher")
        .and_then(|v| v.as_str())
        .unwrap_or("*")
        .to_string();
    let handlers: Vec<&Value> = m
        .get("hooks")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().collect())
        .unwrap_or_default();
    let (commands, handler_types): (Vec<String>, Vec<String>) = handlers
        .iter()
        .filter_map(|h| {
            let kind = h.get("type").and_then(|v| v.as_str()).unwrap_or("command");
            summarize_handler(h).map(|s| (s, kind.to_string()))
        })
        .unzip();
    Some(HookRule {
        id: format!("{event}::{matcher}"),
        event: event.to_string(),
        matcher,
        commands,
        handler_types,
        is_enabled,
    })
}

/// Move rules carrying the legacy `"disabled": true` flag out of settings.json
/// and into the sidecar. Returns true if anything moved.
fn migrate_legacy_disabled(
    hooks: &mut Map<String, Value>,
    disabled: &mut Map<String, Value>,
) -> AppResult<bool> {
    let mut changed = false;
    let events: Vec<String> = hooks.keys().cloned().collect();
    for event in events {
        let Some(Value::Array(arr)) = hooks.get_mut(&event) else { continue };
        let mut i = 0;
        while i < arr.len() {
            let flagged = arr[i]
                .get("disabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if flagged {
                let mut rule = arr.remove(i);
                if let Some(m) = rule.as_object_mut() {
                    m.shift_remove("disabled");
                }
                put_rule(disabled, &event, rule)?;
                changed = true;
            } else {
                i += 1;
            }
        }
        if arr.is_empty() {
            hooks.shift_remove(&event);
        }
    }
    Ok(changed)
}

fn collect_rules(hooks: &Map<String, Value>, is_enabled: bool, out: &mut Vec<HookRule>) {
    for (event, rules) in hooks {
        let Value::Array(arr) = rules else { continue };
        for rule in arr {
            if let Some(hr) = rule_to_hook_rule(event, rule, is_enabled) {
                out.push(hr);
            }
        }
    }
}

/// Every hook rule from both stores, sorted by event then matcher.
pub fn list_hooks() -> AppResult<Vec<HookRule>> {
    let mut settings = load_settings()?;
    let mut disabled = load_disabled_hooks()?;

    let mut migrated = false;
    if let Some(Value::Object(hooks)) = settings.get_mut("hooks") {
        migrated = migrate_legacy_disabled(hooks, &mut disabled)?;
    }
    if migrated {
        // Sidecar first: if the settings.json write then fails, the rule
        // exists in both stores (duplicated) rather than in neither (lost).
        save_disabled_hooks(&disabled)?;
        save_settings(&settings)?;
    }

    let mut out: Vec<HookRule> = Vec::new();
    if let Some(Value::Object(hooks)) = settings.get("hooks") {
        collect_rules(hooks, true, &mut out);
    }
    // A sidecar rule whose event+matcher already exists in settings.json is
    // stale — skip it instead of listing the same rule twice.
    let settings_ids: HashSet<String> = out.iter().map(|hr| hr.id.clone()).collect();
    let mut disabled_out: Vec<HookRule> = Vec::new();
    collect_rules(&disabled, false, &mut disabled_out);
    out.extend(disabled_out.into_iter().filter(|hr| !settings_ids.contains(&hr.id)));
    out.sort_by(|a, b| a.event.cmp(&b.event).then(a.matcher.cmp(&b.matcher)));
    Ok(out)
}

/// Pretty JSON for one rule, from whichever store holds it.
pub fn hook_rule_json(event: &str, matcher: &str) -> AppResult<String> {
    let settings = load_settings()?;
    if let Some(Value::Object(hooks)) = settings.get("hooks") {
        if let Some(rule) = find_rule(hooks, event, matcher) {
            return Ok(serde_json::to_string_pretty(rule).unwrap_or_else(|_| "{}".into()));
        }
    }
    let disabled = load_disabled_hooks()?;
    if let Some(rule) = find_rule(&disabled, event, matcher) {
        return Ok(serde_json::to_string_pretty(rule).unwrap_or_else(|_| "{}".into()));
    }
    Ok("{}".to_string())
}

/// Enable = move sidecar → settings.json. Disable = move settings.json → sidecar.
pub fn toggle_hook(event: &str, matcher: &str, enable: bool) -> AppResult<()> {
    let mut settings = load_settings()?;
    let mut disabled = load_disabled_hooks()?;
    if enable {
        let rule = take_rule(&mut disabled, event, matcher)
            .ok_or_else(|| anyhow!("no disabled hook matched {event}/{matcher}"))?;
        put_rule(settings_hooks_mut(&mut settings)?, event, rule)?;
        save_settings(&settings)?;
        save_disabled_hooks(&disabled)
    } else {
        let hooks = settings
            .get_mut("hooks")
            .and_then(|v| v.as_object_mut())
            .ok_or_else(|| anyhow!("no hooks section in settings.json"))?;
        let rule = take_rule(hooks, event, matcher)
            .ok_or_else(|| anyhow!("no enabled hook matched {event}/{matcher}"))?;
        put_rule(&mut disabled, event, rule)?;
        save_disabled_hooks(&disabled)?;
        save_settings(&settings)
    }
}

/// Create a new enabled `command` hook rule. Refuses to duplicate a matcher
/// that already exists under `event` in either store.
pub fn create_hook(event: &str, matcher: &str, commands: &[String]) -> AppResult<()> {
    if commands.iter().all(|c| c.trim().is_empty()) {
        return Err(anyhow!("hook needs at least one non-empty command").into());
    }
    let mut settings = load_settings()?;
    let disabled = load_disabled_hooks()?;
    let hooks = settings_hooks_mut(&mut settings)?;
    if find_rule(hooks, event, matcher).is_some() || find_rule(&disabled, event, matcher).is_some() {
        return Err(anyhow!("a hook with matcher '{matcher}' already exists under {event}").into());
    }
    let commands_json: Vec<Value> = commands
        .iter()
        .filter(|c| !c.trim().is_empty())
        .map(|c| json!({ "type": "command", "command": c }))
        .collect();
    put_rule(hooks, event, json!({ "matcher": matcher, "hooks": commands_json }))?;
    save_settings(&settings)
}

/// Remove `event/matcher` from both stores. Returns (removed_from_settings,
/// removed_from_sidecar). A non-object `hooks` value in settings is treated
/// as "nothing there".
fn remove_rule_everywhere(
    settings: &mut Map<String, Value>,
    disabled: &mut Map<String, Value>,
    event: &str,
    matcher: &str,
) -> (bool, bool) {
    let removed_enabled = match settings.get_mut("hooks") {
        Some(Value::Object(hooks)) => take_rule(hooks, event, matcher).is_some(),
        _ => false,
    };
    let removed_disabled = take_rule(disabled, event, matcher).is_some();
    (removed_enabled, removed_disabled)
}

/// Delete a rule from both stores (a stale sidecar copy can shadow a settings.json rule).
pub fn delete_hook(event: &str, matcher: &str) -> AppResult<()> {
    let mut settings = load_settings()?;
    let mut disabled = load_disabled_hooks()?;
    let (removed_enabled, removed_disabled) =
        remove_rule_everywhere(&mut settings, &mut disabled, event, matcher);
    if !removed_enabled && !removed_disabled {
        return Err(anyhow!("no hook rule matched {event}/{matcher}").into());
    }
    // Settings first: even if the sidecar write then fails, the hook has stopped firing, which is what the user asked for. A leftover sidecar copy just shows up as disabled and can be deleted again.
    if removed_enabled {
        save_settings(&settings)?;
    }
    if removed_disabled {
        save_disabled_hooks(&disabled)?;
    }
    Ok(())
}

/// Replace the whole JSON body of a rule, in whichever store holds it.
pub fn update_hook_rule(event: &str, matcher: &str, new_json: &str) -> AppResult<()> {
    let new_rule: Value = serde_json::from_str(new_json)
        .with_context(|| "invalid JSON — check for syntax errors")?;
    if !new_rule.is_object() {
        return Err(anyhow!("hook rule must be a JSON object").into());
    }
    let mut settings = load_settings()?;
    let mut disabled = load_disabled_hooks()?;
    let in_settings = match settings.get_mut("hooks") {
        Some(Value::Object(hooks)) => replace_rule(hooks, event, matcher, new_rule.clone()),
        _ => false,
    };
    if in_settings {
        // The enabled copy is canonical; a stale sidecar copy would
        // resurface as "disabled" after the enabled one is deleted.
        let stale = take_rule(&mut disabled, event, matcher).is_some();
        save_settings(&settings)?;
        if stale {
            save_disabled_hooks(&disabled)?;
        }
        return Ok(());
    }
    if replace_rule(&mut disabled, event, matcher, new_rule) {
        return save_disabled_hooks(&disabled);
    }
    Err(anyhow!("no hook rule matched {event}/{matcher}").into())
}

// ---------------------------------------------------------------------------
// Plugins (shared by Agents and Skills)
// ---------------------------------------------------------------------------

fn load_enabled_plugins() -> BTreeMap<String, bool> {
    let Ok(settings) = load_settings() else {
        return BTreeMap::new();
    };
    let Some(Value::Object(ep)) = settings.get("enabledPlugins") else {
        return BTreeMap::new();
    };
    ep.iter()
        .filter_map(|(k, v)| v.as_bool().map(|b| (k.clone(), b)))
        .collect()
}

#[derive(Debug, Clone)]
pub(crate) struct InstalledPlugin {
    /// `<plugin>@<marketplace>` — matches `enabledPlugins` keys.
    pub key: String,
    /// The part before `@`.
    pub name: String,
    pub install_path: PathBuf,
}

/// Parse the `plugins` map of installed_plugins.json. Accepts the v2 shape
/// (an array of install records per key) and the older single-object shape.
pub(crate) fn parse_installed_plugins(root: &Value) -> Vec<InstalledPlugin> {
    let mut out = Vec::new();
    let Some(Value::Object(plugins)) = root.get("plugins") else { return out };
    for (key, entry) in plugins {
        // v2 keeps one record per install scope; a project-scope install can
        // sit next to a user-scope one. This app manages user-level state,
        // so prefer the user record, then any record that has a path.
        let records: Vec<&Map<String, Value>> = match entry {
            Value::Array(arr) => arr.iter().filter_map(|v| v.as_object()).collect(),
            Value::Object(m) => vec![m],
            _ => continue,
        };
        let install_path_of = |r: &Map<String, Value>| {
            r.get("installPath").and_then(|v| v.as_str()).map(str::to_string)
        };
        let is_user = |r: &Map<String, Value>| {
            r.get("scope").and_then(|v| v.as_str()) == Some("user")
        };
        let Some(path) = records
            .iter()
            .copied()
            .filter(|r| is_user(r))
            .find_map(install_path_of)
            .or_else(|| records.iter().copied().find_map(install_path_of))
        else {
            continue;
        };
        let name = key.split('@').next().unwrap_or(key).to_string();
        out.push(InstalledPlugin {
            key: key.clone(),
            name,
            install_path: PathBuf::from(path),
        });
    }
    out.sort_by(|a, b| a.key.cmp(&b.key));
    out
}

fn load_installed_plugins() -> Vec<InstalledPlugin> {
    let path = paths::installed_plugins_file();
    let Ok(raw) = fs::read_to_string(&path) else { return Vec::new() };
    let Ok(root) = serde_json::from_str::<Value>(&raw) else { return Vec::new() };
    parse_installed_plugins(&root)
        .into_iter()
        .filter(|p| p.install_path.is_dir())
        .collect()
}

/// Flip a single plugin's entry in enabledPlugins. plugin_key is <plugin>@<marketplace>.
pub fn toggle_plugin(plugin_key: &str) -> AppResult<()> {
    let mut settings = load_settings()?;
    let entry = settings
        .entry("enabledPlugins".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let ep = match entry {
        Value::Object(m) => m,
        other => {
            *other = Value::Object(Map::new());
            match other {
                Value::Object(m) => m,
                _ => unreachable!(),
            }
        }
    };
    let current = ep
        .get(plugin_key)
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    ep.insert(plugin_key.to_string(), Value::Bool(!current));
    save_settings(&settings)
}

// ---------------------------------------------------------------------------
// Agents
// ---------------------------------------------------------------------------

pub fn list_agents() -> AppResult<Vec<AgentEntry>> {
    let enabled = load_enabled_plugins();
    let mut out: Vec<AgentEntry> = Vec::new();

    // 1. Personal agents — ~/.claude/agents/*.md
    let personal_dir = paths::personal_agents_dir();
    if let Ok(entries) = fs::read_dir(&personal_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }
            let Ok(content) = fs::read_to_string(&path) else { continue };
            let meta = parse_frontmatter(&content);
            let tools: Vec<String> = meta
                .get("tools")
                .map(|s| {
                    s.split(',')
                        .map(|t| t.trim().to_string())
                        .filter(|t| !t.is_empty())
                        .collect()
                })
                .unwrap_or_default();
            out.push(AgentEntry {
                name: meta
                    .get("name")
                    .cloned()
                    .unwrap_or_else(|| file_stem(&path)),
                description: meta.get("description").cloned().unwrap_or_default(),
                tools,
                model: meta.get("model").cloned().unwrap_or_default(),
                color: meta
                    .get("color")
                    .cloned()
                    .unwrap_or_else(|| "blue".to_string()),
                plugin_name: "Personal".to_string(),
                plugin_key: String::new(),
                file_path: path.to_string_lossy().into_owned(),
                source: AgentSource::Personal,
                is_plugin_enabled: true,
            });
        }
    }

    // 2. Plugin agents — <installPath>/agents/*.md for each installed plugin.
    for plugin in load_installed_plugins() {
        let agents_subdir = plugin.install_path.join("agents");
        let Ok(agent_entries) = fs::read_dir(&agents_subdir) else { continue };
        let is_enabled = enabled.get(&plugin.key).copied().unwrap_or(false);
        for agent_entry in agent_entries.flatten() {
            let path = agent_entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }
            let Ok(content) = fs::read_to_string(&path) else { continue };
            let meta = parse_frontmatter(&content);
            let tools: Vec<String> = meta
                .get("tools")
                .map(|s| {
                    s.split(',')
                        .map(|t| t.trim().to_string())
                        .filter(|t| !t.is_empty())
                        .collect()
                })
                .unwrap_or_default();
            out.push(AgentEntry {
                name: meta
                    .get("name")
                    .cloned()
                    .unwrap_or_else(|| file_stem(&path)),
                description: meta.get("description").cloned().unwrap_or_default(),
                tools,
                model: meta.get("model").cloned().unwrap_or_default(),
                color: meta.get("color").cloned().unwrap_or_default(),
                plugin_name: plugin.name.clone(),
                plugin_key: plugin.key.clone(),
                file_path: path.to_string_lossy().into_owned(),
                source: AgentSource::Plugin,
                is_plugin_enabled: is_enabled,
            });
        }
    }

    // Personal first, then by plugin name, then by agent name.
    out.sort_by(|a, b| {
        match (a.source, b.source) {
            (AgentSource::Personal, AgentSource::Plugin) => std::cmp::Ordering::Less,
            (AgentSource::Plugin, AgentSource::Personal) => std::cmp::Ordering::Greater,
            _ => a
                .plugin_name
                .cmp(&b.plugin_name)
                .then(a.name.cmp(&b.name)),
        }
    });
    Ok(out)
}

/// Create a new personal agent file with the stock template. The name is
/// slugified (lowercase, non-alphanumeric stripped except `-`) before being
/// used as a filename.
pub fn create_agent(name: &str) -> AppResult<String> {
    let safe = slugify(name);
    if safe.is_empty() {
        return Err(anyhow!("invalid agent name").into());
    }
    let dir = paths::personal_agents_dir();
    fs::create_dir_all(&dir)?;
    let file_path = dir.join(format!("{safe}.md"));
    if file_path.exists() {
        return Err(anyhow!("agent \"{safe}\" already exists").into());
    }
    let template = format!(
        "---\nname: {safe}\ndescription: A custom agent\ntools: Read, Edit, Write, Bash, Glob, Grep\nmodel: sonnet\ncolor: blue\n---\n\nYou are a specialized agent. Describe your role and capabilities here.\n\n## Instructions\n\n- What should this agent do?\n- What tools should it use and when?\n- What rules should it follow?\n"
    );
    fs::write(&file_path, template)?;
    Ok(file_path.to_string_lossy().into_owned())
}

/// Delete a personal agent. Rejects attempts to touch plugin agents — those
/// are managed by the plugin itself, and the UI shouldn't be calling this
/// for them anyway.
pub fn delete_agent(file_path: &str) -> AppResult<()> {
    let path = Path::new(file_path);
    // Sanity check: must be inside ~/.claude/agents/
    let personal = paths::personal_agents_dir();
    let canonical = fs::canonicalize(path)
        .with_context(|| format!("resolve {}", path.display()))?;
    let canonical_personal = fs::canonicalize(&personal)
        .unwrap_or(personal.clone());
    if !canonical.starts_with(&canonical_personal) {
        return Err(anyhow!("can only delete personal agents").into());
    }
    fs::remove_file(&canonical)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Skills
// ---------------------------------------------------------------------------

pub fn list_skills() -> AppResult<Vec<SkillEntry>> {
    let enabled = load_enabled_plugins();
    let mut out: Vec<SkillEntry> = Vec::new();

    let commands_dir = paths::commands_dir();
    let skills_dir = paths::skills_dir();

    scan_skill_dir(&commands_dir, SkillSource::Command, true, "", &mut out);
    scan_skill_dir(
        &commands_dir.join(".disabled"),
        SkillSource::Command,
        false,
        "",
        &mut out,
    );

    scan_skill_dir(&skills_dir, SkillSource::Skill, true, "", &mut out);
    scan_skill_dir(
        &skills_dir.join(".disabled"),
        SkillSource::Skill,
        false,
        "",
        &mut out,
    );

    // Plugin skills (read-only, toggled via plugin enable/disable).
    for plugin in load_installed_plugins() {
        let is_enabled = enabled.get(&plugin.key).copied().unwrap_or(false);
        scan_skill_dir(
            &plugin.install_path.join("skills"),
            SkillSource::Plugin,
            is_enabled,
            &plugin.key,
            &mut out,
        );
    }

    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

/// Walk a directory looking for either `<name>.md` files or `<name>/SKILL.md`
/// subdirectories — both shapes are valid Claude Code skills. Hidden
/// `.disabled` subdirectory is skipped (it's scanned separately via a direct
/// call with `is_enabled=false`).
fn scan_skill_dir(
    dir: &Path,
    source: SkillSource,
    is_enabled: bool,
    plugin_key: &str,
    out: &mut Vec<SkillEntry>,
) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name();
        if file_name == ".disabled" {
            continue;
        }
        let skill_file = if path.extension().and_then(|s| s.to_str()) == Some("md") {
            path.clone()
        } else if path.is_dir() {
            let candidate = path.join("SKILL.md");
            if candidate.exists() {
                candidate
            } else {
                continue;
            }
        } else {
            continue;
        };
        let Ok(content) = fs::read_to_string(&skill_file) else { continue };
        let meta = parse_frontmatter(&content);
        let display_name = meta
            .get("name")
            .cloned()
            .unwrap_or_else(|| file_stem(&path));
        out.push(SkillEntry {
            name: display_name,
            description: meta.get("description").cloned().unwrap_or_default(),
            source,
            plugin_key: plugin_key.to_string(),
            file_path: skill_file.to_string_lossy().into_owned(),
            is_enabled,
        });
    }
}

/// Toggle a skill on or off by moving it between its home directory and
/// the `.disabled/` sibling. Plugin skills are not movable — their enable
/// state is tied to the plugin toggle, so this errors out for them.
///
/// For single `.md` files we move the file itself; for skills packaged as
/// `<name>/SKILL.md` we move the parent directory (so any sibling files
/// like scripts/ or references/ come along).
pub fn toggle_skill(
    file_path: &str,
    source: SkillSource,
    currently_enabled: bool,
) -> AppResult<()> {
    if matches!(source, SkillSource::Plugin) {
        return Err(anyhow!("plugin skills are toggled via the plugin").into());
    }
    let file = PathBuf::from(file_path);
    let parent = file
        .parent()
        .ok_or_else(|| anyhow!("invalid file path"))?
        .to_path_buf();

    // Move whole directory if this is a SKILL.md inside its own folder;
    // otherwise move just the .md file.
    let is_skill_md = file
        .file_name()
        .and_then(|s| s.to_str())
        .map(|n| n == "SKILL.md")
        .unwrap_or(false);
    let item_to_move = if is_skill_md { parent.clone() } else { file.clone() };
    let item_name = item_to_move
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("couldn't derive item name"))?
        .to_string();

    // Pick the active / disabled pair based on source.
    let (active_root, disabled_root) = match source {
        SkillSource::Command => {
            let root = paths::commands_dir();
            (root.clone(), root.join(".disabled"))
        }
        SkillSource::Skill => {
            let root = paths::skills_dir();
            (root.clone(), root.join(".disabled"))
        }
        SkillSource::Plugin => unreachable!(),
    };

    let (src, dest_root) = if currently_enabled {
        (item_to_move.clone(), disabled_root)
    } else {
        (item_to_move.clone(), active_root)
    };
    fs::create_dir_all(&dest_root)?;
    let dest = dest_root.join(&item_name);
    if dest.exists() {
        return Err(anyhow!(
            "destination {} already exists",
            dest.display()
        )
        .into());
    }
    fs::rename(&src, &dest)?;
    Ok(())
}

/// Where a newly created skill or command file goes. Claude Code loads
/// skills only from `<root>/<name>/SKILL.md`; commands are flat `.md` files.
pub(crate) fn skill_target_path(root: &Path, safe: &str, source: SkillSource) -> PathBuf {
    match source {
        SkillSource::Skill => root.join(safe).join("SKILL.md"),
        SkillSource::Command => root.join(format!("{safe}.md")),
        SkillSource::Plugin => unreachable!("plugin skills are not user-creatable"),
    }
}

/// Starter content for a new command or skill. Commands derive their name
/// from the filename, so only `description`/`argument-hint` go in the
/// frontmatter; skills need `name` + `description`.
pub(crate) fn skill_template(safe: &str, source: SkillSource) -> String {
    match source {
        SkillSource::Command => format!(
            "---\ndescription: A custom slash command\nargument-hint: [args]\n---\n\nYou are executing the /{safe} command.\n\n## Instructions\n\nDescribe what this command should do when invoked.\n"
        ),
        SkillSource::Skill => format!(
            "---\nname: {safe}\ndescription: A custom skill. Describe when Claude should use it.\n---\n\n## Instructions\n\nDescribe what this skill does and the steps Claude should follow.\n"
        ),
        SkillSource::Plugin => unreachable!("plugin skills are not user-creatable"),
    }
}

fn kind_label(source: SkillSource) -> &'static str {
    match source {
        SkillSource::Command => "command",
        SkillSource::Skill => "skill",
        SkillSource::Plugin => "plugin skill",
    }
}

/// Create a new personal slash command or skill. Commands are created as
/// `<name>.md`; skills are created as `<name>/SKILL.md`, the only layout
/// Claude Code loads personal skills from. Source must be Command or Skill —
/// plugin skills aren't user-creatable.
pub fn create_skill(name: &str, source: SkillSource) -> AppResult<String> {
    if matches!(source, SkillSource::Plugin) {
        return Err(anyhow!("can't create plugin skills").into());
    }
    let safe = slugify(name);
    if safe.is_empty() {
        return Err(anyhow!("invalid skill name").into());
    }
    let dir = match source {
        SkillSource::Command => paths::commands_dir(),
        SkillSource::Skill => paths::skills_dir(),
        SkillSource::Plugin => unreachable!(),
    };
    let file_path = skill_target_path(&dir, &safe, source);
    let legacy_flat = dir.join(format!("{safe}.md"));
    if file_path.exists() {
        return Err(anyhow!("a {} named \"{safe}\" already exists", kind_label(source)).into());
    }
    if source == SkillSource::Skill && legacy_flat.exists() {
        return Err(anyhow!(
            "a skill file named \"{safe}.md\" already exists (legacy layout); delete or rename it first"
        )
        .into());
    }
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let template = skill_template(&safe, source);
    fs::write(&file_path, template)?;
    Ok(file_path.to_string_lossy().into_owned())
}

/// Delete a personal slash command or skill. Rejects plugin skills, which
/// are managed by the plugin itself. For `<name>/SKILL.md`-shaped skills we
/// remove the whole parent directory so sibling assets (scripts/, etc.)
/// don't get orphaned; for single `<name>.md` files we just remove the file.
pub fn delete_skill(file_path: &str) -> AppResult<()> {
    let path = Path::new(file_path);
    let canonical = fs::canonicalize(path)
        .with_context(|| format!("resolve {}", path.display()))?;

    // The path must live under ~/.claude/commands or ~/.claude/skills
    // (including their .disabled/ siblings). Plugin skills live elsewhere
    // and are read-only from the user's perspective.
    //
    // Both roots must canonicalize successfully. Falling back to a raw
    // (non-canonical) path here would let a prefix-match slip through when
    // `~/.claude` itself is a symlink, so we require the roots to exist
    // and treat a resolve failure as an error. Matches delete_agent's
    // stricter posture.
    let commands = paths::commands_dir();
    let skills = paths::skills_dir();
    let canonical_commands = fs::canonicalize(&commands)
        .with_context(|| format!("resolve {}", commands.display()))?;
    let canonical_skills = fs::canonicalize(&skills)
        .with_context(|| format!("resolve {}", skills.display()))?;
    let allowed = canonical.starts_with(&canonical_commands)
        || canonical.starts_with(&canonical_skills);
    if !allowed {
        return Err(anyhow!("can only delete personal skills or commands").into());
    }

    // If this is a SKILL.md, blow away the whole folder so scripts/,
    // references/, etc. come with it. Otherwise just the .md file.
    let is_skill_md = canonical
        .file_name()
        .and_then(|s| s.to_str())
        .map(|n| n == "SKILL.md")
        .unwrap_or(false);
    if is_skill_md {
        let parent = canonical
            .parent()
            .ok_or_else(|| anyhow!("invalid skill path"))?;
        // Safety rail: never let a bogus path (like a bare `SKILL.md`
        // dropped straight into the commands/skills root or their
        // `.disabled/` siblings) talk us into blowing away the whole
        // directory and every skill inside it.
        let disabled_commands = canonical_commands.join(".disabled");
        let disabled_skills = canonical_skills.join(".disabled");
        if parent == canonical_commands
            || parent == canonical_skills
            || parent == disabled_commands
            || parent == disabled_skills
        {
            return Err(anyhow!("refusing to delete root skills directory").into());
        }
        fs::remove_dir_all(parent)?;
    } else {
        fs::remove_file(&canonical)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Raw file read / write (for agent & skill editor pane)
// ---------------------------------------------------------------------------

/// Read an agent or skill file as plain text, but only if the path lives
/// inside one of the known Claude Code directories. Same sandbox trick as
/// `read_backup_content` — we canonicalize both sides and check prefix.
pub fn read_claude_file(file_path: &str) -> AppResult<String> {
    let p = Path::new(file_path);
    let canonical = fs::canonicalize(p)
        .with_context(|| format!("resolve {}", p.display()))?;

    let allowed = allowed_roots();
    let ok = allowed.iter().any(|root| {
        fs::canonicalize(root)
            .map(|r| canonical.starts_with(&r))
            .unwrap_or(false)
    });
    if !ok {
        return Err(anyhow!("path not allowed").into());
    }
    let text = fs::read_to_string(&canonical)
        .with_context(|| format!("read {}", canonical.display()))?;
    Ok(text)
}

/// Write raw text to an agent or skill file, restricted to personal
/// directories (agents / commands / skills) — never into plugin directories,
/// which are read-only from the user's perspective.
pub fn write_claude_file(file_path: &str, content: &str) -> AppResult<()> {
    let p = Path::new(file_path);
    let canonical = fs::canonicalize(p)
        .with_context(|| format!("resolve {}", p.display()))?;

    // Personal dirs only — no plugin writes.
    let allowed = [
        paths::personal_agents_dir(),
        paths::commands_dir(),
        paths::skills_dir(),
    ];
    let ok = allowed.iter().any(|root| {
        fs::canonicalize(root)
            .map(|r| canonical.starts_with(&r))
            .unwrap_or(false)
    });
    if !ok {
        return Err(anyhow!("can only write personal files").into());
    }
    fs::write(&canonical, content)?;
    Ok(())
}

fn allowed_roots() -> Vec<PathBuf> {
    vec![
        paths::personal_agents_dir(),
        paths::commands_dir(),
        paths::skills_dir(),
        paths::plugins_root(),
    ]
}

// ---------------------------------------------------------------------------
// Legacy settings.json `mcpServers` migration
// ---------------------------------------------------------------------------
//
// Configonaut ≤ 0.2.3 wrote MCP servers to ~/.claude/settings.json. Claude
// Code never read them from there; they belong in ~/.claude.json.

/// Names of servers still sitting under settings.json → mcpServers.
pub fn legacy_settings_mcp_names() -> AppResult<Vec<String>> {
    let settings = load_settings()?;
    Ok(match settings.get("mcpServers") {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => Vec::new(),
    })
}

/// Split legacy entries into (entries to add to ~/.claude.json, names skipped
/// because a server with that name already exists).
pub(crate) fn plan_legacy_migration(
    settings: &Map<String, Value>,
    existing: &HashSet<String>,
) -> (Vec<(String, Value)>, Vec<String>) {
    let mut to_add = Vec::new();
    let mut skipped = Vec::new();
    if let Some(Value::Object(legacy)) = settings.get("mcpServers") {
        for (name, config) in legacy {
            if existing.contains(name) {
                skipped.push(name.clone());
            } else {
                to_add.push((name.clone(), config.clone()));
            }
        }
    }
    (to_add, skipped)
}

/// Move legacy entries into ~/.claude.json (CLI mode), archive the original
/// block under Configonaut's storage dir, then drop the key from settings.json.
/// Names that already exist as active or stored CLI servers are skipped, not
/// overwritten — their legacy configs survive in the archive file.
pub fn migrate_legacy_settings_mcp() -> AppResult<LegacyMigrationResult> {
    use crate::models::AppMode;
    let settings = load_settings()?;
    let Some(block) = settings.get("mcpServers").cloned() else {
        return Err(anyhow!("settings.json has no mcpServers block").into());
    };

    let listing = crate::config::list_servers(AppMode::Cli)?;
    let existing: HashSet<String> = listing
        .active_servers
        .iter()
        .chain(listing.stored_servers.iter())
        .map(|s| s.name.clone())
        .collect();
    let (to_add, skipped) = plan_legacy_migration(&settings, &existing);
    let moved: Vec<String> = to_add.iter().map(|(n, _)| n.clone()).collect();

    // Archive first so nothing is lost if a later step fails.
    let dir = paths::storage_dir();
    fs::create_dir_all(&dir)?;
    let stamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
    let archive = dir.join(format!("legacy_settings_mcpServers_{stamp}.json"));
    crate::config::write_atomic(&archive, serde_json::to_string_pretty(&block)?.as_bytes())?;

    if !to_add.is_empty() {
        // add_to_active backs up ~/.claude.json and normalizes url-only
        // entries (adds "type") on the way in.
        crate::config::add_to_active(AppMode::Cli, to_add)?;
    }

    // Re-read right before the destructive write to shrink the window in
    // which an external edit to settings.json could be clobbered.
    let mut settings = load_settings()?;
    settings.shift_remove("mcpServers");
    save_settings(&settings)?;

    Ok(LegacyMigrationResult {
        moved,
        skipped,
        archive_path: archive.to_string_lossy().into_owned(),
    })
}

// ---------------------------------------------------------------------------
// Slug / path helpers
// ---------------------------------------------------------------------------

fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c == ' ' { '-' } else { c })
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unnamed")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn hooks_obj(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    #[test]
    fn take_rule_removes_by_matcher_and_prunes_empty_event() {
        let mut hooks = hooks_obj(json!({
            "PreToolUse": [
                { "matcher": "Bash", "hooks": [{ "type": "command", "command": "a" }] }
            ]
        }));
        let taken = take_rule(&mut hooks, "PreToolUse", "Bash").unwrap();
        assert_eq!(taken["hooks"][0]["command"], "a");
        assert!(hooks.get("PreToolUse").is_none());
    }

    #[test]
    fn take_rule_treats_missing_matcher_as_star() {
        let mut hooks = hooks_obj(json!({
            "Stop": [ { "hooks": [{ "type": "command", "command": "a" }] } ]
        }));
        assert!(take_rule(&mut hooks, "Stop", "*").is_some());
    }

    #[test]
    fn put_rule_creates_event_array() {
        let mut hooks = Map::new();
        put_rule(&mut hooks, "Stop", json!({ "matcher": "*", "hooks": [] })).unwrap();
        assert_eq!(hooks["Stop"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn summarize_handler_covers_all_types() {
        assert_eq!(
            summarize_handler(&json!({ "type": "command", "command": "npm test" })),
            Some("npm test".into())
        );
        assert_eq!(
            summarize_handler(&json!({ "command": "no-type" })),
            Some("no-type".into())
        );
        assert_eq!(
            summarize_handler(&json!({ "type": "prompt", "prompt": "Is this safe?" })),
            Some("prompt: Is this safe?".into())
        );
        assert_eq!(
            summarize_handler(&json!({ "type": "http", "url": "https://x/hook" })),
            Some("http: https://x/hook".into())
        );
        assert_eq!(
            summarize_handler(&json!({ "type": "mcp_tool", "server": "s", "tool": "t" })),
            Some("mcp_tool: s/t".into())
        );
    }

    #[test]
    fn rule_to_hook_rule_keeps_non_command_handlers() {
        let rule = json!({
            "matcher": "Write",
            "hooks": [
                { "type": "prompt", "prompt": "check it" },
                { "type": "command", "command": "lint" }
            ]
        });
        let hr = rule_to_hook_rule("PreToolUse", &rule, true).unwrap();
        assert_eq!(hr.id, "PreToolUse::Write");
        assert_eq!(hr.commands, vec!["prompt: check it", "lint"]);
        assert_eq!(hr.handler_types, vec!["prompt", "command"]);
        assert!(hr.is_enabled);
    }

    #[test]
    fn migrate_legacy_disabled_moves_flagged_rules_to_sidecar() {
        let mut hooks = hooks_obj(json!({
            "PreToolUse": [
                { "matcher": "Bash", "disabled": true, "hooks": [{ "type": "command", "command": "a" }] },
                { "matcher": "Write", "hooks": [{ "type": "command", "command": "b" }] }
            ],
            "Stop": [
                { "matcher": "*", "disabled": true, "hooks": [{ "type": "command", "command": "c" }] }
            ]
        }));
        let mut disabled = Map::new();
        assert!(migrate_legacy_disabled(&mut hooks, &mut disabled).unwrap());
        assert_eq!(hooks["PreToolUse"].as_array().unwrap().len(), 1);
        assert!(hooks.get("Stop").is_none());
        let moved = &disabled["PreToolUse"][0];
        assert!(moved.get("disabled").is_none());
        assert_eq!(moved["matcher"], "Bash");
        assert_eq!(disabled["Stop"][0]["hooks"][0]["command"], "c");
        // Second pass is a no-op.
        assert!(!migrate_legacy_disabled(&mut hooks, &mut disabled).unwrap());
    }

    #[test]
    fn rule_to_hook_rule_keeps_commands_and_types_aligned() {
        let rule = json!({
            "matcher": "*",
            "hooks": [
                { "type": "command" },
                { "type": "http", "url": "https://x/hook" }
            ]
        });
        let hr = rule_to_hook_rule("Stop", &rule, true).unwrap();
        assert_eq!(hr.commands, vec!["http: https://x/hook"]);
        assert_eq!(hr.handler_types, vec!["http"]);
    }

    #[test]
    fn put_rule_replaces_existing_matcher() {
        let mut hooks = hooks_obj(json!({ "Stop": [ { "matcher": "*", "hooks": [{ "type": "command", "command": "old" }] } ] }));
        put_rule(&mut hooks, "Stop", json!({ "matcher": "*", "hooks": [{ "type": "command", "command": "new" }] })).unwrap();
        let arr = hooks["Stop"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["hooks"][0]["command"], "new");
    }

    #[test]
    fn put_rule_errors_on_non_array_event() {
        let mut hooks = hooks_obj(json!({ "Stop": "oops" }));
        assert!(put_rule(&mut hooks, "Stop", json!({ "matcher": "*", "hooks": [] })).is_err());
        assert_eq!(hooks["Stop"], "oops");
    }

    #[test]
    fn settings_hooks_mut_errors_on_non_object() {
        let mut settings = hooks_obj(json!({ "hooks": [1, 2] }));
        assert!(settings_hooks_mut(&mut settings).is_err());
        assert_eq!(settings["hooks"], json!([1, 2]));
    }

    #[test]
    fn replace_rule_keeps_position() {
        let mut hooks = hooks_obj(json!({ "PreToolUse": [
            { "matcher": "A", "hooks": [] },
            { "matcher": "B", "hooks": [] },
            { "matcher": "C", "hooks": [] }
        ] }));
        assert!(replace_rule(&mut hooks, "PreToolUse", "B", json!({ "matcher": "B", "hooks": [{ "type": "command", "command": "x" }] })));
        let arr = hooks["PreToolUse"].as_array().unwrap();
        assert_eq!(arr[1]["matcher"], "B");
        assert_eq!(arr[1]["hooks"][0]["command"], "x");
        assert!(!replace_rule(&mut hooks, "PreToolUse", "Z", json!({})));
    }

    #[test]
    fn remove_rule_everywhere_clears_both_stores() {
        let mut settings = hooks_obj(json!({ "hooks": { "Stop": [ { "matcher": "*", "hooks": [] } ] } }));
        let mut disabled = hooks_obj(json!({ "Stop": [ { "matcher": "*", "hooks": [] } ] }));
        assert_eq!(remove_rule_everywhere(&mut settings, &mut disabled, "Stop", "*"), (true, true));
        assert!(settings["hooks"].as_object().unwrap().get("Stop").is_none());
        assert!(disabled.get("Stop").is_none());
        assert_eq!(remove_rule_everywhere(&mut settings, &mut disabled, "Stop", "*"), (false, false));
    }

    #[test]
    fn remove_rule_everywhere_tolerates_non_object_hooks() {
        let mut settings = hooks_obj(json!({ "hooks": "oops" }));
        let mut disabled = hooks_obj(json!({ "Stop": [ { "matcher": "*", "hooks": [] } ] }));
        assert_eq!(remove_rule_everywhere(&mut settings, &mut disabled, "Stop", "*"), (false, true));
        assert_eq!(settings["hooks"], "oops");
    }

    #[test]
    fn migrate_legacy_disabled_replaces_existing_sidecar_copy() {
        let mut hooks = hooks_obj(json!({ "Stop": [ { "matcher": "*", "disabled": true, "hooks": [{ "type": "command", "command": "new" }] } ] }));
        let mut disabled = hooks_obj(json!({ "Stop": [ { "matcher": "*", "hooks": [{ "type": "command", "command": "old" }] } ] }));
        assert!(migrate_legacy_disabled(&mut hooks, &mut disabled).unwrap());
        let arr = disabled["Stop"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["hooks"][0]["command"], "new");
    }

    #[test]
    fn parse_installed_plugins_reads_v2_array_records() {
        let root = json!({
            "version": 2,
            "plugins": {
                "superpowers@claude-plugins-official": [
                    { "installPath": "/home/u/.claude/plugins/cache/claude-plugins-official/superpowers/5.1.0", "scope": "user" }
                ],
                "legacy@other": { "installPath": "/home/u/.claude/plugins/cache/other/legacy/1.0.0" },
                "broken@x": [ { "scope": "user" } ]
            }
        });
        let plugins = parse_installed_plugins(&root);
        assert_eq!(plugins.len(), 2);
        assert_eq!(plugins[0].key, "legacy@other");
        assert_eq!(plugins[0].name, "legacy");
        assert_eq!(plugins[1].key, "superpowers@claude-plugins-official");
        assert_eq!(plugins[1].name, "superpowers");
        assert!(plugins[1].install_path.ends_with("superpowers/5.1.0"));
    }

    #[test]
    fn parse_installed_plugins_tolerates_missing_or_empty_map() {
        assert!(parse_installed_plugins(&json!({ "version": 2 })).is_empty());
        assert!(parse_installed_plugins(&json!({ "plugins": [] })).is_empty());
        assert!(parse_installed_plugins(&json!(null)).is_empty());
        assert!(parse_installed_plugins(&json!({ "plugins": { "a@m": [] } })).is_empty());
    }

    #[test]
    fn parse_installed_plugins_prefers_user_scope_record() {
        let root = json!({ "plugins": { "p@m": [
            { "scope": "project", "installPath": "/proj/p" },
            { "scope": "user", "installPath": "/user/p" }
        ] } });
        let plugins = parse_installed_plugins(&root);
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].install_path, PathBuf::from("/user/p"));
    }

    #[test]
    fn parse_installed_plugins_skips_unusable_first_record() {
        let root = json!({ "plugins": { "p@m": [
            { "scope": "user" },
            { "scope": "project", "installPath": "/proj/p" }
        ] } });
        let plugins = parse_installed_plugins(&root);
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].install_path, PathBuf::from("/proj/p"));
    }

    #[test]
    fn parse_installed_plugins_keeps_same_name_from_two_marketplaces() {
        let root = json!({ "plugins": {
            "tools@alpha": [ { "installPath": "/a/tools" } ],
            "tools@beta": [ { "installPath": "/b/tools" } ]
        } });
        let plugins = parse_installed_plugins(&root);
        assert_eq!(plugins.len(), 2);
        assert_eq!(plugins[0].key, "tools@alpha");
        assert_eq!(plugins[1].key, "tools@beta");
        assert!(plugins.iter().all(|p| p.name == "tools"));
    }

    #[test]
    fn skill_target_path_uses_directory_layout_for_skills() {
        let root = Path::new("/r");
        assert_eq!(
            skill_target_path(root, "my-skill", SkillSource::Skill),
            PathBuf::from("/r/my-skill/SKILL.md")
        );
        assert_eq!(
            skill_target_path(root, "my-cmd", SkillSource::Command),
            PathBuf::from("/r/my-cmd.md")
        );
    }

    #[test]
    fn skill_template_frontmatter_round_trips() {
        let skill = parse_frontmatter(&skill_template("foo", SkillSource::Skill));
        assert_eq!(skill.get("name").map(String::as_str), Some("foo"));
        assert!(!skill.get("description").unwrap_or(&String::new()).is_empty());

        let cmd = parse_frontmatter(&skill_template("bar", SkillSource::Command));
        assert!(!cmd.contains_key("name"));
        assert!(cmd.contains_key("description"));
        assert_eq!(cmd.get("argument-hint").map(String::as_str), Some("[args]"));
    }

    fn names(v: &[&str]) -> HashSet<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn plan_legacy_migration_skips_names_already_existing() {
        let settings = hooks_obj(json!({ "mcpServers": {
            "figma": { "url": "http://127.0.0.1:3845/mcp" },
            "fathom": { "command": "npx", "args": ["-y", "fathom-mcp"] }
        } }));
        let (to_add, skipped) = plan_legacy_migration(&settings, &names(&["figma"]));
        assert_eq!(to_add.len(), 1);
        assert_eq!(to_add[0].0, "fathom");
        assert_eq!(skipped, vec!["figma"]);
    }

    #[test]
    fn plan_legacy_migration_handles_missing_empty_and_non_object() {
        let empty = names(&[]);
        assert_eq!(plan_legacy_migration(&hooks_obj(json!({})), &empty), (vec![], vec![]));
        assert_eq!(plan_legacy_migration(&hooks_obj(json!({ "mcpServers": {} })), &empty), (vec![], vec![]));
        assert_eq!(plan_legacy_migration(&hooks_obj(json!({ "mcpServers": [1] })), &empty), (vec![], vec![]));
    }

    #[test]
    fn plan_legacy_migration_preserves_order_and_reports_all_skipped() {
        let settings = hooks_obj(json!({ "mcpServers": { "a": {}, "b": {}, "c": {} } }));
        let (to_add, skipped) = plan_legacy_migration(&settings, &names(&[]));
        assert_eq!(to_add.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(), vec!["a", "b", "c"]);
        assert!(skipped.is_empty());
        let (to_add, skipped) = plan_legacy_migration(&settings, &names(&["a", "b", "c"]));
        assert!(to_add.is_empty());
        assert_eq!(skipped, vec!["a", "b", "c"]);
    }
}
