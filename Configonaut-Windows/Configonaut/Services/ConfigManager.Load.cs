using System.Collections.ObjectModel;
using System.Text.Json.Nodes;
using CommunityToolkit.Mvvm.ComponentModel;
using Configonaut.Models;

namespace Configonaut.Services;

public partial class ConfigManager : ObservableObject
{
    // --- Published state (observable properties) ---

    [ObservableProperty]
    private ObservableCollection<ServerEntry> _activeServers = new();

    [ObservableProperty]
    private ObservableCollection<ServerEntry> _storedServers = new();

    [ObservableProperty]
    private ObservableCollection<HookRule> _hookRules = new();

    [ObservableProperty]
    private ObservableCollection<BackupFile> _backupFiles = new();

    [ObservableProperty]
    private ObservableCollection<AgentEntry> _agents = new();

    [ObservableProperty]
    private ObservableCollection<SkillEntry> _skills = new();

    [ObservableProperty]
    private Dictionary<string, bool> _enabledPlugins = new();

    [ObservableProperty]
    private string _statusMessage = "Ready";

    [ObservableProperty]
    private bool _statusIsError = false;

    [ObservableProperty]
    private bool _needsRestart = false;

    private AppMode _mode = AppMode.Desktop;
    public AppMode Mode
    {
        get => _mode;
        set
        {
            if (SetProperty(ref _mode, value))
            {
                // Persist to Windows settings
                try
                {
                    var localSettings = Windows.Storage.ApplicationData.Current.LocalSettings;
                    localSettings.Values["appMode"] = value.ToDisplayString();
                }
                catch { /* ApplicationData may not be available in tests */ }

                NeedsRestart = false;
                _lastBackupHash = null;
                ReloadAll();
            }
        }
    }

    // --- Private state ---
    private DateTime? _lastBackupDate;
    private int? _lastBackupHash;

    // --- Computed paths (delegate to PathResolver) ---
    public string ConfigPath => PathResolver.GetConfigPath(Mode);
    public string StoredPath => PathResolver.GetStoredServersPath(Mode);
    public string BackupDir => PathResolver.GetBackupDir(Mode);

    // --- Constructor ---

    public ConfigManager()
    {
        // Load persisted mode
        try
        {
            var localSettings = Windows.Storage.ApplicationData.Current.LocalSettings;
            if (localSettings.Values.TryGetValue("appMode", out var savedMode) && savedMode is string modeStr)
                _mode = AppModeExtensions.FromString(modeStr);
        }
        catch
        {
            // ApplicationData may not be available in tests — default to Desktop
        }

        MigrateIfNeeded();
        ReloadAll();
    }

    // --- Migration ---

    private void MigrateIfNeeded()
    {
        try
        {
            var localSettings = Windows.Storage.ApplicationData.Current.LocalSettings;
            if (localSettings.Values.ContainsKey("migrated_v2_mode_split"))
                return;

            var storageDir = PathResolver.StorageDir;

            // Migrate stored_servers.json -> stored_servers_desktop.json
            var oldStored = Path.Combine(storageDir, "stored_servers.json");
            var newStored = PathResolver.GetStoredServersPath(AppMode.Desktop);
            if (File.Exists(oldStored) && !File.Exists(newStored))
            {
                Directory.CreateDirectory(Path.GetDirectoryName(newStored)!);
                File.Move(oldStored, newStored);
            }

            // Migrate flat backups/ -> backups/desktop/
            var oldBackupDir = Path.Combine(storageDir, "backups");
            var desktopBackupDir = PathResolver.GetBackupDir(AppMode.Desktop);
            if (Directory.Exists(oldBackupDir) && !Directory.Exists(desktopBackupDir))
            {
                Directory.CreateDirectory(desktopBackupDir);
                foreach (var file in Directory.GetFiles(oldBackupDir, "*.json"))
                    File.Move(file, Path.Combine(desktopBackupDir, Path.GetFileName(file)));
            }

            // Create CLI backup dir
            Directory.CreateDirectory(PathResolver.GetBackupDir(AppMode.Cli));

            localSettings.Values["migrated_v2_mode_split"] = true;
        }
        catch
        {
            // Migration failure is non-fatal
        }
    }

    // --- Reload All ---

    public void ReloadAll()
    {
        LoadActive();
        LoadStored();
        LoadHooks();
        LoadBackups();
        LoadEnabledPlugins();
        LoadAgents();
        LoadSkills();
        OnPropertyChanged(nameof(ConfigPath));
        OnPropertyChanged(nameof(StoredPath));
        OnPropertyChanged(nameof(BackupDir));
    }

    // --- Load Methods ---

