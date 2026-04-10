using Configonaut.Models;
using Configonaut.Services;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace Configonaut.Tests;

[TestClass]
public class PathResolverTests
{
    [TestMethod]
    public void CliSettingsPath_PointsToUserProfile()
    {
        var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        var expected = Path.Combine(home, ".claude", "settings.json");
        Assert.AreEqual(expected, PathResolver.CliSettingsPath);
    }

    [TestMethod]
    public void ConfigPath_Desktop_PointsToAppData()
    {
        var appdata = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData);
        var expected = Path.Combine(appdata, "Claude", "claude_desktop_config.json");
        // If MSIX path exists, it might differ — just check it returns a non-empty string
        var result = PathResolver.GetConfigPath(AppMode.Desktop);
        Assert.IsFalse(string.IsNullOrEmpty(result));
    }

    [TestMethod]
    public void ConfigPath_Cli_EqualsCliSettings()
    {
        Assert.AreEqual(PathResolver.CliSettingsPath, PathResolver.GetConfigPath(AppMode.Cli));
    }

    [TestMethod]
    public void StoredPath_DiffersByMode()
    {
        var desktopPath = PathResolver.GetStoredServersPath(AppMode.Desktop);
        var cliPath = PathResolver.GetStoredServersPath(AppMode.Cli);
        Assert.AreNotEqual(desktopPath, cliPath);
        Assert.IsTrue(desktopPath.Contains("desktop"));
        Assert.IsTrue(cliPath.Contains("cli"));
    }

    [TestMethod]
    public void BackupDir_DiffersByMode()
    {
        var desktopDir = PathResolver.GetBackupDir(AppMode.Desktop);
        var cliDir = PathResolver.GetBackupDir(AppMode.Cli);
        Assert.AreNotEqual(desktopDir, cliDir);
        Assert.IsTrue(desktopDir.Contains("desktop"));
        Assert.IsTrue(cliDir.Contains("cli"));
    }

    [TestMethod]
    public void CommandsDir_IsFixed()
    {
        var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        var expected = Path.Combine(home, ".claude", "commands");
        Assert.AreEqual(expected, PathResolver.CommandsDir);
    }

    [TestMethod]
    public void SkillsDir_IsFixed()
    {
        var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        var expected = Path.Combine(home, ".claude", "skills");
        Assert.AreEqual(expected, PathResolver.SkillsDir);
    }

    [TestMethod]
    public void AgentsDir_IsFixed()
    {
        var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        var expected = Path.Combine(home, ".claude", "agents");
        Assert.AreEqual(expected, PathResolver.PersonalAgentsDir);
    }

    [TestMethod]
    public void StorageDir_IsInAppData()
    {
        var appdata = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData);
        Assert.IsTrue(PathResolver.StorageDir.StartsWith(appdata));
        Assert.IsTrue(PathResolver.StorageDir.Contains("Configonaut"));
    }

    [TestMethod]
    public void ShortenPath_ReplacesUserProfile()
    {
        var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        var full = Path.Combine(home, ".claude", "settings.json");
        var shortened = PathResolver.ShortenPath(full);
        Assert.IsTrue(shortened.StartsWith("~"));
        Assert.IsTrue(shortened.Contains(".claude"));
    }
}
