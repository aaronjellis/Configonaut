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
//       "hooks": { "<Event>": [ { "matcher": "...", "disabled": bool?,
//                                  "hooks": [ { "command": "..." } ] } ] },
//       "enabledPlugins": { "<pluginName>@claude-plugins-official": bool },
//       ...
//     }
//   ~/.claude/agents/*.md                       (personal agents)
//   ~/.claude/commands/*.md                     (custom slash commands)
//   ~/.claude/commands/.disabled/*.md           (hidden/off)
//   ~/.claude/skills/{name,SKILL.md|<name>.md}  (custom skills)
//   ~/.claude/skills/.disabled/...              (hidden/off)
//   ~/.claude/plugins/marketplaces/claude-plugins-official/plugins/
//       <plugin>/agents/*.md                    (read-only plugin agents)
//       <plugin>/skills/...                     (read-only plugin skills)

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context};
use serde_json::{Map, Value};

use crate::models::{
    AgentEntry, AgentSource, AppResult, HookRule, SkillEntry, SkillSource,
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
    let tmp = match path.parent() {
        Some(parent) => parent.join(format!(
            ".{}.tmp",
            path.file_name().and_then(|s| s.to_str()).unwrap_or("settings")
        )),
        None => path.with_extension("tmp"),
    };
    fs::write(&tmp, json.as_bytes())?;
    fs::rename(&tmp, &path)?;
    Ok(())
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

/// Read every hook rule out of settings.json, sorted by event then matcher
/// for a stable UI order. The id is a synthetic `event::matcher` pair — not
/// globally unique if the user registers the same matcher twice under the
/// same event, but that's a misconfiguration and the UI doesn't need to
/// support it.
pub fn list_hooks() -> AppResult<Vec<HookRule>> {
    let settings = load_settings()?;
    let Some(Value::Object(hooks)) = settings.get("hooks") else {
        return Ok(vec![]);
    };

    let mut out: Vec<HookRule> = Vec::new();
    for (event, rules) in hooks {
        let Value::Array(arr) = rules else { continue };
        for rule in arr {
            let Value::Object(rule_map) = rule else { continue };
            let matcher = rule_map
                .get("matcher")
                .and_then(|v| v.as_str())
                .unwrap_or("*")
                .to_string();
            let is_disabled = rule_map
                .get("disabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let commands: Vec<String> = rule_map
                .get("hooks")
                .and_then(|v| v.as_array())
                .map(|inner| {
                    inner
                        .iter()
                        .filter_map(|h| {
                            h.as_object()
                                .and_then(|o| o.get("command"))
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        })
                        .collect()
                })
                .unwrap_or_default();
            if commands.is_empty() {
                continue;
            }
            out.push(HookRule {
                id: format!("{event}::{matcher}"),
                event: event.clone(),
                matcher,
                commands,
                is_enabled: !is_disabled,
            });
        }
    }
    out.sort_by(|a, b| a.event.cmp(&b.event).then(a.matcher.cmp(&b.matcher)));
    Ok(out)
}

/// Pretty-print the raw JSON for one specific hook rule so the editor pane
/// can show exactly what's on disk, without needing to reverse-engineer it
/// from the parsed HookRule fields.
pub fn hook_rule_json(event: &str, matcher: &str) -> AppResult<String> {
    let settings = load_settings()?;
    let Some(Value::Object(hooks)) = settings.get("hooks") else {
        return Ok("{}".to_string());
    };
    let Some(Value::Array(arr)) = hooks.get(event) else {
        return Ok("{}".to_string());
    };
    for rule in arr {
        if let Value::Object(m) = rule {
            let m_matcher = m
                .get("matcher")
                .and_then(|v| v.as_str())
                .unwrap_or("*");
            if m_matcher == matcher {
                return Ok(serde_json::to_string_pretty(rule)
                    .unwrap_or_else(|_| "{}".into()));
            }
        }
    }
    Ok("{}".to_string())
}

/// Flip a hook's `disabled` flag. We don't care what it currently is — we
/// set it to the caller's intent, so repeated clicks are idempotent.
pub fn toggle_hook(event: &str, matcher: &str, enable: bool) -> AppResult<()> {
    let mut settings = load_settings()?;
    let hooks = settings
        .get_mut("hooks")
        .and_then(|v| v.as_object_mut())
        .ok_or_else(|| anyhow!("no hooks section in settings.json"))?;
    let arr = hooks
        .get_mut(event)
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| anyhow!("hook event {event} not found"))?;

    for rule in arr.iter_mut() {
        if let Value::Object(m) = rule {
            let m_matcher = m
                .get("matcher")
                .and_then(|v| v.as_str())
                .unwrap_or("*");
            if m_matcher == matcher {
                if enable {
                    m.remove("disabled");
                } else {
                    m.insert("disabled".to_string(), Value::Bool(true));
                }
                break;
            }
        }
    }
    save_settings(&settings)
}

/// Create a brand new hook rule under `event`. We refuse to silently
/// overwrite: if a rule with the same matcher already exists under this
/// event, the caller gets an error instead of a surprise stomp.
///
/// The inner shape follows what Claude Code actually writes:
///   { "matcher": "...", "hooks": [ { "type": "command", "command": "..." } ] }
/// `type: "command"` isn't required by our reader but is the documented
/// format, so we emit it for forward-compat.
pub fn create_hook(event: &str, matcher: &str, commands: &[String]) -> AppResult<()> {
    if commands.iter().all(|c| c.trim().is_empty()) {
        return Err(anyhow!("hook needs at least one non-empty command").into());
    }

    let mut settings = load_settings()?;

    // Ensure the top-level "hooks" object exists (may be absent on fresh
    // installs; `create_hook` is the one command that's allowed to mint it).
    let hooks_entry = settings
        .entry("hooks".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let hooks_obj = hooks_entry
        .as_object_mut()
        .ok_or_else(|| anyhow!("'hooks' in settings.json is not a JSON object"))?;

    // Ensure the event array exists.
    let event_entry = hooks_obj
        .entry(event.to_string())
        .or_insert_with(|| Value::Array(vec![]));
    let arr = event_entry
        .as_array_mut()
        .ok_or_else(|| anyhow!("'hooks.{event}' is not a JSON array"))?;

    // Duplicate check. Same convention as list_hooks: absent matcher defaults to "*".
    for rule in arr.iter() {
        if let Value::Object(m) = rule {
            let existing = m.get("matcher").and_then(|v| v.as_str()).unwrap_or("*");
            if existing == matcher {
                return Err(anyhow!(
                    "a hook with matcher '{matcher}' already exists under {event}"
                )
                .into());
            }
        }
    }

    // Build the new rule. Skip empty command strings; the caller may have
    // sent placeholders from a multi-row form.
    let commands_json: Vec<Value> = commands
        .iter()
        .filter(|c| !c.trim().is_empty())
        .map(|c| {
            let mut m = Map::new();
            m.insert("type".to_string(), Value::String("command".to_string()));
            m.insert("command".to_string(), Value::String(c.clone()));
            Value::Object(m)
        })
        .collect();

    let mut rule = Map::new();
    rule.insert("matcher".to_string(), Value::String(matcher.to_string()));
    rule.insert("hooks".to_string(), Value::Array(commands_json));
    arr.push(Value::Object(rule));

    save_settings(&settings)
}

/// Delete a single hook rule. Also prunes the event key if this was the
/// last rule under it, so the file stays tidy and list_hooks doesn't see
/// phantom events with zero rules.
pub fn delete_hook(event: &str, matcher: &str) -> AppResult<()> {
    let mut settings = load_settings()?;
    let hooks_obj = settings
        .get_mut("hooks")
        .and_then(|v| v.as_object_mut())
        .ok_or_else(|| anyhow!("no hooks section in settings.json"))?;
    let arr = hooks_obj
        .get_mut(event)
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| anyhow!("hook event {event} not found"))?;

    let before = arr.len();
    arr.retain(|rule| {
        let Value::Object(m) = rule else { return true };
        let existing = m.get("matcher").and_then(|v| v.as_str()).unwrap_or("*");
        existing != matcher
    });
    if arr.len() == before {
        return Err(anyhow!("no hook rule matched {event}/{matcher}").into());
    }

    if arr.is_empty() {
        hooks_obj.remove(event);
    }
    save_settings(&settings)
}

/// Replace the whole JSON body for a single hook rule. The new JSON is
/// expected to be a complete rule object (matcher, hooks, etc.) — we don't
/// merge, we substitute.
pub fn update_hook_rule(
    event: &str,
    matcher: &str,
    new_json: &str,
) -> AppResult<()> {
    let new_rule: Value = serde_json::from_str(new_json)
        .with_context(|| "invalid JSON — check for syntax errors")?;
    if !new_rule.is_object() {
        return Err(anyhow!("hook rule must be a JSON object").into());
    }
    let mut settings = load_settings()?;
    let hooks = settings
        .get_mut("hooks")
        .and_then(|v| v.as_object_mut())
        .ok_or_else(|| anyhow!("no hooks section in settings.json"))?;
    let arr = hooks
        .get_mut(event)
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| anyhow!("hook event {event} not found"))?;

    let mut replaced = false;
    for rule in arr.iter_mut() {
        if let Value::Object(m) = rule {
            let m_matcher = m
                .get("matcher")
                .and_then(|v| v.as_str())
                .unwrap_or("*");
            if m_matcher == matcher {
                *rule = new_rule.clone();
                replaced = true;
                break;
            }
        }
    }
    if !replaced {
        return Err(anyhow!("no hook rule matched {event}/{matcher}").into());
    }
    save_settings(&settings)
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

/// Flip a single plugin's entry in `enabledPlugins`. Key format is
/// `<pluginName>@claude-plugins-official`, matching what the Swift version
/// wrote. Callers pass the already-formatted key.
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
                file_path: path.to_string_lossy().into_owned(),
                source: AgentSource::Personal,
                is_plugin_enabled: true,
            });
        }
    }

    // 2. Plugin agents — ~/.claude/plugins/.../plugins/<plugin>/agents/*.md
    let plugins_dir = paths::plugins_dir();
    if let Ok(plugin_entries) = fs::read_dir(&plugins_dir) {
        for plugin_entry in plugin_entries.flatten() {
            let plugin_path = plugin_entry.path();
            if !plugin_path.is_dir() {
                continue;
            }
            let plugin_name = plugin_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            if plugin_name.is_empty() {
                continue;
            }
            let agents_subdir = plugin_path.join("agents");
            let Ok(agent_entries) = fs::read_dir(&agents_subdir) else { continue };
            let plugin_key = format!("{plugin_name}@claude-plugins-official");
            let is_enabled = enabled.get(&plugin_key).copied().unwrap_or(false);
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
                    plugin_name: plugin_name.clone(),
                    file_path: path.to_string_lossy().into_owned(),
                    source: AgentSource::Plugin,
                    is_plugin_enabled: is_enabled,
                });
            }
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

    scan_skill_dir(&commands_dir, SkillSource::Command, true, &mut out);
    scan_skill_dir(
        &commands_dir.join(".disabled"),
        SkillSource::Command,
        false,
        &mut out,
    );

    scan_skill_dir(&skills_dir, SkillSource::Skill, true, &mut out);
    scan_skill_dir(
        &skills_dir.join(".disabled"),
        SkillSource::Skill,
        false,
        &mut out,
    );

    // Plugin skills (read-only, toggled via plugin enable/disable).
    let plugins_dir = paths::plugins_dir();
    if let Ok(plugin_entries) = fs::read_dir(&plugins_dir) {
        for plugin_entry in plugin_entries.flatten() {
            let plugin_path = plugin_entry.path();
            if !plugin_path.is_dir() {
                continue;
            }
            let plugin_name = plugin_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            if plugin_name.is_empty() {
                continue;
            }
            let plugin_key = format!("{plugin_name}@claude-plugins-official");
            let is_enabled = enabled.get(&plugin_key).copied().unwrap_or(false);
            scan_skill_dir(
                &plugin_path.join("skills"),
                SkillSource::Plugin,
                is_enabled,
                &mut out,
            );
        }
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

/// Create a new personal slash command or skill file from the stock template.
/// Source must be Command or Skill — plugin skills aren't user-creatable.
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
    fs::create_dir_all(&dir)?;
    let file_path = dir.join(format!("{safe}.md"));
    if file_path.exists() {
        return Err(anyhow!("{safe}.md already exists").into());
    }
    let template = match source {
        SkillSource::Command => format!(
            "---\nname: {safe}\ndescription: A custom slash command\n---\n\nYou are executing the /{safe} command.\n\n## Instructions\n\nDescribe what this command should do when invoked.\n"
        ),
        SkillSource::Skill => format!(
            "---\nname: {safe}\ndescription: A custom skill\n---\n\nYou are a specialized skill.\n\n## Instructions\n\nDescribe what this skill does and when it should activate.\n"
        ),
        SkillSource::Plugin => unreachable!(),
    };
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
        paths::plugins_dir(),
    ]
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
