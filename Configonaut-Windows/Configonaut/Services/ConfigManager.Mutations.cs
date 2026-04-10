using System.Text.Json.Nodes;
using Configonaut.Models;

namespace Configonaut.Services;

public partial class ConfigManager
{
    // ==========================================================
    // MUTATION METHODS
    // ==========================================================

    // --- Server mutations ---

    public bool AddToActive(IEnumerable<ServerEntry> entries)
    {
        var root = JsonHelper.ReadJsonObject(ConfigPath) ?? new JsonObject();
        var mcpServers = root["mcpServers"]?.AsObject() ?? new JsonObject();

        foreach (var entry in entries)
        {
            var parsed = JsonHelper.ParseJsonObject(entry.ConfigJson);
            if (parsed is null) continue;
            mcpServers[entry.Name] = parsed;
        }

        root["mcpServers"] = mcpServers;
        if (!SaveConfig(root)) return false;
        LoadActive();
        LoadStored(); // dedup
        NeedsRestart = true;
        return true;
    }

    public bool AddToStored(IEnumerable<ServerEntry> entries)
    {
        var root = JsonHelper.ReadJsonObject(StoredPath) ?? new JsonObject();

        foreach (var entry in entries)
        {
            var parsed = JsonHelper.ParseJsonObject(entry.ConfigJson);
            if (parsed is null) continue;
            root[entry.Name] = parsed;
        }

        if (!SaveStored(root)) return false;
        LoadStored();
        return true;
    }

    public bool MoveToStored(string name)
    {
        var configRoot = JsonHelper.ReadJsonObject(ConfigPath);
        if (configRoot?["mcpServers"] is not JsonObject mcpServers || !mcpServers.ContainsKey(name))
            return false;

        var serverObj = mcpServers[name]!.DeepClone();
        mcpServers.Remove(name);

        var storedRoot = JsonHelper.ReadJsonObject(StoredPath) ?? new JsonObject();
        storedRoot[name] = serverObj;

        if (!SaveConfig(configRoot)) return false;
        if (!SaveStored(storedRoot)) return false;

        LoadActive();
        LoadStored();
        NeedsRestart = true;
        SetStatus($"Moved '{name}' to inactive");
        return true;
    }

    public bool MoveToActive(string name)
    {
        var storedRoot = JsonHelper.ReadJsonObject(StoredPath);
        if (storedRoot is null || !storedRoot.ContainsKey(name))
            return false;

        var serverObj = storedRoot[name]!.DeepClone();
        storedRoot.Remove(name);

        var configRoot = JsonHelper.ReadJsonObject(ConfigPath) ?? new JsonObject();
        var mcpServers = configRoot["mcpServers"]?.AsObject() ?? new JsonObject();
        mcpServers[name] = serverObj;
        configRoot["mcpServers"] = mcpServers;

        if (!SaveConfig(configRoot)) return false;
        if (!SaveStored(storedRoot)) return false;

        LoadActive();
        LoadStored();
        NeedsRestart = true;
        SetStatus($"Activated '{name}'");
        return true;
    }

    public bool DeleteServer(string name, ServerSource source)
    {
        if (source == ServerSource.Active)
        {
            var root = JsonHelper.ReadJsonObject(ConfigPath);
            if (root?["mcpServers"] is JsonObject mcpServers)
            {
                mcpServers.Remove(name);
                if (!SaveConfig(root)) return false;
                LoadActive();
                NeedsRestart = true;
            }
        }
        else
        {
            var root = JsonHelper.ReadJsonObject(StoredPath);
            if (root is not null)
            {
                root.Remove(name);
                if (!SaveStored(root)) return false;
                LoadStored();
            }
        }

        SetStatus($"Deleted '{name}'");
        return true;
    }

    public bool UpdateServerConfig(string name, ServerSource source, string newJson)
    {
        var parsed = JsonHelper.ParseJsonObject(newJson);
        if (parsed is null)
        {
            SetStatus("Invalid JSON", isError: true);
            return false;
        }

        if (source == ServerSource.Active)
        {
            var root = JsonHelper.ReadJsonObject(ConfigPath);
            if (root?["mcpServers"] is JsonObject mcpServers)
            {
                mcpServers[name] = parsed;
                if (!SaveConfig(root)) return false;
                LoadActive();
                NeedsRestart = true;
            }
        }
        else
        {
            var root = JsonHelper.ReadJsonObject(StoredPath) ?? new JsonObject();
            root[name] = parsed;
            if (!SaveStored(root)) return false;
            LoadStored();
        }

        SetStatus($"Updated '{name}'");
        return true;
    }

