using Configonaut.Models;

namespace Configonaut.Services;

public static class PathResolver
{
    private static readonly string _home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
    private static readonly string _appData = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData);
    private static readonly string _localAppData = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);

    // MSIX package family name for Claude Desktop
    private const string MsixPackageFamily = "Claude_pzs8sxrjxfjjc";

    /// <summary>Configonaut's own storage directory for stored servers, backups, etc.</summary>
    public static string StorageDir => Path.Combine(_appData, "Configonaut");

    /// <summary>CLI settings path — always %USERPROFILE%\.claude\settings.json</summary>
    public static string CliSettingsPath => Path.Combine(_home, ".claude", "settings.json");

    /// <summary>Personal agents directory</summary>
    public static string PersonalAgentsDir => Path.Combine(_home, ".claude", "agents");

    /// <summary>Custom commands directory</summary>
    public static string CommandsDir => Path.Combine(_home, ".claude", "commands");

    /// <summary>Custom skills directory</summary>
    public static string SkillsDir => Path.Combine(_home, ".claude", "skills");

    /// <summary>Plugins marketplace directory</summary>
    public static string PluginsDir => Path.Combine(_home, ".claude", "plugins", "marketplaces", "claude-plugins-official", "plugins");

    /// <summary>Check if Claude Desktop was installed via MSIX (Microsoft Store)</summary>
    public static bool IsMsixInstall
    {
        get
        {
            var msixConfig = Path.Combine(
                _localAppData, "Packages", MsixPackageFamily,
                "LocalCache", "Roaming", "Claude", "claude_desktop_config.json");
            return File.Exists(msixConfig);
        }
    }

    /// <summary>Standard (non-MSIX) Desktop config path</summary>
    public static string StandardDesktopConfigPath =>
        Path.Combine(_appData, "Claude", "claude_desktop_config.json");

    /// <summary>MSIX Desktop config path</summary>
    public static string MsixDesktopConfigPath =>
        Path.Combine(_localAppData, "Packages", MsixPackageFamily,
            "LocalCache", "Roaming", "Claude", "claude_desktop_config.json");

    /// <summary>Get the active config file path for the given mode</summary>
    public static string GetConfigPath(AppMode mode)
    {
        if (mode == AppMode.Cli)
            return CliSettingsPath;

        // Desktop mode: prefer MSIX if that config file exists
        return IsMsixInstall ? MsixDesktopConfigPath : StandardDesktopConfigPath;
    }

    /// <summary>Stored (inactive) servers file path, separated per mode</summary>
    public static string GetStoredServersPath(AppMode mode)
    {
        var key = mode.ToFileKey();
        return Path.Combine(StorageDir, $"stored_servers_{key}.json");
    }

    /// <summary>Backup directory, separated per mode</summary>
    public static string GetBackupDir(AppMode mode)
    {
        var key = mode.ToFileKey();
        return Path.Combine(StorageDir, "backups", key);
    }

    /// <summary>Replace user profile path with ~ for display</summary>
    public static string ShortenPath(string path)
    {
        if (path.StartsWith(_home, StringComparison.OrdinalIgnoreCase))
            return "~" + path[_home.Length..].Replace('\\', '/');
        return path;
    }

    /// <summary>Both standard and MSIX Desktop configs exist — show info banner</summary>
    public static bool HasDualDesktopConfigs =>
        File.Exists(StandardDesktopConfigPath) && File.Exists(MsixDesktopConfigPath);
}