    public void LoadActive()
    {
        var root = JsonHelper.ReadJsonObject(ConfigPath);
        var servers = new ObservableCollection<ServerEntry>();

        if (root?["mcpServers"] is JsonObject mcpNode)
        {
            foreach (var kvp in mcpNode.OrderBy(k => k.Key, StringComparer.OrdinalIgnoreCase))
            {
                if (kvp.Value is JsonObject serverObj)
                {
                    var json = JsonHelper.PrettyPrint(serverObj);
                    servers.Add(new ServerEntry(kvp.Key, json));
                }
            }
        }

        ActiveServers = servers;
        SetStatus($"Loaded {servers.Count} active server{(servers.Count == 1 ? "" : "s")}");
    }

    public void LoadStored()
    {
        var root = JsonHelper.ReadJsonObject(StoredPath);
        var servers = new ObservableCollection<ServerEntry>();

        if (root is not null)
        {
            var activeNames = ActiveServers.Select(s => s.Name).ToHashSet();
            foreach (var kvp in root.OrderBy(k => k.Key, StringComparer.OrdinalIgnoreCase))
            {
                if (kvp.Value is JsonObject serverObj && !activeNames.Contains(kvp.Key))
                {
                    var json = JsonHelper.PrettyPrint(serverObj);
                    servers.Add(new ServerEntry(kvp.Key, json));
                }
            }
        }

        StoredServers = servers;
    }

    public void LoadHooks()
    {
        var settings = JsonHelper.ReadJsonObject(PathResolver.CliSettingsPath);
        var rules = new ObservableCollection<HookRule>();

        if (settings?["hooks"] is JsonObject hooksNode)
        {
            foreach (var kvp in hooksNode.OrderBy(k => k.Key, StringComparer.OrdinalIgnoreCase))
            {
                var eventName = kvp.Key;
                if (kvp.Value is not JsonArray ruleArray) continue;

                foreach (var ruleNode in ruleArray)
                {
                    if (ruleNode is not JsonObject ruleObj) continue;

                    var matcher = ruleObj["matcher"]?.GetValue<string>() ?? "*";
                    var isDisabled = ruleObj["disabled"]?.GetValue<bool>() ?? false;

                    var commands = new List<string>();
                    if (ruleObj["hooks"] is JsonArray hooksArray)
                    {
                        foreach (var hookItem in hooksArray)
                        {
                            if (hookItem is JsonObject hookObj)
                            {
                                var cmd = hookObj["command"]?.GetValue<string>();
                                if (!string.IsNullOrEmpty(cmd))
                                    commands.Add(cmd);
                            }
                        }
                    }

                    if (commands.Count > 0)
                        rules.Add(new HookRule(eventName, matcher, commands, !isDisabled));
                }
            }
        }

        HookRules = rules;
    }

    public void LoadBackups()
    {
        var backups = new ObservableCollection<BackupFile>();
        var dir = BackupDir;

        if (Directory.Exists(dir))
        {
            foreach (var file in Directory.GetFiles(dir, "*.json").OrderByDescending(f => f))
            {
                var info = new FileInfo(file);
                backups.Add(new BackupFile(
                    Path.GetFileName(file),
                    info.CreationTime,
                    info.Length
                ));
            }
        }

        BackupFiles = backups;
    }

    public void LoadEnabledPlugins()
    {
        var settings = JsonHelper.ReadJsonObject(PathResolver.CliSettingsPath);
        if (settings?["enabledPlugins"] is JsonObject pluginsNode)
        {
            var dict = new Dictionary<string, bool>();
            foreach (var kvp in pluginsNode)
            {
                if (kvp.Value is JsonValue val && val.TryGetValue<bool>(out var enabled))
                    dict[kvp.Key] = enabled;
            }
            EnabledPlugins = dict;
        }
        else
        {
            EnabledPlugins = new Dictionary<string, bool>();
        }
    }