    // --- Save helpers ---

    private bool SaveConfig(JsonObject root)
    {
        try
        {
            CreateBackup();
            JsonHelper.WriteJsonObject(ConfigPath, root);
            return true;
        }
        catch (Exception ex)
        {
            SetStatus($"Save failed: {ex.Message}", isError: true);
            return false;
        }
    }

    private bool SaveStored(JsonObject root)
    {
        try
        {
            CreateBackup();
            Directory.CreateDirectory(PathResolver.StorageDir);
            JsonHelper.WriteJsonObject(StoredPath, root);
            return true;
        }
        catch (Exception ex)
        {
            SetStatus($"Save failed: {ex.Message}", isError: true);
            return false;
        }
    }

    private bool SaveSettings(JsonObject settings)
    {
        try
        {
            JsonHelper.WriteJsonObject(PathResolver.CliSettingsPath, settings);
            return true;
        }
        catch (Exception ex)
        {
            SetStatus($"Save failed: {ex.Message}", isError: true);
            return false;
        }
    }

    // --- Backup logic ---

    private void CreateBackup()
    {
        try
        {
            if (!File.Exists(ConfigPath)) return;

            var data = File.ReadAllText(ConfigPath);
            var hash = data.GetHashCode();

            if (_lastBackupHash.HasValue && hash == _lastBackupHash.Value) return;
            if (_lastBackupDate.HasValue && (DateTime.Now - _lastBackupDate.Value).TotalSeconds < 300) return;

            WriteBackup(data, hash);
        }
        catch
        {
            // Backup failure is non-fatal
        }
    }

    public void ForceBackup()
    {
        try
        {
            if (!File.Exists(ConfigPath)) return;
            var data = File.ReadAllText(ConfigPath);
            WriteBackup(data, data.GetHashCode());
        }
        catch
        {
            // Non-fatal
        }
    }

    private void WriteBackup(string data, int hash)
    {
        var dir = BackupDir;
        Directory.CreateDirectory(dir);

        var timestamp = DateTime.Now.ToString("yyyy-MM-dd_HH-mm-ss");
        var fileName = $"config_{timestamp}.json";
        var filePath = Path.Combine(dir, fileName);

        File.WriteAllText(filePath, data);

        _lastBackupDate = DateTime.Now;
        _lastBackupHash = hash;

        // Prune to 30 max
        var files = Directory.GetFiles(dir, "*.json").OrderByDescending(f => f).ToList();
        foreach (var old in files.Skip(30))
            File.Delete(old);
    }

    public bool RestoreBackup(BackupFile backup)
    {
        try
        {
            var backupPath = Path.Combine(BackupDir, backup.FileName);
            if (!File.Exists(backupPath))
            {
                SetStatus("Backup file not found", isError: true);
                return false;
            }

            var content = File.ReadAllText(backupPath);
            var parsed = JsonHelper.ParseJsonObject(content);
            if (parsed is null)
            {
                SetStatus("Backup contains invalid JSON", isError: true);
                return false;
            }

            ForceBackup(); // Back up current state before restoring
            JsonHelper.WriteJsonObject(ConfigPath, parsed);
            LoadActive();
            NeedsRestart = true;
            SetStatus("Backup restored");
            return true;
        }
        catch (Exception ex)
        {
            SetStatus($"Restore failed: {ex.Message}", isError: true);
            return false;
        }
    }

    public void DeleteBackup(BackupFile backup)
    {
        var path = Path.Combine(BackupDir, backup.FileName);
        if (File.Exists(path))
            File.Delete(path);
        LoadBackups();
    }

    /// <summary>Read a backup file's content for preview</summary>
    public string? ReadBackupContent(BackupFile backup)
    {
        var path = Path.Combine(BackupDir, backup.FileName);
        return File.Exists(path) ? File.ReadAllText(path) : null;
    }

    // --- Hook mutations ---

    public string? GetHookRuleJson(HookRule hook)
    {
        var settings = JsonHelper.ReadJsonObject(PathResolver.CliSettingsPath);
        if (settings?["hooks"] is not JsonObject hooksNode) return null;
        if (hooksNode[hook.Event] is not JsonArray ruleArray) return null;

        foreach (var ruleNode in ruleArray)
        {
            if (ruleNode is not JsonObject ruleObj) continue;
            var matcher = ruleObj["matcher"]?.GetValue<string>() ?? "*";
            if (matcher == hook.Matcher)
                return JsonHelper.PrettyPrint(ruleObj);
        }
        return null;
    }

    public bool UpdateHookRule(HookRule hook, string newJson)
    {
        var parsed = JsonHelper.ParseJsonObject(newJson);
        if (parsed is null)
        {
            SetStatus("Invalid JSON", isError: true);
            return false;
        }

        var settings = JsonHelper.ReadJsonObject(PathResolver.CliSettingsPath);
        if (settings?["hooks"] is not JsonObject hooksNode) return false;
        if (hooksNode[hook.Event] is not JsonArray ruleArray) return false;

        for (int i = 0; i < ruleArray.Count; i++)
        {
            if (ruleArray[i] is not JsonObject ruleObj) continue;
            var matcher = ruleObj["matcher"]?.GetValue<string>() ?? "*";
            if (matcher == hook.Matcher)
            {
                ruleArray[i] = parsed;
                break;
            }
        }

        if (!SaveSettings(settings!)) return false;
        LoadHooks();
        SetStatus("Hook updated");
        return true;
    }

    public bool ToggleHook(HookRule hook)
    {
        var settings = JsonHelper.ReadJsonObject(PathResolver.CliSettingsPath);
        if (settings?["hooks"] is not JsonObject hooksNode) return false;
        if (hooksNode[hook.Event] is not JsonArray ruleArray) return false;

        for (int i = 0; i < ruleArray.Count; i++)
        {
            if (ruleArray[i] is not JsonObject ruleObj) continue;
            var matcher = ruleObj["matcher"]?.GetValue<string>() ?? "*";
            if (matcher == hook.Matcher)
            {
                if (hook.IsEnabled)
                    ruleObj["disabled"] = true;
                else
                    ruleObj.Remove("disabled");
                break;
            }
        }

        if (!SaveSettings(settings!)) return false;
        LoadHooks();
        return true;
    }

    // --- Plugin toggle ---

    public bool TogglePlugin(string pluginKey)
    {
        var settings = JsonHelper.ReadJsonObject(PathResolver.CliSettingsPath) ?? new JsonObject();
        var plugins = settings["enabledPlugins"]?.AsObject() ?? new JsonObject();

        var currentlyEnabled = plugins.ContainsKey(pluginKey) &&
            plugins[pluginKey]?.GetValue<bool>() == true;
        plugins[pluginKey] = !currentlyEnabled;
        settings["enabledPlugins"] = plugins;

        if (!SaveSettings(settings)) return false;
        LoadEnabledPlugins();
        LoadAgents();
        return true;
    }

    // --- Skill toggle ---

    public bool ToggleSkill(SkillEntry skill)
    {
        if (skill.Source == SkillSource.Plugin) return false;

        try
        {
            var filePath = skill.FilePath;
            string targetPath;

            if (skill.IsEnabled)
            {
                var dir = Path.GetDirectoryName(filePath)!;
                var disabledDir = Path.Combine(dir, ".disabled");
                Directory.CreateDirectory(disabledDir);

                if (Path.GetFileName(filePath) == "SKILL.md")
                {
                    var parentDir = Path.GetDirectoryName(filePath)!;
                    var parentName = Path.GetFileName(parentDir);
                    var grandparentDir = Path.GetDirectoryName(parentDir)!;
                    var targetDir = Path.Combine(grandparentDir, ".disabled", parentName);
                    Directory.Move(parentDir, targetDir);
                }
                else
                {
                    targetPath = Path.Combine(disabledDir, Path.GetFileName(filePath));
                    File.Move(filePath, targetPath);
                }
            }
            else
            {
                if (Path.GetFileName(filePath) == "SKILL.md")
                {
                    var parentDir = Path.GetDirectoryName(filePath)!;
                    var parentName = Path.GetFileName(parentDir);
                    var disabledDir = Path.GetDirectoryName(parentDir)!; // .disabled/
                    var activeDir = Path.GetDirectoryName(disabledDir)!;
                    var targetDir = Path.Combine(activeDir, parentName);
                    Directory.Move(parentDir, targetDir);
                }
                else
                {
                    var disabledDir = Path.GetDirectoryName(filePath)!;
                    var activeDir = Path.GetDirectoryName(disabledDir)!;
                    targetPath = Path.Combine(activeDir, Path.GetFileName(filePath));
                    File.Move(filePath, targetPath);
                }
            }

            LoadSkills();
            return true;
        }
        catch (Exception ex)
        {
            SetStatus($"Toggle failed: {ex.Message}", isError: true);
            return false;
        }
    }