    public void LoadAgents()
    {
        var list = new List<AgentEntry>();

        // Personal agents
        if (Directory.Exists(PathResolver.PersonalAgentsDir))
        {
            foreach (var file in Directory.GetFiles(PathResolver.PersonalAgentsDir, "*.md"))
            {
                var content = File.ReadAllText(file);
                var fm = JsonHelper.ParseFrontmatter(content);
                list.Add(new AgentEntry(
                    name: fm.GetValueOrDefault("name", Path.GetFileNameWithoutExtension(file)),
                    description: fm.GetValueOrDefault("description", ""),
                    tools: fm.TryGetValue("tools", out var t) ? t.Split(',').Select(s => s.Trim()).ToArray() : Array.Empty<string>(),
                    model: fm.GetValueOrDefault("model", ""),
                    color: fm.GetValueOrDefault("color", ""),
                    pluginName: "Personal",
                    filePath: file,
                    source: AgentSource.Personal,
                    isPluginEnabled: true
                ));
            }
        }

        // Plugin agents
        if (Directory.Exists(PathResolver.PluginsDir))
        {
            foreach (var pluginDir in Directory.GetDirectories(PathResolver.PluginsDir))
            {
                var pluginName = Path.GetFileName(pluginDir);
                var agentsDir = Path.Combine(pluginDir, "agents");
                if (!Directory.Exists(agentsDir)) continue;

                var pluginKey = $"{pluginName}@claude-plugins-official";
                var isEnabled = EnabledPlugins.TryGetValue(pluginKey, out var e) && e;

                foreach (var file in Directory.GetFiles(agentsDir, "*.md"))
                {
                    var content = File.ReadAllText(file);
                    var fm = JsonHelper.ParseFrontmatter(content);
                    list.Add(new AgentEntry(
                        name: fm.GetValueOrDefault("name", Path.GetFileNameWithoutExtension(file)),
                        description: fm.GetValueOrDefault("description", ""),
                        tools: fm.TryGetValue("tools", out var t2) ? t2.Split(',').Select(s => s.Trim()).ToArray() : Array.Empty<string>(),
                        model: fm.GetValueOrDefault("model", ""),
                        color: fm.GetValueOrDefault("color", ""),
                        pluginName: pluginName,
                        filePath: file,
                        source: AgentSource.Plugin,
                        isPluginEnabled: isEnabled
                    ));
                }
            }
        }

        // Sort: personal first, then by pluginName, then by name
        Agents = new ObservableCollection<AgentEntry>(
            list.OrderBy(a => a.Source == AgentSource.Personal ? 0 : 1)
                .ThenBy(a => a.PluginName, StringComparer.OrdinalIgnoreCase)
                .ThenBy(a => a.Name, StringComparer.OrdinalIgnoreCase)
        );
    }

    public void LoadSkills()
    {
        var list = new List<SkillEntry>();

        ScanSkillDir(PathResolver.CommandsDir, SkillSource.Command, list);
        ScanSkillDir(PathResolver.SkillsDir, SkillSource.Skill, list);

        // Plugin skills
        if (Directory.Exists(PathResolver.PluginsDir))
        {
            foreach (var pluginDir in Directory.GetDirectories(PathResolver.PluginsDir))
            {
                var skillsDir = Path.Combine(pluginDir, "skills");
                if (!Directory.Exists(skillsDir)) continue;

                // Direct .md files
                foreach (var file in Directory.GetFiles(skillsDir, "*.md"))
                {
                    var content = File.ReadAllText(file);
                    var fm = JsonHelper.ParseFrontmatter(content);
                    list.Add(new SkillEntry(
                        name: fm.GetValueOrDefault("name", Path.GetFileNameWithoutExtension(file)),
                        description: fm.GetValueOrDefault("description", ""),
                        source: SkillSource.Plugin,
                        filePath: file,
                        isEnabled: true
                    ));
                }

                // Subdirectory SKILL.md pattern
                foreach (var subDir in Directory.GetDirectories(skillsDir))
                {
                    var skillMd = Path.Combine(subDir, "SKILL.md");
                    if (!File.Exists(skillMd)) continue;
                    var content = File.ReadAllText(skillMd);
                    var fm = JsonHelper.ParseFrontmatter(content);
                    list.Add(new SkillEntry(
                        name: fm.GetValueOrDefault("name", Path.GetFileName(subDir)),
                        description: fm.GetValueOrDefault("description", ""),
                        source: SkillSource.Plugin,
                        filePath: skillMd,
                        isEnabled: true
                    ));
                }
            }
        }

        Skills = new ObservableCollection<SkillEntry>(
            list.OrderBy(s => s.Name, StringComparer.OrdinalIgnoreCase)
        );
    }

    private void ScanSkillDir(string dir, SkillSource source, List<SkillEntry> list)
    {
        if (!Directory.Exists(dir)) return;

        // Enabled skills
        foreach (var file in Directory.GetFiles(dir, "*.md"))
        {
            var content = File.ReadAllText(file);
            var fm = JsonHelper.ParseFrontmatter(content);
            list.Add(new SkillEntry(
                name: fm.GetValueOrDefault("name", Path.GetFileNameWithoutExtension(file)),
                description: fm.GetValueOrDefault("description", ""),
                source: source,
                filePath: file,
                isEnabled: true
            ));
        }

        // Disabled skills (in .disabled/ subdirectory)
        var disabledDir = Path.Combine(dir, ".disabled");
        if (!Directory.Exists(disabledDir)) return;

        foreach (var file in Directory.GetFiles(disabledDir, "*.md"))
        {
            var content = File.ReadAllText(file);
            var fm = JsonHelper.ParseFrontmatter(content);
            list.Add(new SkillEntry(
                name: fm.GetValueOrDefault("name", Path.GetFileNameWithoutExtension(file)),
                description: fm.GetValueOrDefault("description", ""),
                source: source,
                filePath: file,
                isEnabled: false
            ));
        }
    }

    // --- Status helper ---

    private void SetStatus(string message, bool isError = false)
    {
        StatusMessage = message;
        StatusIsError = isError;
    }
}