    // --- Agent create/delete ---

    public string? CreateAgent(string rawName)
    {
        var sanitized = SanitizeName(rawName);
        if (string.IsNullOrEmpty(sanitized)) return null;

        var dir = PathResolver.PersonalAgentsDir;
        Directory.CreateDirectory(dir);
        var path = Path.Combine(dir, $"{sanitized}.md");
        if (File.Exists(path)) return null;

        File.WriteAllText(path, ""); // Caller will overwrite with content
        return path;
    }

    public bool DeleteAgent(AgentEntry agent)
    {
        if (agent.Source != AgentSource.Personal) return false;
        try
        {
            File.Delete(agent.FilePath);
            LoadAgents();
            return true;
        }
        catch
        {
            return false;
        }
    }

    // --- Skill create ---

    public string? CreateSkill(string rawName, SkillSource source)
    {
        if (source == SkillSource.Plugin) return null;
        var sanitized = SanitizeName(rawName);
        if (string.IsNullOrEmpty(sanitized)) return null;

        var dir = source == SkillSource.Command ? PathResolver.CommandsDir : PathResolver.SkillsDir;
        Directory.CreateDirectory(dir);
        var path = Path.Combine(dir, $"{sanitized}.md");
        if (File.Exists(path)) return null;

        File.WriteAllText(path, ""); // Caller will overwrite
        return path;
    }

    // --- Input parsing ---

    public ParseResult ParseInput(string text)
    {
        if (string.IsNullOrWhiteSpace(text))
            return new ParseResult.Error("Paste a JSON server config");

        var result = TryClassify(text);
        if (result is not ParseResult.Error)
            return result;

        // Try wrapping in braces
        result = TryClassify("{" + text + "}");
        if (result is not ParseResult.Error)
            return result;

        return new ParseResult.Error("Invalid JSON — check syntax");
    }

    private ParseResult TryClassify(string text)
    {
        var obj = JsonHelper.ParseJsonObject(text);
        if (obj is null) return new ParseResult.Error("Invalid JSON");

        if (obj["mcpServers"] is JsonObject mcpServers)
        {
            var entries = new List<ServerEntry>();
            foreach (var kvp in mcpServers)
            {
                if (kvp.Value is JsonObject serverObj)
                    entries.Add(new ServerEntry(kvp.Key, JsonHelper.PrettyPrint(serverObj)));
            }
            return entries.Count > 0
                ? new ParseResult.Servers(entries)
                : new ParseResult.Error("No servers found in mcpServers");
        }

        if (obj.ContainsKey("command"))
            return new ParseResult.NeedsName(JsonHelper.PrettyPrint(obj));

        var servers = new List<ServerEntry>();
        foreach (var kvp in obj)
        {
            if (kvp.Value is JsonObject serverObj)
                servers.Add(new ServerEntry(kvp.Key, JsonHelper.PrettyPrint(serverObj)));
        }

        return servers.Count > 0
            ? new ParseResult.Servers(servers)
            : new ParseResult.Error("No valid server configs found");
    }

    // --- Helpers ---

    private static string SanitizeName(string name)
    {
        var lower = name.ToLowerInvariant().Replace(' ', '-');
        return new string(lower.Where(c => char.IsLetterOrDigit(c) || c == '-').ToArray());
    }
}

// --- ParseResult types ---

public abstract record ParseResult
{
    public record Servers(List<ServerEntry> Entries) : ParseResult;
    public record NeedsName(string ConfigJson) : ParseResult;
    public record Error(string Message) : ParseResult;
}
