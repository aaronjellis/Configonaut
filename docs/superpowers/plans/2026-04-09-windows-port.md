# Configonaut Windows Port Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a WinUI 3 / C# Windows app with full feature parity to the macOS Configonaut app — managing Claude Desktop and Claude Code config files (MCP servers, hooks, agents, skills, backups) with a Desktop/CLI mode toggle.

**Architecture:** MVVM with a single `ConfigManager` as the central observable state object (mirrors the macOS `ObservableObject` pattern). Views bind directly to ConfigManager properties. PathResolver handles Windows-specific config path resolution including MSIX detection. Dark theme with neon accent colors matching the macOS aesthetic.

**Tech Stack:** .NET 8, WinUI 3 (Windows App SDK 1.6), CommunityToolkit.Mvvm (source generators for INotifyPropertyChanged), CommunityToolkit.WinUI (Segmented control), System.Text.Json

**Repository:** `Configonaut-Windows` (new repo, separate from macOS)

---

## File Structure

```
Configonaut-Windows/
├── Configonaut/
│   ├── Configonaut.csproj
│   ├── App.xaml
│   ├── App.xaml.cs
│   ├── MainWindow.xaml
│   ├── MainWindow.xaml.cs
│   ├── Models/
│   │   ├── AppMode.cs
│   │   ├── ServerEntry.cs
│   │   ├── HookRule.cs
│   │   ├── AgentEntry.cs
│   │   ├── SkillEntry.cs
│   │   └── BackupFile.cs
│   ├── Services/
│   │   ├── ConfigManager.cs
│   │   ├── PathResolver.cs
│   │   └── JsonHelper.cs
│   ├── Views/
│   │   ├── MCPPage.xaml
│   │   ├── MCPPage.xaml.cs
│   │   ├── HooksPage.xaml
│   │   ├── HooksPage.xaml.cs
│   │   ├── AgentsPage.xaml
│   │   ├── AgentsPage.xaml.cs
│   │   ├── SkillsPage.xaml
│   │   ├── SkillsPage.xaml.cs
│   │   ├── BackupsPage.xaml
│   │   └── BackupsPage.xaml.cs
│   ├── Converters/
│   │   └── Converters.cs
│   ├── Assets/
│   │   └── AppIcon.png
│   └── Package.appxmanifest
├── Configonaut.Tests/
│   ├── Configonaut.Tests.csproj
│   ├── PathResolverTests.cs
│   ├── JsonHelperTests.cs
│   └── ModelTests.cs
├── Configonaut.sln
├── .gitignore
└── README.md
```

---

### Task 1: Project Scaffold

**Files:**
- Create: `Configonaut-Windows/.gitignore`
- Create: `Configonaut-Windows/Configonaut.sln`
- Create: `Configonaut-Windows/Configonaut/Configonaut.csproj`
- Create: `Configonaut-Windows/Configonaut.Tests/Configonaut.Tests.csproj`

- [ ] **Step 1: Create repository and .gitignore**

```bash
cd /tmp
mkdir Configonaut-Windows && cd Configonaut-Windows
git init
```

Write `.gitignore`:

```gitignore
## .NET
bin/
obj/
*.user
*.suo
*.cache
*.log
.vs/
*.DotSettings.user

## Build results
[Dd]ebug/
[Rr]elease/
x64/
x86/
[Aa][Rr][Mm]/
bld/
[Bb]in/
[Oo]bj/

## NuGet
*.nupkg
**/packages/*
project.lock.json
project.fragment.lock.json
artifacts/

## Windows
Thumbs.db
ehthumbs.db
Desktop.ini
$RECYCLE.BIN/

## MSIX
*.msix
*.appx
AppPackages/
BundleArtifacts/
```

- [ ] **Step 2: Create solution and project files**

Create `Configonaut.sln`:

```xml
Microsoft Visual Studio Solution File, Format Version 12.00
# Visual Studio Version 17
VisualStudioVersion = 17.0.31903.59
MinimumVisualStudioVersion = 10.0.40219.1
Project("{9A19103F-16F7-4668-BE54-9A1E7A4F7556}") = "Configonaut", "Configonaut\Configonaut.csproj", "{A1B2C3D4-E5F6-7890-ABCD-EF1234567890}"
EndProject
Project("{9A19103F-16F7-4668-BE54-9A1E7A4F7556}") = "Configonaut.Tests", "Configonaut.Tests\Configonaut.Tests.csproj", "{B2C3D4E5-F6A7-8901-BCDE-F12345678901}"
EndProject
Global
	GlobalSection(SolutionConfigurationPlatforms) = preSolution
		Debug|x64 = Debug|x64
		Release|x64 = Release|x64
	EndGlobalSection
	GlobalSection(ProjectConfigurationPlatforms) = postSolution
		{A1B2C3D4-E5F6-7890-ABCD-EF1234567890}.Debug|x64.ActiveCfg = Debug|x64
		{A1B2C3D4-E5F6-7890-ABCD-EF1234567890}.Debug|x64.Build.0 = Debug|x64
		{A1B2C3D4-E5F6-7890-ABCD-EF1234567890}.Release|x64.ActiveCfg = Release|x64
		{A1B2C3D4-E5F6-7890-ABCD-EF1234567890}.Release|x64.Build.0 = Release|x64
		{B2C3D4E5-F6A7-8901-BCDE-F12345678901}.Debug|x64.ActiveCfg = Debug|x64
		{B2C3D4E5-F6A7-8901-BCDE-F12345678901}.Debug|x64.Build.0 = Debug|x64
		{B2C3D4E5-F6A7-8901-BCDE-F12345678901}.Release|x64.ActiveCfg = Release|x64
		{B2C3D4E5-F6A7-8901-BCDE-F12345678901}.Release|x64.Build.0 = Release|x64
	EndGlobalSection
EndGlobal
```

Create `Configonaut/Configonaut.csproj`:

```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>WinExe</OutputType>
    <TargetFramework>net8.0-windows10.0.22621.0</TargetFramework>
    <WindowsSdkPackageVersion>10.0.22621.45</WindowsSdkPackageVersion>
    <RootNamespace>Configonaut</RootNamespace>
    <ApplicationManifest>app.manifest</ApplicationManifest>
    <Platforms>x64</Platforms>
    <RuntimeIdentifiers>win-x64</RuntimeIdentifiers>
    <UseWinUI>true</UseWinUI>
    <Nullable>enable</Nullable>
    <ImplicitUsings>enable</ImplicitUsings>
  </PropertyGroup>

  <ItemGroup>
    <PackageReference Include="Microsoft.WindowsAppSDK" Version="1.6.*" />
    <PackageReference Include="CommunityToolkit.Mvvm" Version="8.*" />
    <PackageReference Include="CommunityToolkit.WinUI.Controls.Segmented" Version="8.*" />
  </ItemGroup>
</Project>
```

Create `Configonaut.Tests/Configonaut.Tests.csproj`:

```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0-windows10.0.22621.0</TargetFramework>
    <WindowsSdkPackageVersion>10.0.22621.45</WindowsSdkPackageVersion>
    <Platforms>x64</Platforms>
    <RuntimeIdentifiers>win-x64</RuntimeIdentifiers>
    <UseWinUI>true</UseWinUI>
    <Nullable>enable</Nullable>
    <ImplicitUsings>enable</ImplicitUsings>
    <IsPackable>false</IsPackable>
    <IsTestProject>true</IsTestProject>
  </PropertyGroup>

  <ItemGroup>
    <PackageReference Include="Microsoft.NET.Test.Sdk" Version="17.*" />
    <PackageReference Include="MSTest.TestAdapter" Version="3.*" />
    <PackageReference Include="MSTest.TestFramework" Version="3.*" />
  </ItemGroup>

  <ItemGroup>
    <ProjectReference Include="..\Configonaut\Configonaut.csproj" />
  </ItemGroup>
</Project>
```

- [ ] **Step 3: Create directory structure**

```bash
mkdir -p Configonaut/Models Configonaut/Services Configonaut/Views Configonaut/Converters Configonaut/Assets
mkdir -p Configonaut.Tests
```

- [ ] **Step 4: Verify solution builds**

```bash
dotnet restore
dotnet build --configuration Debug
```

Expected: Build succeeds (with warnings about missing App.xaml — that's fine for now).

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "chore: scaffold WinUI 3 solution with test project"
```

---

### Task 2: Data Models

**Files:**
- Create: `Configonaut/Models/AppMode.cs`
- Create: `Configonaut/Models/ServerEntry.cs`
- Create: `Configonaut/Models/HookRule.cs`
- Create: `Configonaut/Models/AgentEntry.cs`
- Create: `Configonaut/Models/SkillEntry.cs`
- Create: `Configonaut/Models/BackupFile.cs`
- Create: `Configonaut.Tests/ModelTests.cs`

- [ ] **Step 1: Write model tests**

Create `Configonaut.Tests/ModelTests.cs`:

```csharp
using Configonaut.Models;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace Configonaut.Tests;

[TestClass]
public class ModelTests
{
    [TestMethod]
    public void AppMode_HasDesktopAndCli()
    {
        Assert.AreEqual("Desktop", AppMode.Desktop.ToDisplayString());
        Assert.AreEqual("CLI", AppMode.Cli.ToDisplayString());
    }

    [TestMethod]
    public void AppMode_FromString_RoundTrips()
    {
        Assert.AreEqual(AppMode.Desktop, AppModeExtensions.FromString("Desktop"));
        Assert.AreEqual(AppMode.Cli, AppModeExtensions.FromString("CLI"));
        Assert.AreEqual(AppMode.Desktop, AppModeExtensions.FromString("invalid"));
    }

    [TestMethod]
    public void ServerEntry_IdentityIsName()
    {
        var server = new ServerEntry("test-server", "{}");
        Assert.AreEqual("test-server", server.Name);
        Assert.AreEqual("{}", server.ConfigJson);
    }

    [TestMethod]
    public void ServerEntry_Equality_ByName()
    {
        var a = new ServerEntry("s1", "{\"a\":1}");
        var b = new ServerEntry("s1", "{\"b\":2}");
        Assert.AreEqual(a, b);
    }

    [TestMethod]
    public void HookRule_DefaultMatcher_IsStar()
    {
        var rule = new HookRule("PreToolUse", "*", new[] { "echo hello" }, true);
        Assert.AreEqual("PreToolUse", rule.Event);
        Assert.AreEqual("*", rule.Matcher);
        Assert.IsTrue(rule.IsEnabled);
        Assert.AreEqual(1, rule.Commands.Count);
    }

    [TestMethod]
    public void BackupFile_FormattedSize_Works()
    {
        var backup = new BackupFile("test.json", DateTime.Now, 4200);
        Assert.IsTrue(backup.FormattedSize.Contains("KB") || backup.FormattedSize.Contains("4"));
    }

    [TestMethod]
    public void AgentEntry_PluginDisabled_Tracks()
    {
        var agent = new AgentEntry(
            name: "test-agent",
            description: "A test",
            tools: new[] { "Read", "Write" },
            model: "sonnet",
            color: "blue",
            pluginName: "my-plugin",
            filePath: @"C:\test\agent.md",
            source: AgentSource.Plugin,
            isPluginEnabled: false
        );
        Assert.AreEqual(AgentSource.Plugin, agent.Source);
        Assert.IsFalse(agent.IsPluginEnabled);
    }

    [TestMethod]
    public void SkillEntry_EnabledState_Tracks()
    {
        var skill = new SkillEntry(
            name: "my-skill",
            description: "Does stuff",
            source: SkillSource.Command,
            filePath: @"C:\test\skill.md",
            isEnabled: true
        );
        Assert.IsTrue(skill.IsEnabled);
        Assert.AreEqual(SkillSource.Command, skill.Source);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
dotnet test --configuration Debug
```

Expected: FAIL — types not defined yet.

- [ ] **Step 3: Create AppMode.cs**

```csharp
namespace Configonaut.Models;

public enum AppMode
{
    Desktop,
    Cli
}

public static class AppModeExtensions
{
    public static string ToDisplayString(this AppMode mode) => mode switch
    {
        AppMode.Desktop => "Desktop",
        AppMode.Cli => "CLI",
        _ => "Desktop"
    };

    public static AppMode FromString(string value) => value switch
    {
        "CLI" => AppMode.Cli,
        "Desktop" => AppMode.Desktop,
        _ => AppMode.Desktop
    };

    public static string ToFileKey(this AppMode mode) => mode switch
    {
        AppMode.Desktop => "desktop",
        AppMode.Cli => "cli",
        _ => "desktop"
    };
}
```

- [ ] **Step 4: Create ServerEntry.cs**

```csharp
namespace Configonaut.Models;

public enum ServerSource
{
    Active,
    Stored
}

public record ServerEntry(string Name, string ConfigJson)
{
    public virtual bool Equals(ServerEntry? other) => other is not null && Name == other.Name;
    public override int GetHashCode() => Name.GetHashCode();
}
```

- [ ] **Step 5: Create HookRule.cs**

```csharp
namespace Configonaut.Models;

public class HookRule
{
    public Guid Id { get; } = Guid.NewGuid();
    public string Event { get; }
    public string Matcher { get; }
    public IReadOnlyList<string> Commands { get; }
    public bool IsEnabled { get; }

    public HookRule(string @event, string matcher, IEnumerable<string> commands, bool isEnabled)
    {
        Event = @event;
        Matcher = matcher;
        Commands = commands.ToList().AsReadOnly();
        IsEnabled = isEnabled;
    }
}
```

- [ ] **Step 6: Create AgentEntry.cs**

```csharp
namespace Configonaut.Models;

public enum AgentSource
{
    Personal,
    Plugin
}

public class AgentEntry
{
    public string Name { get; }
    public string Description { get; }
    public IReadOnlyList<string> Tools { get; }
    public string Model { get; }
    public string Color { get; }
    public string PluginName { get; }
    public string FilePath { get; }
    public AgentSource Source { get; }
    public bool IsPluginEnabled { get; }
    public string Id => FilePath;

    public AgentEntry(
        string name, string description, IEnumerable<string> tools,
        string model, string color, string pluginName,
        string filePath, AgentSource source, bool isPluginEnabled)
    {
        Name = name;
        Description = description;
        Tools = tools.ToList().AsReadOnly();
        Model = model;
        Color = color;
        PluginName = pluginName;
        FilePath = filePath;
        Source = source;
        IsPluginEnabled = isPluginEnabled;
    }
}
```

- [ ] **Step 7: Create SkillEntry.cs**

```csharp
namespace Configonaut.Models;

public enum SkillSource
{
    Command,
    Skill,
    Plugin
}

public class SkillEntry
{
    public string Name { get; }
    public string Description { get; }
    public SkillSource Source { get; }
    public string FilePath { get; }
    public bool IsEnabled { get; }
    public string Id => FilePath;

    public string SourceLabel => Source switch
    {
        SkillSource.Command => "Custom Command",
        SkillSource.Skill => "Custom Skill",
        SkillSource.Plugin => "Plugin",
        _ => "Unknown"
    };

    public SkillEntry(string name, string description, SkillSource source, string filePath, bool isEnabled)
    {
        Name = name;
        Description = description;
        Source = source;
        FilePath = filePath;
        IsEnabled = isEnabled;
    }
}
```

- [ ] **Step 8: Create BackupFile.cs**

```csharp
namespace Configonaut.Models;

public class BackupFile
{
    public string FileName { get; }
    public DateTime Date { get; }
    public long SizeBytes { get; }
    public string Id => FileName;

    public string FormattedDate => Date.ToString("MMM d, yyyy h:mm tt");

    public string FormattedSize
    {
        get
        {
            if (SizeBytes < 1024) return $"{SizeBytes} B";
            if (SizeBytes < 1024 * 1024) return $"{SizeBytes / 1024.0:F1} KB";
            return $"{SizeBytes / (1024.0 * 1024.0):F1} MB";
        }
    }

    public BackupFile(string fileName, DateTime date, long sizeBytes)
    {
        FileName = fileName;
        Date = date;
        SizeBytes = sizeBytes;
    }
}
```

- [ ] **Step 9: Run tests to verify they pass**

```bash
dotnet test --configuration Debug
```

Expected: All 7 tests PASS.

- [ ] **Step 10: Commit**

```bash
git add -A
git commit -m "feat: add data models (AppMode, ServerEntry, HookRule, AgentEntry, SkillEntry, BackupFile)"
```

---

### Task 3: PathResolver

**Files:**
- Create: `Configonaut/Services/PathResolver.cs`
- Create: `Configonaut.Tests/PathResolverTests.cs`

- [ ] **Step 1: Write PathResolver tests**

Create `Configonaut.Tests/PathResolverTests.cs`:

```csharp
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
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
dotnet test --configuration Debug
```

Expected: FAIL — `PathResolver` not defined.

- [ ] **Step 3: Implement PathResolver**

Create `Configonaut/Services/PathResolver.cs`:

```csharp
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
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
dotnet test --configuration Debug
```

Expected: All 10 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: add PathResolver with Windows config path resolution and MSIX detection"
```

---

### Task 4: JsonHelper

**Files:**
- Create: `Configonaut/Services/JsonHelper.cs`
- Create: `Configonaut.Tests/JsonHelperTests.cs`

- [ ] **Step 1: Write JsonHelper tests**

Create `Configonaut.Tests/JsonHelperTests.cs`:

```csharp
using Configonaut.Services;
using Microsoft.VisualStudio.TestTools.UnitTesting;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace Configonaut.Tests;

[TestClass]
public class JsonHelperTests
{
    private string _tempDir = null!;

    [TestInitialize]
    public void Setup()
    {
        _tempDir = Path.Combine(Path.GetTempPath(), "ConfigonautTests_" + Guid.NewGuid().ToString("N")[..8]);
        Directory.CreateDirectory(_tempDir);
    }

    [TestCleanup]
    public void Cleanup()
    {
        if (Directory.Exists(_tempDir))
            Directory.Delete(_tempDir, recursive: true);
    }

    [TestMethod]
    public void ReadJsonObject_ReturnsNull_WhenFileMissing()
    {
        var result = JsonHelper.ReadJsonObject(Path.Combine(_tempDir, "nope.json"));
        Assert.IsNull(result);
    }

    [TestMethod]
    public void ReadJsonObject_ReturnsNull_WhenInvalidJson()
    {
        var path = Path.Combine(_tempDir, "bad.json");
        File.WriteAllText(path, "not json {{{");
        var result = JsonHelper.ReadJsonObject(path);
        Assert.IsNull(result);
    }

    [TestMethod]
    public void WriteJsonObject_CreatesParentDirs()
    {
        var path = Path.Combine(_tempDir, "sub", "dir", "test.json");
        var obj = new JsonObject { ["key"] = "value" };
        JsonHelper.WriteJsonObject(path, obj);

        Assert.IsTrue(File.Exists(path));
        var readBack = JsonHelper.ReadJsonObject(path);
        Assert.IsNotNull(readBack);
        Assert.AreEqual("value", readBack!["key"]?.GetValue<string>());
    }

    [TestMethod]
    public void WriteJsonObject_PrettyPrintsAndSortsKeys()
    {
        var path = Path.Combine(_tempDir, "sorted.json");
        var obj = new JsonObject { ["zebra"] = 1, ["alpha"] = 2 };
        JsonHelper.WriteJsonObject(path, obj);

        var text = File.ReadAllText(path);
        var alphaIdx = text.IndexOf("alpha");
        var zebraIdx = text.IndexOf("zebra");
        Assert.IsTrue(alphaIdx < zebraIdx, "Keys should be sorted alphabetically");
        Assert.IsTrue(text.Contains('\n'), "Should be pretty-printed");
    }

    [TestMethod]
    public void RoundTrip_PreservesData()
    {
        var path = Path.Combine(_tempDir, "roundtrip.json");
        var obj = new JsonObject
        {
            ["mcpServers"] = new JsonObject
            {
                ["server1"] = new JsonObject { ["command"] = "npx", ["args"] = new JsonArray("@test/mcp") }
            }
        };
        JsonHelper.WriteJsonObject(path, obj);

        var readBack = JsonHelper.ReadJsonObject(path);
        Assert.IsNotNull(readBack);
        var servers = readBack!["mcpServers"]?.AsObject();
        Assert.IsNotNull(servers);
        Assert.IsTrue(servers!.ContainsKey("server1"));
    }

    [TestMethod]
    public void PrettyPrint_FormatsSingleObject()
    {
        var obj = new JsonObject { ["command"] = "node", ["args"] = new JsonArray("server.js") };
        var pretty = JsonHelper.PrettyPrint(obj);
        Assert.IsTrue(pretty.Contains("command"));
        Assert.IsTrue(pretty.Contains('\n'));
    }

    [TestMethod]
    public void ParseFrontmatter_ExtractsKeyValues()
    {
        var content = "---\nname: test-agent\ndescription: A test agent\nmodel: sonnet\n---\nBody content here";
        var fm = JsonHelper.ParseFrontmatter(content);
        Assert.AreEqual("test-agent", fm["name"]);
        Assert.AreEqual("A test agent", fm["description"]);
        Assert.AreEqual("sonnet", fm["model"]);
    }

    [TestMethod]
    public void ParseFrontmatter_StripsQuotes()
    {
        var content = "---\nname: \"quoted-name\"\ndescription: 'single-quoted'\n---\nBody";
        var fm = JsonHelper.ParseFrontmatter(content);
        Assert.AreEqual("quoted-name", fm["name"]);
        Assert.AreEqual("single-quoted", fm["description"]);
    }

    [TestMethod]
    public void ParseFrontmatter_ReturnsEmpty_WhenNoFrontmatter()
    {
        var content = "No frontmatter here";
        var fm = JsonHelper.ParseFrontmatter(content);
        Assert.AreEqual(0, fm.Count);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
dotnet test --configuration Debug
```

Expected: FAIL — `JsonHelper` not defined.

- [ ] **Step 3: Implement JsonHelper**

Create `Configonaut/Services/JsonHelper.cs`:

```csharp
using System.Text.Json;
using System.Text.Json.Nodes;

namespace Configonaut.Services;

public static class JsonHelper
{
    private static readonly JsonSerializerOptions _writeOptions = new()
    {
        WriteIndented = true,
        Encoder = System.Text.Encodings.Web.JavaScriptEncoder.UnsafeRelaxedJsonEscaping
    };

    /// <summary>Read a JSON file and parse as JsonObject. Returns null if missing or invalid.</summary>
    public static JsonObject? ReadJsonObject(string path)
    {
        try
        {
            if (!File.Exists(path)) return null;
            var text = File.ReadAllText(path);
            var node = JsonNode.Parse(text);
            return node?.AsObject();
        }
        catch
        {
            return null;
        }
    }

    /// <summary>Write a JsonObject to a file with pretty-printing and sorted keys. Creates parent directories.</summary>
    public static void WriteJsonObject(string path, JsonObject obj)
    {
        var dir = Path.GetDirectoryName(path);
        if (!string.IsNullOrEmpty(dir))
            Directory.CreateDirectory(dir);

        var sorted = SortKeys(obj);
        var json = sorted.ToJsonString(_writeOptions);

        // Atomic write: write to temp file then rename
        var tempPath = path + ".tmp";
        File.WriteAllText(tempPath, json);
        File.Move(tempPath, path, overwrite: true);
    }

    /// <summary>Pretty-print a JsonNode for display (e.g., in the JSON editor)</summary>
    public static string PrettyPrint(JsonNode node)
    {
        return node.ToJsonString(_writeOptions);
    }

    /// <summary>Parse a JSON string into a JsonObject. Returns null on failure.</summary>
    public static JsonObject? ParseJsonObject(string text)
    {
        try
        {
            var node = JsonNode.Parse(text);
            return node?.AsObject();
        }
        catch
        {
            return null;
        }
    }

    /// <summary>Parse YAML-style frontmatter from a markdown file (--- delimited)</summary>
    public static Dictionary<string, string> ParseFrontmatter(string content)
    {
        var result = new Dictionary<string, string>();
        if (!content.StartsWith("---"))
            return result;

        var lines = content.Split('\n');
        var inFrontmatter = false;

        foreach (var line in lines)
        {
            var trimmed = line.Trim();
            if (trimmed == "---")
            {
                if (!inFrontmatter) { inFrontmatter = true; continue; }
                break; // closing ---
            }

            if (!inFrontmatter) continue;

            var colonIdx = trimmed.IndexOf(':');
            if (colonIdx <= 0) continue;

            var key = trimmed[..colonIdx].Trim();
            var value = trimmed[(colonIdx + 1)..].Trim();

            // Strip surrounding quotes
            if (value.Length >= 2)
            {
                if ((value[0] == '"' && value[^1] == '"') ||
                    (value[0] == '\'' && value[^1] == '\''))
                {
                    value = value[1..^1];
                }
            }

            result[key] = value;
        }

        return result;
    }

    /// <summary>Recursively sort JSON object keys alphabetically</summary>
    private static JsonObject SortKeys(JsonObject obj)
    {
        var sorted = new JsonObject();
        foreach (var kvp in obj.OrderBy(k => k.Key, StringComparer.Ordinal))
        {
            JsonNode? value = kvp.Value;
            if (value is JsonObject childObj)
                sorted[kvp.Key] = SortKeys(childObj);
            else if (value is JsonArray arr)
                sorted[kvp.Key] = SortArray(arr);
            else
                sorted[kvp.Key] = value?.DeepClone();
        }
        return sorted;
    }

    private static JsonArray SortArray(JsonArray arr)
    {
        var newArr = new JsonArray();
        foreach (var item in arr)
        {
            if (item is JsonObject childObj)
                newArr.Add(SortKeys(childObj));
            else
                newArr.Add(item?.DeepClone());
        }
        return newArr;
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
dotnet test --configuration Debug
```

Expected: All 9 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: add JsonHelper with sorted-key JSON I/O and frontmatter parser"
```

---

### Task 5: ConfigManager — Loading

**Files:**
- Create: `Configonaut/Services/ConfigManager.cs`

This task implements ConfigManager's properties, initialization, migration, and all load methods. Mutation methods are in Task 6.

- [ ] **Step 1: Create ConfigManager with properties and loading**

Create `Configonaut/Services/ConfigManager.cs`:

```csharp
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
                var localSettings = Windows.Storage.ApplicationData.Current.LocalSettings;
                localSettings.Values["appMode"] = value.ToDisplayString();
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
```

- [ ] **Step 2: Verify it compiles**

```bash
dotnet build --configuration Debug
```

Expected: Build succeeds.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: add ConfigManager with all load methods and migration logic"
```

---

### Task 6: ConfigManager — Mutations

**Files:**
- Modify: `Configonaut/Services/ConfigManager.cs`

This task adds all mutation methods: server add/move/delete/update, backup create/restore, hook toggle/update, plugin toggle, skill toggle, agent/skill create/delete, and the input parser.

- [ ] **Step 1: Add mutation methods to ConfigManager**

Append to the end of `ConfigManager.cs` (before the closing `}`):

```csharp
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
        // Read active config
        var configRoot = JsonHelper.ReadJsonObject(ConfigPath);
        if (configRoot?["mcpServers"] is not JsonObject mcpServers || !mcpServers.ContainsKey(name))
            return false;

        var serverObj = mcpServers[name]!.DeepClone();
        mcpServers.Remove(name);

        // Add to stored
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

        if (!SaveSettings(settings)) return false;
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

        if (!SaveSettings(settings)) return false;
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
                // Move to .disabled/
                var dir = Path.GetDirectoryName(filePath)!;
                var disabledDir = Path.Combine(dir, ".disabled");
                Directory.CreateDirectory(disabledDir);

                // Check if it's a SKILL.md inside a subdirectory
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
                // Move from .disabled/ back to active
                if (Path.GetFileName(filePath) == "SKILL.md")
                {
                    var parentDir = Path.GetDirectoryName(filePath)!;
                    var parentName = Path.GetFileName(parentDir);
                    var disabledDir = Path.GetDirectoryName(parentDir)!; // .disabled/
                    var activeDir = Path.GetDirectoryName(disabledDir)!; // parent of .disabled/
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

        // Try as-is
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

        // Has mcpServers wrapper?
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

        // Bare server config (has "command" key)?
        if (obj.ContainsKey("command"))
            return new ParseResult.NeedsName(JsonHelper.PrettyPrint(obj));

        // Each top-level key is a server name
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
```

- [ ] **Step 2: Verify it compiles**

```bash
dotnet build --configuration Debug
```

Expected: Build succeeds.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: add ConfigManager mutation methods (server CRUD, hooks, backups, skills, input parser)"
```

---

### Task 7: Theme & Converters

**Files:**
- Create: `Configonaut/Theme/NeonTheme.xaml`
- Create: `Configonaut/Converters/Converters.cs`

- [ ] **Step 1: Create NeonTheme.xaml**

Create `Configonaut/Theme/NeonTheme.xaml`:

```xml
<ResourceDictionary
    xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">

    <!-- Background Colors -->
    <Color x:Key="BgPrimary">#FF0A0E15</Color>
    <Color x:Key="BgElevated">#FF111827</Color>

    <!-- Accent Colors -->
    <Color x:Key="NeonGreen">#FF00F5A0</Color>
    <Color x:Key="NeonRed">#FFFF5C8A</Color>
    <Color x:Key="NeonBlue">#FF00B4FF</Color>
    <Color x:Key="NeonPurple">#FFB36CFF</Color>
    <Color x:Key="NeonAmber">#FFFFD43B</Color>
    <Color x:Key="NeonCyan">#FF00E5FF</Color>
    <Color x:Key="NeonOrange">#FFFF8A3D</Color>

    <!-- Brushes -->
    <SolidColorBrush x:Key="BgPrimaryBrush" Color="{StaticResource BgPrimary}" />
    <SolidColorBrush x:Key="BgElevatedBrush" Color="{StaticResource BgElevated}" />
    <SolidColorBrush x:Key="NeonGreenBrush" Color="{StaticResource NeonGreen}" />
    <SolidColorBrush x:Key="NeonRedBrush" Color="{StaticResource NeonRed}" />
    <SolidColorBrush x:Key="NeonBlueBrush" Color="{StaticResource NeonBlue}" />
    <SolidColorBrush x:Key="NeonPurpleBrush" Color="{StaticResource NeonPurple}" />
    <SolidColorBrush x:Key="NeonAmberBrush" Color="{StaticResource NeonAmber}" />
    <SolidColorBrush x:Key="NeonCyanBrush" Color="{StaticResource NeonCyan}" />
    <SolidColorBrush x:Key="NeonOrangeBrush" Color="{StaticResource NeonOrange}" />

    <!-- Card surfaces -->
    <SolidColorBrush x:Key="CardFillBrush" Color="#0AFFFFFF" />
    <SolidColorBrush x:Key="CardHoverBrush" Color="#14FFFFFF" />
    <SolidColorBrush x:Key="CardBorderBrush" Color="#14FFFFFF" />
    <SolidColorBrush x:Key="SubtleBorderBrush" Color="#0FFFFFFF" />

    <!-- Text -->
    <SolidColorBrush x:Key="PrimaryTextBrush" Color="#FFFFFFFF" />
    <SolidColorBrush x:Key="SecondaryTextBrush" Color="#99FFFFFF" />
    <SolidColorBrush x:Key="TertiaryTextBrush" Color="#66FFFFFF" />

    <!-- Sidebar item style -->
    <Style x:Key="SidebarItemStyle" TargetType="Button">
        <Setter Property="Background" Value="Transparent" />
        <Setter Property="BorderBrush" Value="Transparent" />
        <Setter Property="BorderThickness" Value="0" />
        <Setter Property="Padding" Value="10,8" />
        <Setter Property="HorizontalAlignment" Value="Stretch" />
        <Setter Property="HorizontalContentAlignment" Value="Left" />
    </Style>

    <!-- Monospace font -->
    <FontFamily x:Key="MonoFont">Cascadia Code, Consolas, Courier New</FontFamily>

    <!-- Section label style -->
    <Style x:Key="SectionLabelStyle" TargetType="TextBlock">
        <Setter Property="FontSize" Value="10" />
        <Setter Property="FontWeight" Value="Bold" />
        <Setter Property="Foreground" Value="{StaticResource TertiaryTextBrush}" />
        <Setter Property="CharacterSpacing" Value="150" />
        <Setter Property="Margin" Value="18,10,0,2" />
    </Style>

    <!-- Badge style -->
    <Style x:Key="BadgeStyle" TargetType="Border">
        <Setter Property="CornerRadius" Value="8" />
        <Setter Property="Padding" Value="6,2" />
    </Style>
</ResourceDictionary>
```

- [ ] **Step 2: Create Converters**

Create `Configonaut/Converters/Converters.cs`:

```csharp
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Data;
using Microsoft.UI.Xaml.Media;
using Windows.UI;

namespace Configonaut.Converters;

public class BoolToVisibilityConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        var boolValue = (bool)value;
        if (parameter is string p && p == "invert")
            boolValue = !boolValue;
        return boolValue ? Visibility.Visible : Visibility.Collapsed;
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language) =>
        (Visibility)value == Visibility.Visible;
}

public class CountToVisibilityConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        var count = System.Convert.ToInt32(value);
        var invert = parameter is string p && p == "zero";
        return (invert ? count == 0 : count > 0) ? Visibility.Visible : Visibility.Collapsed;
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language) =>
        throw new NotImplementedException();
}

public class EventToColorConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        var eventName = value as string ?? "";
        return eventName switch
        {
            "PreToolUse" => new SolidColorBrush(ColorFromHex("#00B4FF")),
            "PostToolUse" => new SolidColorBrush(ColorFromHex("#00F5A0")),
            "Notification" => new SolidColorBrush(ColorFromHex("#FF8A3D")),
            "Stop" => new SolidColorBrush(ColorFromHex("#FF5C8A")),
            "SubagentStop" => new SolidColorBrush(ColorFromHex("#B36CFF")),
            _ => new SolidColorBrush(ColorFromHex("#99FFFFFF"))
        };
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language) =>
        throw new NotImplementedException();

    private static Color ColorFromHex(string hex)
    {
        hex = hex.TrimStart('#');
        byte a = 0xFF, r, g, b;
        if (hex.Length == 8)
        {
            a = System.Convert.ToByte(hex[..2], 16);
            r = System.Convert.ToByte(hex[2..4], 16);
            g = System.Convert.ToByte(hex[4..6], 16);
            b = System.Convert.ToByte(hex[6..8], 16);
        }
        else
        {
            r = System.Convert.ToByte(hex[..2], 16);
            g = System.Convert.ToByte(hex[2..4], 16);
            b = System.Convert.ToByte(hex[4..6], 16);
        }
        return Color.FromArgb(a, r, g, b);
    }
}

public class BoolToOpacityConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language) =>
        (bool)value ? 1.0 : 0.5;

    public object ConvertBack(object value, Type targetType, object parameter, string language) =>
        throw new NotImplementedException();
}

public class ServerSourceToBadgeConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language) =>
        value is Models.ServerSource source
            ? source == Models.ServerSource.Active ? "Active" : "Inactive"
            : "Unknown";

    public object ConvertBack(object value, Type targetType, object parameter, string language) =>
        throw new NotImplementedException();
}
```

- [ ] **Step 3: Verify it compiles**

```bash
dotnet build --configuration Debug
```

Expected: Build succeeds.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: add NeonTheme resource dictionary and value converters"
```

---

### Task 8: MainWindow Shell

**Files:**
- Create: `Configonaut/MainWindow.xaml`
- Create: `Configonaut/MainWindow.xaml.cs`

- [ ] **Step 1: Create MainWindow.xaml**

```xml
<Window
    x:Class="Configonaut.MainWindow"
    xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
    xmlns:local="using:Configonaut"
    xmlns:views="using:Configonaut.Views"
    xmlns:ctc="using:CommunityToolkit.WinUI.Controls"
    Title="Configonaut"
    MinWidth="920" MinHeight="660">

    <Grid Background="{StaticResource BgPrimaryBrush}">
        <Grid.ColumnDefinitions>
            <ColumnDefinition Width="220" />
            <ColumnDefinition Width="1" />
            <ColumnDefinition Width="*" />
        </Grid.ColumnDefinitions>

        <!-- Sidebar -->
        <Grid Grid.Column="0" Background="{ThemeResource AcrylicBackgroundFillColorDefaultBrush}">
            <ScrollViewer VerticalScrollBarVisibility="Auto">
                <StackPanel Spacing="2" Padding="0,0,0,8">

                    <!-- App branding -->
                    <StackPanel Orientation="Horizontal" Spacing="10"
                                Padding="14,14,14,18">
                        <Grid Width="38" Height="38">
                            <Ellipse Width="48" Height="48" Opacity="0.3"
                                     Fill="{StaticResource NeonGreenBrush}" />
                            <Image Source="Assets/AppIcon.png" Width="38" Height="38"
                                   Stretch="UniformToFill">
                                <Image.Clip>
                                    <RectangleGeometry Rect="0,0,38,38" RadiusX="9" RadiusY="9" />
                                </Image.Clip>
                            </Image>
                        </Grid>
                        <StackPanel VerticalAlignment="Center" Spacing="1">
                            <TextBlock Text="Configonaut"
                                       FontSize="15" FontWeight="Bold"
                                       Foreground="{StaticResource PrimaryTextBrush}" />
                            <TextBlock x:Name="ModeSubtitle"
                                       FontSize="10" FontWeight="Medium"
                                       Foreground="{StaticResource NeonGreenBrush}"
                                       Opacity="0.5" />
                        </StackPanel>
                    </StackPanel>

                    <!-- Desktop / CLI toggle -->
                    <Grid Padding="8,0,8,12" HorizontalAlignment="Center">
                        <ctc:Segmented x:Name="ModeToggle"
                                       SelectionMode="Single"
                                       SelectionChanged="ModeToggle_SelectionChanged">
                            <ctc:SegmentedItem Content="Desktop" Tag="Desktop" />
                            <ctc:SegmentedItem Content="CLI" Tag="CLI" />
                        </ctc:Segmented>
                    </Grid>

                    <!-- TOOLS section -->
                    <TextBlock Text="TOOLS" Style="{StaticResource SectionLabelStyle}" />

                    <Button x:Name="BtnServers" Style="{StaticResource SidebarItemStyle}"
                            Click="SidebarItem_Click" Tag="Servers">
                        <Grid>
                            <Grid.ColumnDefinitions>
                                <ColumnDefinition Width="Auto" />
                                <ColumnDefinition Width="*" />
                                <ColumnDefinition Width="Auto" />
                            </Grid.ColumnDefinitions>
                            <FontIcon Grid.Column="0" Glyph="&#xE83B;" FontSize="14"
                                      Foreground="{StaticResource NeonGreenBrush}"
                                      Margin="0,0,10,0" />
                            <StackPanel Grid.Column="1">
                                <TextBlock Text="MCP Servers" FontSize="13" FontWeight="SemiBold"
                                           Foreground="{StaticResource PrimaryTextBrush}" />
                                <TextBlock Text="Add, remove &amp; swap tools" FontSize="10"
                                           Foreground="{StaticResource TertiaryTextBrush}" />
                            </StackPanel>
                            <Border Grid.Column="2" x:Name="BadgeServers"
                                    Style="{StaticResource BadgeStyle}"
                                    Background="{StaticResource NeonGreenBrush}" Opacity="0.7">
                                <TextBlock x:Name="BadgeServersCount" FontSize="9"
                                           FontWeight="Bold" Foreground="White" />
                            </Border>
                        </Grid>
                    </Button>

                    <Button x:Name="BtnHooks" Style="{StaticResource SidebarItemStyle}"
                            Click="SidebarItem_Click" Tag="Hooks">
                        <Grid>
                            <Grid.ColumnDefinitions>
                                <ColumnDefinition Width="Auto" />
                                <ColumnDefinition Width="*" />
                                <ColumnDefinition Width="Auto" />
                            </Grid.ColumnDefinitions>
                            <FontIcon Grid.Column="0" Glyph="&#xE8D8;" FontSize="14"
                                      Foreground="{StaticResource NeonBlueBrush}"
                                      Margin="0,0,10,0" />
                            <StackPanel Grid.Column="1">
                                <TextBlock Text="Hooks" FontSize="13" FontWeight="SemiBold"
                                           Foreground="{StaticResource PrimaryTextBrush}" />
                                <TextBlock Text="Automation triggers" FontSize="10"
                                           Foreground="{StaticResource TertiaryTextBrush}" />
                            </StackPanel>
                            <Border Grid.Column="2" x:Name="BadgeHooks"
                                    Style="{StaticResource BadgeStyle}"
                                    Background="{StaticResource NeonBlueBrush}" Opacity="0.4">
                                <TextBlock x:Name="BadgeHooksCount" FontSize="9"
                                           FontWeight="Bold" Foreground="White" />
                            </Border>
                        </Grid>
                    </Button>

                    <!-- EXTEND section -->
                    <TextBlock Text="EXTEND" Style="{StaticResource SectionLabelStyle}"
                               Margin="18,10,0,2" />

                    <Button x:Name="BtnAgents" Style="{StaticResource SidebarItemStyle}"
                            Click="SidebarItem_Click" Tag="Agents">
                        <Grid>
                            <Grid.ColumnDefinitions>
                                <ColumnDefinition Width="Auto" />
                                <ColumnDefinition Width="*" />
                                <ColumnDefinition Width="Auto" />
                            </Grid.ColumnDefinitions>
                            <FontIcon Grid.Column="0" Glyph="&#xE716;" FontSize="14"
                                      Foreground="{StaticResource NeonPurpleBrush}"
                                      Margin="0,0,10,0" />
                            <StackPanel Grid.Column="1">
                                <TextBlock Text="Agents" FontSize="13" FontWeight="SemiBold"
                                           Foreground="{StaticResource PrimaryTextBrush}" />
                                <TextBlock Text="Plugin agent configs" FontSize="10"
                                           Foreground="{StaticResource TertiaryTextBrush}" />
                            </StackPanel>
                            <Border Grid.Column="2" x:Name="BadgeAgents"
                                    Style="{StaticResource BadgeStyle}"
                                    Background="{StaticResource NeonPurpleBrush}" Opacity="0.4">
                                <TextBlock x:Name="BadgeAgentsCount" FontSize="9"
                                           FontWeight="Bold" Foreground="White" />
                            </Border>
                        </Grid>
                    </Button>

                    <Button x:Name="BtnSkills" Style="{StaticResource SidebarItemStyle}"
                            Click="SidebarItem_Click" Tag="Skills">
                        <Grid>
                            <Grid.ColumnDefinitions>
                                <ColumnDefinition Width="Auto" />
                                <ColumnDefinition Width="*" />
                                <ColumnDefinition Width="Auto" />
                            </Grid.ColumnDefinitions>
                            <FontIcon Grid.Column="0" Glyph="&#xE734;" FontSize="14"
                                      Foreground="{StaticResource NeonAmberBrush}"
                                      Margin="0,0,10,0" />
                            <StackPanel Grid.Column="1">
                                <TextBlock Text="Skills" FontSize="13" FontWeight="SemiBold"
                                           Foreground="{StaticResource PrimaryTextBrush}" />
                                <TextBlock Text="Commands &amp; slash skills" FontSize="10"
                                           Foreground="{StaticResource TertiaryTextBrush}" />
                            </StackPanel>
                            <Border Grid.Column="2" x:Name="BadgeSkills"
                                    Style="{StaticResource BadgeStyle}"
                                    Background="{StaticResource NeonAmberBrush}" Opacity="0.4">
                                <TextBlock x:Name="BadgeSkillsCount" FontSize="9"
                                           FontWeight="Bold" Foreground="White" />
                            </Border>
                        </Grid>
                    </Button>

                    <!-- Spacer -->
                    <Grid Height="1" Margin="14,0" Opacity="0.15"
                          Background="{StaticResource NeonCyanBrush}" />

                    <Button x:Name="BtnBackups" Style="{StaticResource SidebarItemStyle}"
                            Click="SidebarItem_Click" Tag="Backups">
                        <Grid>
                            <Grid.ColumnDefinitions>
                                <ColumnDefinition Width="Auto" />
                                <ColumnDefinition Width="*" />
                                <ColumnDefinition Width="Auto" />
                            </Grid.ColumnDefinitions>
                            <FontIcon Grid.Column="0" Glyph="&#xE81C;" FontSize="14"
                                      Foreground="{StaticResource NeonCyanBrush}"
                                      Margin="0,0,10,0" />
                            <StackPanel Grid.Column="1">
                                <TextBlock Text="Backups" FontSize="13" FontWeight="SemiBold"
                                           Foreground="{StaticResource PrimaryTextBrush}" />
                                <TextBlock Text="Config history &amp; restore" FontSize="10"
                                           Foreground="{StaticResource TertiaryTextBrush}" />
                            </StackPanel>
                            <Border Grid.Column="2" x:Name="BadgeBackups"
                                    Style="{StaticResource BadgeStyle}"
                                    Background="{StaticResource NeonCyanBrush}" Opacity="0.4">
                                <TextBlock x:Name="BadgeBackupsCount" FontSize="9"
                                           FontWeight="Bold" Foreground="White" />
                            </Border>
                        </Grid>
                    </Button>

                    <!-- Version footer -->
                    <TextBlock Text="v1.0.0" FontSize="9" FontWeight="Medium"
                               FontFamily="{StaticResource MonoFont}"
                               Foreground="{StaticResource TertiaryTextBrush}"
                               HorizontalAlignment="Center" Opacity="0.5"
                               Margin="0,8,0,0" />
                </StackPanel>
            </ScrollViewer>
        </Grid>

        <!-- Separator -->
        <Rectangle Grid.Column="1" Fill="{StaticResource SubtleBorderBrush}" />

        <!-- Content area -->
        <Frame x:Name="ContentFrame" Grid.Column="2" />
    </Grid>
</Window>
```

- [ ] **Step 2: Create MainWindow.xaml.cs**

```csharp
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Configonaut.Models;
using Configonaut.Services;
using Configonaut.Views;
using CommunityToolkit.WinUI.Controls;
using Windows.UI;

namespace Configonaut;

public sealed partial class MainWindow : Window
{
    public ConfigManager Config { get; }
    private string _currentSection = "Servers";

    public MainWindow()
    {
        this.InitializeComponent();

        // Set dark theme
        if (Content is FrameworkElement root)
            root.RequestedTheme = ElementTheme.Dark;

        // Set minimum size
        var appWindow = this.AppWindow;
        appWindow.Resize(new Windows.Graphics.SizeInt32(960, 700));

        Config = new ConfigManager();

        // Set initial mode toggle state
        ModeToggle.SelectedIndex = Config.Mode == AppMode.Desktop ? 0 : 1;
        UpdateModeSubtitle();
        UpdateBadges();

        // Subscribe to property changes for badge updates
        Config.PropertyChanged += (s, e) => DispatcherQueue.TryEnqueue(UpdateBadges);

        // Navigate to default page
        NavigateToSection("Servers");
        UpdateSidebarSelection();
    }

    private void ModeToggle_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (ModeToggle.SelectedItem is SegmentedItem item && item.Tag is string tag)
        {
            Config.Mode = tag == "CLI" ? AppMode.Cli : AppMode.Desktop;
            UpdateModeSubtitle();
            UpdateBadges();
            // Re-navigate to refresh the current page
            NavigateToSection(_currentSection);
        }
    }

    private void SidebarItem_Click(object sender, RoutedEventArgs e)
    {
        if (sender is Button btn && btn.Tag is string section)
        {
            _currentSection = section;
            NavigateToSection(section);
            UpdateSidebarSelection();
        }
    }

    private void NavigateToSection(string section)
    {
        switch (section)
        {
            case "Servers":
                ContentFrame.Navigate(typeof(MCPPage), Config);
                break;
            case "Hooks":
                ContentFrame.Navigate(typeof(HooksPage), Config);
                break;
            case "Agents":
                ContentFrame.Navigate(typeof(AgentsPage), Config);
                break;
            case "Skills":
                ContentFrame.Navigate(typeof(SkillsPage), Config);
                break;
            case "Backups":
                ContentFrame.Navigate(typeof(BackupsPage), Config);
                break;
        }
    }

    private void UpdateModeSubtitle()
    {
        ModeSubtitle.Text = Config.Mode == AppMode.Desktop ? "Desktop Config" : "CLI Config";
        ModeSubtitle.Foreground = Config.Mode == AppMode.Desktop
            ? (SolidColorBrush)Application.Current.Resources["NeonGreenBrush"]
            : (SolidColorBrush)Application.Current.Resources["NeonBlueBrush"];
    }

    private void UpdateBadges()
    {
        SetBadge(BadgeServers, BadgeServersCount, Config.ActiveServers.Count);
        SetBadge(BadgeHooks, BadgeHooksCount, Config.HookRules.Count);
        SetBadge(BadgeAgents, BadgeAgentsCount, Config.Agents.Count);
        SetBadge(BadgeSkills, BadgeSkillsCount, Config.Skills.Count);
        SetBadge(BadgeBackups, BadgeBackupsCount, Config.BackupFiles.Count);
    }

    private static void SetBadge(Border border, TextBlock text, int count)
    {
        border.Visibility = count > 0 ? Visibility.Visible : Visibility.Collapsed;
        text.Text = count.ToString();
    }

    private void UpdateSidebarSelection()
    {
        var buttons = new[] { BtnServers, BtnHooks, BtnAgents, BtnSkills, BtnBackups };
        var sections = new[] { "Servers", "Hooks", "Agents", "Skills", "Backups" };

        for (int i = 0; i < buttons.Length; i++)
        {
            var isSelected = sections[i] == _currentSection;
            buttons[i].Background = isSelected
                ? new SolidColorBrush(Color.FromArgb(0x14, 0xFF, 0xFF, 0xFF))
                : new SolidColorBrush(Colors.Transparent);
        }
    }
}
```

- [ ] **Step 3: Verify it compiles**

```bash
dotnet build --configuration Debug
```

Expected: Build succeeds (pages don't exist yet but Frame navigation is lazy).

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: add MainWindow shell with sidebar navigation and mode toggle"
```

---

### Task 9: MCPPage

**Files:**
- Create: `Configonaut/Views/MCPPage.xaml`
- Create: `Configonaut/Views/MCPPage.xaml.cs`

This is the most complex view — two-column server list with drag-drop, JSON editor, and add-server panel.

- [ ] **Step 1: Create MCPPage.xaml**

```xml
<Page
    x:Class="Configonaut.Views.MCPPage"
    xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
    xmlns:local="using:Configonaut.Views"
    Background="Transparent">

    <Grid Padding="24">
        <Grid.RowDefinitions>
            <RowDefinition Height="Auto" />
            <RowDefinition Height="Auto" />
            <RowDefinition Height="*" />
            <RowDefinition Height="Auto" />
            <RowDefinition Height="Auto" />
        </Grid.RowDefinitions>

        <!-- Restart banner -->
        <Border x:Name="RestartBanner" Grid.Row="0" Visibility="Collapsed"
                CornerRadius="10" Padding="16,12" Margin="0,0,0,16">
            <Border.Background>
                <LinearGradientBrush StartPoint="0,0" EndPoint="1,0">
                    <GradientStop Color="{StaticResource NeonBlue}" Offset="0" />
                    <GradientStop Color="{StaticResource NeonPurple}" Offset="1" />
                </LinearGradientBrush>
            </Border.Background>
            <Grid>
                <Grid.ColumnDefinitions>
                    <ColumnDefinition Width="*" />
                    <ColumnDefinition Width="Auto" />
                </Grid.ColumnDefinitions>
                <TextBlock x:Name="RestartText" Grid.Column="0"
                           VerticalAlignment="Center" FontWeight="SemiBold"
                           Foreground="White" />
                <Button Grid.Column="1" Content="Got it" Click="DismissRestart_Click"
                        Background="Transparent" Foreground="White" BorderBrush="White"
                        BorderThickness="1" CornerRadius="6" Padding="12,4" />
            </Grid>
        </Border>

        <!-- Header -->
        <Grid Grid.Row="1" Margin="0,0,0,16">
            <Grid.ColumnDefinitions>
                <ColumnDefinition Width="*" />
                <ColumnDefinition Width="Auto" />
            </Grid.ColumnDefinitions>
            <StackPanel Grid.Column="0">
                <TextBlock Text="MCP Servers" FontSize="22" FontWeight="Bold"
                           Foreground="{StaticResource PrimaryTextBrush}" />
                <TextBlock x:Name="ConfigPathText" FontSize="11"
                           FontFamily="{StaticResource MonoFont}"
                           Foreground="{StaticResource TertiaryTextBrush}" />
            </StackPanel>
            <Button Grid.Column="1" Content="＋ Add Server" Click="AddServer_Click"
                    VerticalAlignment="Top"
                    Background="{StaticResource NeonGreenBrush}" Foreground="Black"
                    FontWeight="SemiBold" CornerRadius="8" Padding="16,8" />
        </Grid>

        <!-- Two-column server lists -->
        <Grid Grid.Row="2">
            <Grid.ColumnDefinitions>
                <ColumnDefinition Width="*" />
                <ColumnDefinition Width="16" />
                <ColumnDefinition Width="*" />
            </Grid.ColumnDefinitions>

            <!-- Active servers column -->
            <Border Grid.Column="0" CornerRadius="12" Padding="1"
                    Background="{StaticResource CardFillBrush}"
                    BorderBrush="{StaticResource CardBorderBrush}" BorderThickness="1"
                    AllowDrop="True" DragOver="ActiveColumn_DragOver" Drop="ActiveColumn_Drop">
                <Grid>
                    <Grid.RowDefinitions>
                        <RowDefinition Height="Auto" />
                        <RowDefinition Height="*" />
                    </Grid.RowDefinitions>
                    <StackPanel Grid.Row="0" Orientation="Horizontal" Spacing="8"
                                Padding="16,12">
                        <Ellipse Width="8" Height="8" Fill="{StaticResource NeonGreenBrush}" />
                        <TextBlock Text="Active" FontSize="13" FontWeight="SemiBold"
                                   Foreground="{StaticResource PrimaryTextBrush}" />
                        <TextBlock x:Name="ActiveCount" FontSize="11"
                                   Foreground="{StaticResource TertiaryTextBrush}" />
                    </StackPanel>
                    <ListView Grid.Row="1" x:Name="ActiveList"
                              SelectionMode="Single" SelectionChanged="ActiveList_SelectionChanged"
                              CanDragItems="True" DragItemsStarting="ActiveList_DragStarting"
                              Background="Transparent">
                        <ListView.ItemTemplate>
                            <DataTemplate>
                                <Grid Padding="12,8">
                                    <TextBlock Text="{Binding Name}"
                                               FontFamily="{StaticResource MonoFont}"
                                               FontSize="13"
                                               Foreground="{StaticResource PrimaryTextBrush}" />
                                </Grid>
                            </DataTemplate>
                        </ListView.ItemTemplate>
                    </ListView>
                </Grid>
            </Border>

            <!-- Stored servers column -->
            <Border Grid.Column="2" CornerRadius="12" Padding="1"
                    Background="{StaticResource CardFillBrush}"
                    BorderBrush="{StaticResource CardBorderBrush}" BorderThickness="1"
                    AllowDrop="True" DragOver="StoredColumn_DragOver" Drop="StoredColumn_Drop">
                <Grid>
                    <Grid.RowDefinitions>
                        <RowDefinition Height="Auto" />
                        <RowDefinition Height="*" />
                    </Grid.RowDefinitions>
                    <StackPanel Grid.Row="0" Orientation="Horizontal" Spacing="8"
                                Padding="16,12">
                        <Ellipse Width="8" Height="8" Fill="{StaticResource NeonRedBrush}" />
                        <TextBlock Text="Inactive" FontSize="13" FontWeight="SemiBold"
                                   Foreground="{StaticResource PrimaryTextBrush}" />
                        <TextBlock x:Name="StoredCount" FontSize="11"
                                   Foreground="{StaticResource TertiaryTextBrush}" />
                    </StackPanel>
                    <ListView Grid.Row="1" x:Name="StoredList"
                              SelectionMode="Single" SelectionChanged="StoredList_SelectionChanged"
                              CanDragItems="True" DragItemsStarting="StoredList_DragStarting"
                              Background="Transparent">
                        <ListView.ItemTemplate>
                            <DataTemplate>
                                <Grid Padding="12,8">
                                    <TextBlock Text="{Binding Name}"
                                               FontFamily="{StaticResource MonoFont}"
                                               FontSize="13"
                                               Foreground="{StaticResource PrimaryTextBrush}" />
                                </Grid>
                            </DataTemplate>
                        </ListView.ItemTemplate>
                    </ListView>
                </Grid>
            </Border>
        </Grid>

        <!-- Detail / Add panel -->
        <Border Grid.Row="3" x:Name="DetailPanel" Visibility="Collapsed"
                CornerRadius="12" Margin="0,16,0,0" Padding="16"
                Background="{StaticResource CardFillBrush}"
                BorderBrush="{StaticResource CardBorderBrush}" BorderThickness="1"
                MinHeight="200" MaxHeight="400">
            <Grid>
                <Grid.RowDefinitions>
                    <RowDefinition Height="Auto" />
                    <RowDefinition Height="*" />
                    <RowDefinition Height="Auto" />
                </Grid.RowDefinitions>

                <!-- Detail header -->
                <Grid Grid.Row="0" x:Name="DetailHeader" Visibility="Collapsed" Margin="0,0,0,8">
                    <Grid.ColumnDefinitions>
                        <ColumnDefinition Width="*" />
                        <ColumnDefinition Width="Auto" />
                    </Grid.ColumnDefinitions>
                    <StackPanel Orientation="Horizontal" Spacing="8">
                        <TextBlock x:Name="DetailServerName" FontSize="15" FontWeight="Bold"
                                   FontFamily="{StaticResource MonoFont}"
                                   Foreground="{StaticResource PrimaryTextBrush}" />
                        <Border x:Name="DetailSourceBadge" CornerRadius="4" Padding="6,2">
                            <TextBlock x:Name="DetailSourceText" FontSize="10" FontWeight="Bold"
                                       Foreground="White" />
                        </Border>
                    </StackPanel>
                    <Button Grid.Column="1" Content="✕" Click="CloseDetail_Click"
                            Background="Transparent" Foreground="{StaticResource TertiaryTextBrush}"
                            FontSize="16" Padding="4" />
                </Grid>

                <!-- Add header -->
                <Grid Grid.Row="0" x:Name="AddHeader" Visibility="Collapsed" Margin="0,0,0,8">
                    <Grid.ColumnDefinitions>
                        <ColumnDefinition Width="*" />
                        <ColumnDefinition Width="Auto" />
                    </Grid.ColumnDefinitions>
                    <TextBlock Text="Add Server" FontSize="15" FontWeight="Bold"
                               Foreground="{StaticResource PrimaryTextBrush}" />
                    <StackPanel Grid.Column="1" Orientation="Horizontal" Spacing="8">
                        <Border x:Name="ValidationBadge" CornerRadius="6" Padding="8,4">
                            <TextBlock x:Name="ValidationText" FontSize="11" FontWeight="SemiBold"
                                       Foreground="White" />
                        </Border>
                        <Button Content="✕" Click="CloseAdd_Click"
                                Background="Transparent"
                                Foreground="{StaticResource TertiaryTextBrush}"
                                FontSize="16" Padding="4" />
                    </StackPanel>
                </Grid>

                <!-- JSON editor -->
                <TextBox Grid.Row="1" x:Name="JsonEditor"
                         AcceptsReturn="True" TextWrapping="Wrap"
                         FontFamily="{StaticResource MonoFont}" FontSize="12"
                         Background="{StaticResource BgPrimaryBrush}"
                         Foreground="{StaticResource PrimaryTextBrush}"
                         BorderThickness="1" CornerRadius="8"
                         BorderBrush="{StaticResource CardBorderBrush}"
                         PlaceholderText="Paste JSON server configuration..."
                         TextChanged="JsonEditor_TextChanged" />

                <!-- Name field (for bare configs) -->
                <StackPanel Grid.Row="1" x:Name="NameFieldPanel" Visibility="Collapsed"
                            Margin="0,0,0,8" Orientation="Horizontal" Spacing="8">
                    <TextBlock Text="Server name:" FontSize="13" VerticalAlignment="Center"
                               Foreground="{StaticResource SecondaryTextBrush}" />
                    <TextBox x:Name="ServerNameField" Width="200"
                             FontFamily="{StaticResource MonoFont}"
                             PlaceholderText="my-server" />
                </StackPanel>

                <!-- Action buttons -->
                <StackPanel Grid.Row="2" Orientation="Horizontal" Spacing="8"
                            HorizontalAlignment="Right" Margin="0,8,0,0">
                    <!-- Detail mode buttons -->
                    <Button x:Name="BtnSave" Content="Save" Click="SaveServer_Click"
                            Visibility="Collapsed"
                            Background="{StaticResource NeonGreenBrush}" Foreground="Black"
                            FontWeight="SemiBold" CornerRadius="6" Padding="12,6" />
                    <Button x:Name="BtnToggle" Click="ToggleServer_Click"
                            Visibility="Collapsed"
                            CornerRadius="6" Padding="12,6" FontWeight="SemiBold" />
                    <Button x:Name="BtnCopy" Content="Copy JSON" Click="CopyJson_Click"
                            Visibility="Collapsed"
                            Background="{StaticResource CardFillBrush}"
                            Foreground="{StaticResource PrimaryTextBrush}"
                            CornerRadius="6" Padding="12,6" />
                    <Button x:Name="BtnDelete" Content="Delete" Click="DeleteServer_Click"
                            Visibility="Collapsed"
                            Background="{StaticResource NeonRedBrush}" Foreground="White"
                            FontWeight="SemiBold" CornerRadius="6" Padding="12,6" />

                    <!-- Add mode buttons -->
                    <Button x:Name="BtnCancel" Content="Cancel" Click="CloseAdd_Click"
                            Visibility="Collapsed"
                            Background="{StaticResource CardFillBrush}"
                            Foreground="{StaticResource PrimaryTextBrush}"
                            CornerRadius="6" Padding="12,6" />
                    <Button x:Name="BtnSaveOnly" Content="Save Only" Click="SaveOnly_Click"
                            Visibility="Collapsed"
                            Background="{StaticResource CardFillBrush}"
                            Foreground="{StaticResource PrimaryTextBrush}"
                            FontWeight="SemiBold" CornerRadius="6" Padding="12,6" />
                    <Button x:Name="BtnTurnOn" Content="Turn On" Click="TurnOn_Click"
                            Visibility="Collapsed"
                            Background="{StaticResource NeonGreenBrush}" Foreground="Black"
                            FontWeight="SemiBold" CornerRadius="6" Padding="12,6" />
                </StackPanel>
            </Grid>
        </Border>

        <!-- Footer -->
        <Grid Grid.Row="4" Margin="0,12,0,0">
            <Grid.ColumnDefinitions>
                <ColumnDefinition Width="*" />
                <ColumnDefinition Width="Auto" />
            </Grid.ColumnDefinitions>
            <StackPanel Grid.Column="0" Orientation="Horizontal" Spacing="6">
                <Ellipse x:Name="StatusDot" Width="6" Height="6"
                         Fill="{StaticResource NeonGreenBrush}" />
                <TextBlock x:Name="StatusText" FontSize="11"
                           Foreground="{StaticResource TertiaryTextBrush}" />
            </StackPanel>
            <TextBlock Grid.Column="1" x:Name="CountSummary" FontSize="11"
                       Foreground="{StaticResource TertiaryTextBrush}" />
        </Grid>
    </Grid>
</Page>
```

- [ ] **Step 2: Create MCPPage.xaml.cs**

```csharp
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Navigation;
using Windows.ApplicationModel.DataTransfer;
using Configonaut.Models;
using Configonaut.Services;

namespace Configonaut.Views;

public sealed partial class MCPPage : Page
{
    private ConfigManager _config = null!;
    private ServerEntry? _selectedServer;
    private ServerSource _selectedSource;
    private bool _isAddMode;
    private bool _needsName;

    public MCPPage()
    {
        this.InitializeComponent();
    }

    protected override void OnNavigatedTo(NavigationEventArgs e)
    {
        if (e.Parameter is ConfigManager config)
        {
            _config = config;
            _config.PropertyChanged += Config_PropertyChanged;
            RefreshUI();
        }
    }

    protected override void OnNavigatedFrom(NavigationEventArgs e)
    {
        if (_config is not null)
            _config.PropertyChanged -= Config_PropertyChanged;
    }

    private void Config_PropertyChanged(object? sender, System.ComponentModel.PropertyChangedEventArgs e)
    {
        DispatcherQueue.TryEnqueue(RefreshUI);
    }

    private void RefreshUI()
    {
        ActiveList.ItemsSource = _config.ActiveServers;
        StoredList.ItemsSource = _config.StoredServers;
        ActiveCount.Text = $"({_config.ActiveServers.Count})";
        StoredCount.Text = $"({_config.StoredServers.Count})";
        CountSummary.Text = $"{_config.ActiveServers.Count} active, {_config.StoredServers.Count} inactive";
        StatusText.Text = _config.StatusMessage;
        StatusDot.Fill = _config.StatusIsError
            ? (Microsoft.UI.Xaml.Media.SolidColorBrush)Application.Current.Resources["NeonRedBrush"]
            : (Microsoft.UI.Xaml.Media.SolidColorBrush)Application.Current.Resources["NeonGreenBrush"];
        ConfigPathText.Text = PathResolver.ShortenPath(_config.ConfigPath);

        RestartBanner.Visibility = _config.NeedsRestart ? Visibility.Visible : Visibility.Collapsed;
        RestartText.Text = $"Changes saved! Quit and reopen {(_config.Mode == AppMode.Desktop ? "Claude Desktop" : "Claude Code")} to apply.";
    }

    // --- Sidebar selection ---

    private void ActiveList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (ActiveList.SelectedItem is ServerEntry server)
        {
            StoredList.SelectedItem = null;
            ShowDetail(server, ServerSource.Active);
        }
    }

    private void StoredList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (StoredList.SelectedItem is ServerEntry server)
        {
            ActiveList.SelectedItem = null;
            ShowDetail(server, ServerSource.Stored);
        }
    }

    private void ShowDetail(ServerEntry server, ServerSource source)
    {
        _selectedServer = server;
        _selectedSource = source;
        _isAddMode = false;

        DetailPanel.Visibility = Visibility.Visible;
        DetailHeader.Visibility = Visibility.Visible;
        AddHeader.Visibility = Visibility.Collapsed;
        NameFieldPanel.Visibility = Visibility.Collapsed;

        DetailServerName.Text = server.Name;
        DetailSourceText.Text = source == ServerSource.Active ? "Active" : "Inactive";
        DetailSourceBadge.Background = source == ServerSource.Active
            ? (Microsoft.UI.Xaml.Media.SolidColorBrush)Application.Current.Resources["NeonGreenBrush"]
            : (Microsoft.UI.Xaml.Media.SolidColorBrush)Application.Current.Resources["NeonRedBrush"];

        JsonEditor.Text = server.ConfigJson;

        // Show detail buttons, hide add buttons
        BtnSave.Visibility = Visibility.Visible;
        BtnToggle.Visibility = Visibility.Visible;
        BtnCopy.Visibility = Visibility.Visible;
        BtnDelete.Visibility = Visibility.Visible;
        BtnCancel.Visibility = Visibility.Collapsed;
        BtnSaveOnly.Visibility = Visibility.Collapsed;
        BtnTurnOn.Visibility = Visibility.Collapsed;

        BtnToggle.Content = source == ServerSource.Active ? "Turn Off" : "Turn On";
        BtnToggle.Background = source == ServerSource.Active
            ? (Microsoft.UI.Xaml.Media.SolidColorBrush)Application.Current.Resources["NeonRedBrush"]
            : (Microsoft.UI.Xaml.Media.SolidColorBrush)Application.Current.Resources["NeonGreenBrush"];
        BtnToggle.Foreground = source == ServerSource.Active
            ? new Microsoft.UI.Xaml.Media.SolidColorBrush(Microsoft.UI.Colors.White)
            : new Microsoft.UI.Xaml.Media.SolidColorBrush(Microsoft.UI.Colors.Black);
    }

    // --- Add server ---

    private void AddServer_Click(object sender, RoutedEventArgs e)
    {
        _isAddMode = true;
        _selectedServer = null;
        ActiveList.SelectedItem = null;
        StoredList.SelectedItem = null;

        DetailPanel.Visibility = Visibility.Visible;
        DetailHeader.Visibility = Visibility.Collapsed;
        AddHeader.Visibility = Visibility.Visible;
        NameFieldPanel.Visibility = Visibility.Collapsed;

        JsonEditor.Text = "";
        ValidationBadge.Visibility = Visibility.Collapsed;

        // Show add buttons, hide detail buttons
        BtnSave.Visibility = Visibility.Collapsed;
        BtnToggle.Visibility = Visibility.Collapsed;
        BtnCopy.Visibility = Visibility.Collapsed;
        BtnDelete.Visibility = Visibility.Collapsed;
        BtnCancel.Visibility = Visibility.Visible;
        BtnSaveOnly.Visibility = Visibility.Visible;
        BtnTurnOn.Visibility = Visibility.Visible;
        BtnSaveOnly.IsEnabled = false;
        BtnTurnOn.IsEnabled = false;
    }

    private void JsonEditor_TextChanged(object sender, TextChangedEventArgs e)
    {
        if (!_isAddMode) return;

        var text = JsonEditor.Text;
        if (string.IsNullOrWhiteSpace(text))
        {
            ValidationBadge.Visibility = Visibility.Collapsed;
            BtnSaveOnly.IsEnabled = false;
            BtnTurnOn.IsEnabled = false;
            NameFieldPanel.Visibility = Visibility.Collapsed;
            return;
        }

        var result = _config.ParseInput(text);
        ValidationBadge.Visibility = Visibility.Visible;

        switch (result)
        {
            case ParseResult.Servers servers:
                ValidationText.Text = $"Ready to add ({servers.Entries.Count})";
                ValidationBadge.Background = (Microsoft.UI.Xaml.Media.SolidColorBrush)Application.Current.Resources["NeonGreenBrush"];
                BtnSaveOnly.IsEnabled = true;
                BtnTurnOn.IsEnabled = true;
                NameFieldPanel.Visibility = Visibility.Collapsed;
                _needsName = false;
                break;
            case ParseResult.NeedsName:
                ValidationText.Text = "Enter a server name";
                ValidationBadge.Background = (Microsoft.UI.Xaml.Media.SolidColorBrush)Application.Current.Resources["NeonAmberBrush"];
                BtnSaveOnly.IsEnabled = !string.IsNullOrWhiteSpace(ServerNameField.Text);
                BtnTurnOn.IsEnabled = !string.IsNullOrWhiteSpace(ServerNameField.Text);
                NameFieldPanel.Visibility = Visibility.Visible;
                _needsName = true;
                break;
            case ParseResult.Error err:
                ValidationText.Text = err.Message;
                ValidationBadge.Background = (Microsoft.UI.Xaml.Media.SolidColorBrush)Application.Current.Resources["NeonRedBrush"];
                BtnSaveOnly.IsEnabled = false;
                BtnTurnOn.IsEnabled = false;
                NameFieldPanel.Visibility = Visibility.Collapsed;
                _needsName = false;
                break;
        }
    }

    private List<ServerEntry>? GetParsedEntries()
    {
        var result = _config.ParseInput(JsonEditor.Text);
        return result switch
        {
            ParseResult.Servers s => s.Entries,
            ParseResult.NeedsName n when !string.IsNullOrWhiteSpace(ServerNameField.Text) =>
                new List<ServerEntry> { new(ServerNameField.Text.Trim(), n.ConfigJson) },
            _ => null
        };
    }

    private void TurnOn_Click(object sender, RoutedEventArgs e)
    {
        var entries = GetParsedEntries();
        if (entries is null) return;
        _config.AddToActive(entries);
        CloseAddPanel();
    }

    private void SaveOnly_Click(object sender, RoutedEventArgs e)
    {
        var entries = GetParsedEntries();
        if (entries is null) return;
        _config.AddToStored(entries);
        CloseAddPanel();
    }

    // --- Detail actions ---

    private void SaveServer_Click(object sender, RoutedEventArgs e)
    {
        if (_selectedServer is null) return;
        _config.UpdateServerConfig(_selectedServer.Name, _selectedSource, JsonEditor.Text);
    }

    private void ToggleServer_Click(object sender, RoutedEventArgs e)
    {
        if (_selectedServer is null) return;
        if (_selectedSource == ServerSource.Active)
            _config.MoveToStored(_selectedServer.Name);
        else
            _config.MoveToActive(_selectedServer.Name);
        CloseDetailPanel();
    }

    private void CopyJson_Click(object sender, RoutedEventArgs e)
    {
        var dp = new DataPackage();
        dp.SetText(JsonEditor.Text);
        Clipboard.SetContent(dp);
    }

    private async void DeleteServer_Click(object sender, RoutedEventArgs e)
    {
        if (_selectedServer is null) return;

        var dialog = new ContentDialog
        {
            Title = "Delete Server",
            Content = $"Delete '{_selectedServer.Name}'? This cannot be undone.",
            PrimaryButtonText = "Delete",
            CloseButtonText = "Cancel",
            XamlRoot = this.XamlRoot
        };

        if (await dialog.ShowAsync() == ContentDialogResult.Primary)
        {
            _config.DeleteServer(_selectedServer.Name, _selectedSource);
            CloseDetailPanel();
        }
    }

    private void DismissRestart_Click(object sender, RoutedEventArgs e) =>
        _config.NeedsRestart = false;

    private void CloseDetail_Click(object sender, RoutedEventArgs e) => CloseDetailPanel();
    private void CloseAdd_Click(object sender, RoutedEventArgs e) => CloseAddPanel();

    private void CloseDetailPanel()
    {
        _selectedServer = null;
        ActiveList.SelectedItem = null;
        StoredList.SelectedItem = null;
        DetailPanel.Visibility = Visibility.Collapsed;
    }

    private void CloseAddPanel()
    {
        _isAddMode = false;
        JsonEditor.Text = "";
        ServerNameField.Text = "";
        DetailPanel.Visibility = Visibility.Collapsed;
    }

    // --- Drag and drop ---

    private void ActiveList_DragStarting(object sender, DragItemsStartingEventArgs e)
    {
        if (e.Items.FirstOrDefault() is ServerEntry server)
        {
            e.Data.SetText($"active:{server.Name}");
            e.Data.RequestedOperation = DataPackageOperation.Move;
        }
    }

    private void StoredList_DragStarting(object sender, DragItemsStartingEventArgs e)
    {
        if (e.Items.FirstOrDefault() is ServerEntry server)
        {
            e.Data.SetText($"stored:{server.Name}");
            e.Data.RequestedOperation = DataPackageOperation.Move;
        }
    }

    private void ActiveColumn_DragOver(object sender, DragEventArgs e)
    {
        e.AcceptedOperation = DataPackageOperation.Move;
    }

    private async void ActiveColumn_Drop(object sender, DragEventArgs e)
    {
        if (e.DataView.Contains(StandardDataFormats.Text))
        {
            var text = await e.DataView.GetTextAsync();
            if (text.StartsWith("stored:"))
            {
                var name = text["stored:".Length..];
                _config.MoveToActive(name);
            }
        }
    }

    private void StoredColumn_DragOver(object sender, DragEventArgs e)
    {
        e.AcceptedOperation = DataPackageOperation.Move;
    }

    private async void StoredColumn_Drop(object sender, DragEventArgs e)
    {
        if (e.DataView.Contains(StandardDataFormats.Text))
        {
            var text = await e.DataView.GetTextAsync();
            if (text.StartsWith("active:"))
            {
                var name = text["active:".Length..];
                _config.MoveToStored(name);
            }
        }
    }
}
```

- [ ] **Step 3: Verify it compiles**

```bash
dotnet build --configuration Debug
```

Expected: Build succeeds.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: add MCPPage with two-column drag-drop, JSON editor, and add-server panel"
```

---

### Task 10: HooksPage

**Files:**
- Create: `Configonaut/Views/HooksPage.xaml`
- Create: `Configonaut/Views/HooksPage.xaml.cs`

- [ ] **Step 1: Create HooksPage.xaml**

```xml
<Page
    x:Class="Configonaut.Views.HooksPage"
    xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
    Background="Transparent">

    <Grid Padding="24">
        <Grid.RowDefinitions>
            <RowDefinition Height="Auto" />
            <RowDefinition Height="*" />
            <RowDefinition Height="Auto" />
            <RowDefinition Height="Auto" />
        </Grid.RowDefinitions>

        <!-- Header -->
        <StackPanel Grid.Row="0" Margin="0,0,0,16">
            <TextBlock Text="Hooks" FontSize="22" FontWeight="Bold"
                       Foreground="{StaticResource PrimaryTextBrush}" />
            <TextBlock Text="Automation triggers for Claude Code"
                       FontSize="12" Foreground="{StaticResource TertiaryTextBrush}" />
        </StackPanel>

        <!-- Hook list or empty state -->
        <Grid Grid.Row="1">
            <!-- Empty state -->
            <StackPanel x:Name="EmptyState" Visibility="Collapsed"
                        HorizontalAlignment="Center" VerticalAlignment="Center" Spacing="12">
                <FontIcon Glyph="&#xE8D8;" FontSize="48"
                          Foreground="{StaticResource TertiaryTextBrush}"
                          HorizontalAlignment="Center" />
                <TextBlock Text="No hooks defined" FontSize="16" FontWeight="SemiBold"
                           HorizontalAlignment="Center"
                           Foreground="{StaticResource SecondaryTextBrush}" />
                <TextBlock Text="Hooks are defined in ~/.claude/settings.json"
                           FontSize="12" HorizontalAlignment="Center"
                           Foreground="{StaticResource TertiaryTextBrush}" />
            </StackPanel>

            <!-- Hook list -->
            <ListView x:Name="HookList" SelectionMode="Single"
                      SelectionChanged="HookList_SelectionChanged"
                      Background="Transparent">
                <ListView.ItemTemplate>
                    <DataTemplate>
                        <Border CornerRadius="10" Padding="14,10" Margin="0,2"
                                Background="{StaticResource CardFillBrush}"
                                BorderBrush="{StaticResource CardBorderBrush}"
                                BorderThickness="1">
                            <Grid>
                                <Grid.ColumnDefinitions>
                                    <ColumnDefinition Width="Auto" />
                                    <ColumnDefinition Width="*" />
                                    <ColumnDefinition Width="Auto" />
                                </Grid.ColumnDefinitions>
                                <Ellipse Grid.Column="0" Width="8" Height="8"
                                         Margin="0,0,10,0" VerticalAlignment="Center" />
                                <StackPanel Grid.Column="1" Spacing="2">
                                    <StackPanel Orientation="Horizontal" Spacing="8">
                                        <TextBlock Text="{Binding Event}"
                                                   FontFamily="{StaticResource MonoFont}"
                                                   FontSize="13" FontWeight="SemiBold"
                                                   Foreground="{StaticResource PrimaryTextBrush}" />
                                        <Border CornerRadius="4" Padding="4,1"
                                                Background="{StaticResource CardFillBrush}"
                                                Visibility="{Binding Matcher}">
                                            <TextBlock Text="{Binding Matcher}"
                                                       FontFamily="{StaticResource MonoFont}"
                                                       FontSize="10"
                                                       Foreground="{StaticResource TertiaryTextBrush}" />
                                        </Border>
                                    </StackPanel>
                                    <TextBlock FontSize="11"
                                               Foreground="{StaticResource TertiaryTextBrush}"
                                               TextTrimming="CharacterEllipsis"
                                               MaxLines="1" />
                                </StackPanel>
                                <Border Grid.Column="2" CornerRadius="4" Padding="6,2"
                                        VerticalAlignment="Center">
                                    <TextBlock FontSize="10" FontWeight="Bold"
                                               Foreground="White" />
                                </Border>
                            </Grid>
                        </Border>
                    </DataTemplate>
                </ListView.ItemTemplate>
            </ListView>
        </Grid>

        <!-- Editor panel -->
        <Border Grid.Row="2" x:Name="EditorPanel" Visibility="Collapsed"
                CornerRadius="12" Margin="0,16,0,0" Padding="16"
                Background="{StaticResource CardFillBrush}"
                BorderBrush="{StaticResource CardBorderBrush}" BorderThickness="1"
                MinHeight="180" MaxHeight="350">
            <Grid>
                <Grid.RowDefinitions>
                    <RowDefinition Height="Auto" />
                    <RowDefinition Height="*" />
                    <RowDefinition Height="Auto" />
                </Grid.RowDefinitions>

                <Grid Grid.Row="0" Margin="0,0,0,8">
                    <Grid.ColumnDefinitions>
                        <ColumnDefinition Width="*" />
                        <ColumnDefinition Width="Auto" />
                    </Grid.ColumnDefinitions>
                    <StackPanel Orientation="Horizontal" Spacing="8">
                        <TextBlock x:Name="EditorEventName" FontSize="14" FontWeight="Bold"
                                   FontFamily="{StaticResource MonoFont}"
                                   Foreground="{StaticResource PrimaryTextBrush}" />
                        <Border x:Name="EditorStatusBadge" CornerRadius="4" Padding="6,2">
                            <TextBlock x:Name="EditorStatusText" FontSize="10" FontWeight="Bold"
                                       Foreground="White" />
                        </Border>
                    </StackPanel>
                    <Button Grid.Column="1" Content="✕" Click="CloseEditor_Click"
                            Background="Transparent" Foreground="{StaticResource TertiaryTextBrush}"
                            FontSize="16" Padding="4" />
                </Grid>

                <TextBox Grid.Row="1" x:Name="HookEditor"
                         AcceptsReturn="True" TextWrapping="Wrap"
                         FontFamily="{StaticResource MonoFont}" FontSize="12"
                         Background="{StaticResource BgPrimaryBrush}"
                         Foreground="{StaticResource PrimaryTextBrush}"
                         BorderThickness="1" CornerRadius="8"
                         BorderBrush="{StaticResource CardBorderBrush}" />

                <StackPanel Grid.Row="2" Orientation="Horizontal" Spacing="8"
                            HorizontalAlignment="Right" Margin="0,8,0,0">
                    <Button x:Name="BtnSaveHook" Content="Save" Click="SaveHook_Click"
                            Background="{StaticResource NeonGreenBrush}" Foreground="Black"
                            FontWeight="SemiBold" CornerRadius="6" Padding="12,6" />
                    <Button x:Name="BtnToggleHook" Click="ToggleHook_Click"
                            CornerRadius="6" Padding="12,6" FontWeight="SemiBold" />
                </StackPanel>
            </Grid>
        </Border>

        <!-- Footer -->
        <Grid Grid.Row="3" Margin="0,12,0,0">
            <StackPanel Orientation="Horizontal" Spacing="6">
                <FontIcon Glyph="&#xE946;" FontSize="12"
                          Foreground="{StaticResource TertiaryTextBrush}" />
                <TextBlock Text="Defined in ~/.claude/settings.json" FontSize="11"
                           Foreground="{StaticResource TertiaryTextBrush}" />
                <TextBlock x:Name="HookCountText" FontSize="11"
                           Foreground="{StaticResource TertiaryTextBrush}" Margin="16,0,0,0" />
            </StackPanel>
        </Grid>
    </Grid>
</Page>
```

- [ ] **Step 2: Create HooksPage.xaml.cs**

```csharp
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Navigation;
using Configonaut.Models;
using Configonaut.Services;

namespace Configonaut.Views;

public sealed partial class HooksPage : Page
{
    private ConfigManager _config = null!;
    private HookRule? _selectedHook;

    public HooksPage()
    {
        this.InitializeComponent();
    }

    protected override void OnNavigatedTo(NavigationEventArgs e)
    {
        if (e.Parameter is ConfigManager config)
        {
            _config = config;
            _config.PropertyChanged += Config_PropertyChanged;
            RefreshUI();
        }
    }

    protected override void OnNavigatedFrom(NavigationEventArgs e)
    {
        if (_config is not null)
            _config.PropertyChanged -= Config_PropertyChanged;
    }

    private void Config_PropertyChanged(object? sender, System.ComponentModel.PropertyChangedEventArgs e)
    {
        DispatcherQueue.TryEnqueue(RefreshUI);
    }

    private void RefreshUI()
    {
        HookList.ItemsSource = _config.HookRules;
        var hasHooks = _config.HookRules.Count > 0;
        HookList.Visibility = hasHooks ? Visibility.Visible : Visibility.Collapsed;
        EmptyState.Visibility = hasHooks ? Visibility.Collapsed : Visibility.Visible;

        var enabled = _config.HookRules.Count(h => h.IsEnabled);
        var disabled = _config.HookRules.Count - enabled;
        HookCountText.Text = $"{enabled} enabled, {disabled} disabled";
    }

    private void HookList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (HookList.SelectedItem is HookRule hook)
            ShowEditor(hook);
    }

    private void ShowEditor(HookRule hook)
    {
        _selectedHook = hook;
        EditorPanel.Visibility = Visibility.Visible;
        EditorEventName.Text = hook.Event;
        EditorStatusText.Text = hook.IsEnabled ? "ON" : "OFF";
        EditorStatusBadge.Background = hook.IsEnabled
            ? (Microsoft.UI.Xaml.Media.SolidColorBrush)Application.Current.Resources["NeonGreenBrush"]
            : (Microsoft.UI.Xaml.Media.SolidColorBrush)Application.Current.Resources["NeonRedBrush"];

        var json = _config.GetHookRuleJson(hook);
        HookEditor.Text = json ?? "";

        BtnToggleHook.Content = hook.IsEnabled ? "Disable" : "Enable";
        BtnToggleHook.Background = hook.IsEnabled
            ? (Microsoft.UI.Xaml.Media.SolidColorBrush)Application.Current.Resources["NeonRedBrush"]
            : (Microsoft.UI.Xaml.Media.SolidColorBrush)Application.Current.Resources["NeonGreenBrush"];
        BtnToggleHook.Foreground = hook.IsEnabled
            ? new Microsoft.UI.Xaml.Media.SolidColorBrush(Microsoft.UI.Colors.White)
            : new Microsoft.UI.Xaml.Media.SolidColorBrush(Microsoft.UI.Colors.Black);
    }

    private void SaveHook_Click(object sender, RoutedEventArgs e)
    {
        if (_selectedHook is null) return;
        _config.UpdateHookRule(_selectedHook, HookEditor.Text);
    }

    private void ToggleHook_Click(object sender, RoutedEventArgs e)
    {
        if (_selectedHook is null) return;
        _config.ToggleHook(_selectedHook);
        CloseEditorPanel();
    }

    private void CloseEditor_Click(object sender, RoutedEventArgs e) => CloseEditorPanel();

    private void CloseEditorPanel()
    {
        _selectedHook = null;
        HookList.SelectedItem = null;
        EditorPanel.Visibility = Visibility.Collapsed;
    }
}
```

- [ ] **Step 3: Verify it compiles**

```bash
dotnet build --configuration Debug
```

Expected: Build succeeds.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: add HooksPage with hook list, JSON editor, and toggle support"
```

---

### Task 11: AgentsPage

**Files:**
- Create: `Configonaut/Views/AgentsPage.xaml`
- Create: `Configonaut/Views/AgentsPage.xaml.cs`

- [ ] **Step 1: Create AgentsPage.xaml**

```xml
<Page
    x:Class="Configonaut.Views.AgentsPage"
    xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
    Background="Transparent">

    <Grid Padding="24">
        <Grid.RowDefinitions>
            <RowDefinition Height="Auto" />
            <RowDefinition Height="Auto" />
            <RowDefinition Height="*" />
            <RowDefinition Height="Auto" />
        </Grid.RowDefinitions>

        <!-- Header -->
        <Grid Grid.Row="0" Margin="0,0,0,12">
            <Grid.ColumnDefinitions>
                <ColumnDefinition Width="*" />
                <ColumnDefinition Width="Auto" />
            </Grid.ColumnDefinitions>
            <StackPanel>
                <TextBlock Text="Agents" FontSize="22" FontWeight="Bold"
                           Foreground="{StaticResource PrimaryTextBrush}" />
                <TextBlock Text="Personal and plugin agent configurations"
                           FontSize="12" Foreground="{StaticResource TertiaryTextBrush}" />
            </StackPanel>
            <Button Grid.Column="1" Content="＋ New Agent" Click="NewAgent_Click"
                    VerticalAlignment="Top"
                    Background="{StaticResource NeonPurpleBrush}" Foreground="White"
                    FontWeight="SemiBold" CornerRadius="8" Padding="16,8" />
        </Grid>

        <!-- Search -->
        <TextBox Grid.Row="1" x:Name="SearchBox" PlaceholderText="Search agents..."
                 TextChanged="SearchBox_TextChanged" Margin="0,0,0,12"
                 CornerRadius="8" Background="{StaticResource CardFillBrush}"
                 Foreground="{StaticResource PrimaryTextBrush}" />

        <!-- Agent list or empty state -->
        <Grid Grid.Row="2">
            <StackPanel x:Name="EmptyState" Visibility="Collapsed"
                        HorizontalAlignment="Center" VerticalAlignment="Center" Spacing="12">
                <FontIcon Glyph="&#xE716;" FontSize="48"
                          Foreground="{StaticResource TertiaryTextBrush}"
                          HorizontalAlignment="Center" />
                <TextBlock Text="No agents found" FontSize="16" FontWeight="SemiBold"
                           HorizontalAlignment="Center"
                           Foreground="{StaticResource SecondaryTextBrush}" />
            </StackPanel>

            <ListView x:Name="AgentList" SelectionMode="Single"
                      SelectionChanged="AgentList_SelectionChanged"
                      Background="Transparent">
                <ListView.ItemTemplate>
                    <DataTemplate>
                        <Border CornerRadius="10" Padding="14,10" Margin="0,2"
                                Background="{StaticResource CardFillBrush}"
                                BorderBrush="{StaticResource CardBorderBrush}"
                                BorderThickness="1">
                            <Grid>
                                <Grid.ColumnDefinitions>
                                    <ColumnDefinition Width="Auto" />
                                    <ColumnDefinition Width="*" />
                                    <ColumnDefinition Width="Auto" />
                                </Grid.ColumnDefinitions>
                                <Ellipse Grid.Column="0" Width="8" Height="8"
                                         Fill="{StaticResource NeonPurpleBrush}"
                                         Margin="0,0,10,0" VerticalAlignment="Top"
                                         Margin="0,4,10,0" />
                                <StackPanel Grid.Column="1" Spacing="2">
                                    <StackPanel Orientation="Horizontal" Spacing="8">
                                        <TextBlock Text="{Binding Name}"
                                                   FontFamily="{StaticResource MonoFont}"
                                                   FontSize="13" FontWeight="SemiBold"
                                                   Foreground="{StaticResource PrimaryTextBrush}" />
                                        <Border CornerRadius="4" Padding="4,1"
                                                Background="{StaticResource CardFillBrush}">
                                            <TextBlock Text="{Binding Model}"
                                                       FontSize="10"
                                                       Foreground="{StaticResource TertiaryTextBrush}" />
                                        </Border>
                                    </StackPanel>
                                    <TextBlock Text="{Binding Description}" FontSize="11"
                                               Foreground="{StaticResource TertiaryTextBrush}"
                                               TextTrimming="CharacterEllipsis" MaxLines="1" />
                                    <TextBlock Text="{Binding PluginName}" FontSize="10"
                                               Foreground="{StaticResource TertiaryTextBrush}" />
                                </StackPanel>
                                <FontIcon Grid.Column="2" Glyph="&#xE76C;" FontSize="12"
                                          Foreground="{StaticResource TertiaryTextBrush}"
                                          VerticalAlignment="Center" />
                            </Grid>
                        </Border>
                    </DataTemplate>
                </ListView.ItemTemplate>
            </ListView>
        </Grid>

        <!-- Editor panel (reused for detail and create) -->
        <Border Grid.Row="3" x:Name="EditorPanel" Visibility="Collapsed"
                CornerRadius="12" Margin="0,16,0,0" Padding="16"
                Background="{StaticResource CardFillBrush}"
                BorderBrush="{StaticResource CardBorderBrush}" BorderThickness="1"
                MinHeight="200" MaxHeight="400">
            <Grid>
                <Grid.RowDefinitions>
                    <RowDefinition Height="Auto" />
                    <RowDefinition Height="Auto" />
                    <RowDefinition Height="*" />
                    <RowDefinition Height="Auto" />
                </Grid.RowDefinitions>

                <!-- Header -->
                <Grid Grid.Row="0" Margin="0,0,0,8">
                    <Grid.ColumnDefinitions>
                        <ColumnDefinition Width="*" />
                        <ColumnDefinition Width="Auto" />
                    </Grid.ColumnDefinitions>
                    <StackPanel Orientation="Horizontal" Spacing="8">
                        <TextBlock x:Name="EditorTitle" FontSize="14" FontWeight="Bold"
                                   Foreground="{StaticResource PrimaryTextBrush}" />
                        <Border x:Name="EditorSourceBadge" CornerRadius="4" Padding="6,2"
                                Background="{StaticResource CardFillBrush}">
                            <TextBlock x:Name="EditorSourceText" FontSize="10"
                                       Foreground="{StaticResource TertiaryTextBrush}" />
                        </Border>
                    </StackPanel>
                    <Button Grid.Column="1" Content="✕" Click="CloseEditor_Click"
                            Background="Transparent" Foreground="{StaticResource TertiaryTextBrush}"
                            FontSize="16" Padding="4" />
                </Grid>

                <!-- Name field (create mode only) -->
                <StackPanel Grid.Row="1" x:Name="NamePanel" Visibility="Collapsed"
                            Orientation="Horizontal" Spacing="8" Margin="0,0,0,8">
                    <TextBlock Text="Name:" FontSize="13" VerticalAlignment="Center"
                               Foreground="{StaticResource SecondaryTextBrush}" />
                    <TextBox x:Name="AgentNameField" Width="200"
                             FontFamily="{StaticResource MonoFont}"
                             PlaceholderText="my-agent"
                             TextChanged="AgentNameField_TextChanged" />
                </StackPanel>

                <!-- Content editor -->
                <TextBox Grid.Row="2" x:Name="AgentEditor"
                         AcceptsReturn="True" TextWrapping="Wrap"
                         FontFamily="{StaticResource MonoFont}" FontSize="12"
                         Background="{StaticResource BgPrimaryBrush}"
                         Foreground="{StaticResource PrimaryTextBrush}"
                         BorderThickness="1" CornerRadius="8"
                         BorderBrush="{StaticResource CardBorderBrush}" />

                <!-- Actions -->
                <StackPanel Grid.Row="3" Orientation="Horizontal" Spacing="8"
                            HorizontalAlignment="Right" Margin="0,8,0,0">
                    <Button x:Name="BtnSaveAgent" Content="Save" Click="SaveAgent_Click"
                            Background="{StaticResource NeonGreenBrush}" Foreground="Black"
                            FontWeight="SemiBold" CornerRadius="6" Padding="12,6" />
                    <Button x:Name="BtnTogglePlugin" Click="TogglePlugin_Click"
                            Visibility="Collapsed"
                            CornerRadius="6" Padding="12,6" FontWeight="SemiBold" />
                    <Button x:Name="BtnDeleteAgent" Content="Delete" Click="DeleteAgent_Click"
                            Visibility="Collapsed"
                            Background="{StaticResource NeonRedBrush}" Foreground="White"
                            FontWeight="SemiBold" CornerRadius="6" Padding="12,6" />
                    <Button x:Name="BtnCreateAgent" Content="Create" Click="CreateAgent_Click"
                            Visibility="Collapsed"
                            Background="{StaticResource NeonPurpleBrush}" Foreground="White"
                            FontWeight="SemiBold" CornerRadius="6" Padding="12,6" />
                </StackPanel>
            </Grid>
        </Border>
    </Grid>
</Page>
```

- [ ] **Step 2: Create AgentsPage.xaml.cs**

```csharp
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Navigation;
using Configonaut.Models;
using Configonaut.Services;

namespace Configonaut.Views;

public sealed partial class AgentsPage : Page
{
    private ConfigManager _config = null!;
    private AgentEntry? _selectedAgent;
    private bool _isCreateMode;
    private string _searchQuery = "";

    private const string AgentTemplate = "---\nname: {0}\ndescription: A custom agent\ntools: Read, Edit, Write, Bash, Glob, Grep\nmodel: sonnet\ncolor: blue\n---\n\nDescribe this agent's purpose and behavior here.\n";

    public AgentsPage()
    {
        this.InitializeComponent();
    }

    protected override void OnNavigatedTo(NavigationEventArgs e)
    {
        if (e.Parameter is ConfigManager config)
        {
            _config = config;
            _config.PropertyChanged += Config_PropertyChanged;
            RefreshUI();
        }
    }

    protected override void OnNavigatedFrom(NavigationEventArgs e)
    {
        if (_config is not null)
            _config.PropertyChanged -= Config_PropertyChanged;
    }

    private void Config_PropertyChanged(object? sender, System.ComponentModel.PropertyChangedEventArgs e)
    {
        DispatcherQueue.TryEnqueue(RefreshUI);
    }

    private void RefreshUI()
    {
        var filtered = string.IsNullOrEmpty(_searchQuery)
            ? _config.Agents.ToList()
            : _config.Agents.Where(a =>
                a.Name.Contains(_searchQuery, StringComparison.OrdinalIgnoreCase) ||
                a.Description.Contains(_searchQuery, StringComparison.OrdinalIgnoreCase) ||
                a.PluginName.Contains(_searchQuery, StringComparison.OrdinalIgnoreCase)
            ).ToList();

        AgentList.ItemsSource = filtered;
        EmptyState.Visibility = filtered.Count == 0 ? Visibility.Visible : Visibility.Collapsed;
        AgentList.Visibility = filtered.Count > 0 ? Visibility.Visible : Visibility.Collapsed;
    }

    private void SearchBox_TextChanged(object sender, TextChangedEventArgs e)
    {
        _searchQuery = SearchBox.Text;
        RefreshUI();
    }

    private void AgentList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (AgentList.SelectedItem is AgentEntry agent)
            ShowDetailEditor(agent);
    }

    private void ShowDetailEditor(AgentEntry agent)
    {
        _selectedAgent = agent;
        _isCreateMode = false;

        EditorPanel.Visibility = Visibility.Visible;
        NamePanel.Visibility = Visibility.Collapsed;
        EditorTitle.Text = agent.Name;
        EditorSourceText.Text = agent.Source == AgentSource.Personal ? "Personal" : agent.PluginName;

        try { AgentEditor.Text = File.ReadAllText(agent.FilePath); }
        catch { AgentEditor.Text = ""; }

        BtnSaveAgent.Visibility = Visibility.Visible;
        BtnDeleteAgent.Visibility = agent.Source == AgentSource.Personal ? Visibility.Visible : Visibility.Collapsed;
        BtnTogglePlugin.Visibility = agent.Source == AgentSource.Plugin ? Visibility.Visible : Visibility.Collapsed;
        BtnCreateAgent.Visibility = Visibility.Collapsed;

        if (agent.Source == AgentSource.Plugin)
        {
            var pluginKey = $"{agent.PluginName}@claude-plugins-official";
            var isEnabled = _config.EnabledPlugins.TryGetValue(pluginKey, out var e2) && e2;
            BtnTogglePlugin.Content = isEnabled ? "Disable Plugin" : "Enable Plugin";
        }
    }

    private void NewAgent_Click(object sender, RoutedEventArgs e)
    {
        _isCreateMode = true;
        _selectedAgent = null;
        AgentList.SelectedItem = null;

        EditorPanel.Visibility = Visibility.Visible;
        NamePanel.Visibility = Visibility.Visible;
        EditorTitle.Text = "New Agent";
        EditorSourceText.Text = "Personal";
        AgentNameField.Text = "";
        AgentEditor.Text = string.Format(AgentTemplate, "my-agent");

        BtnSaveAgent.Visibility = Visibility.Collapsed;
        BtnDeleteAgent.Visibility = Visibility.Collapsed;
        BtnTogglePlugin.Visibility = Visibility.Collapsed;
        BtnCreateAgent.Visibility = Visibility.Visible;
    }

    private void AgentNameField_TextChanged(object sender, TextChangedEventArgs e)
    {
        var name = AgentNameField.Text.ToLowerInvariant().Replace(' ', '-');
        name = new string(name.Where(c => char.IsLetterOrDigit(c) || c == '-').ToArray());
        if (!string.IsNullOrEmpty(name))
            AgentEditor.Text = string.Format(AgentTemplate, name);
    }

    private void SaveAgent_Click(object sender, RoutedEventArgs e)
    {
        if (_selectedAgent is null) return;
        try
        {
            File.WriteAllText(_selectedAgent.FilePath, AgentEditor.Text);
            _config.LoadAgents();
        }
        catch { }
    }

    private void CreateAgent_Click(object sender, RoutedEventArgs e)
    {
        if (string.IsNullOrWhiteSpace(AgentNameField.Text)) return;
        var path = _config.CreateAgent(AgentNameField.Text);
        if (path is null) return;
        File.WriteAllText(path, AgentEditor.Text);
        _config.LoadAgents();
        CloseEditorPanel();
    }

    private async void DeleteAgent_Click(object sender, RoutedEventArgs e)
    {
        if (_selectedAgent is null) return;
        var dialog = new ContentDialog
        {
            Title = "Delete Agent",
            Content = $"Delete '{_selectedAgent.Name}'?",
            PrimaryButtonText = "Delete",
            CloseButtonText = "Cancel",
            XamlRoot = this.XamlRoot
        };
        if (await dialog.ShowAsync() == ContentDialogResult.Primary)
        {
            _config.DeleteAgent(_selectedAgent);
            CloseEditorPanel();
        }
    }

    private void TogglePlugin_Click(object sender, RoutedEventArgs e)
    {
        if (_selectedAgent is null) return;
        var pluginKey = $"{_selectedAgent.PluginName}@claude-plugins-official";
        _config.TogglePlugin(pluginKey);
        CloseEditorPanel();
    }

    private void CloseEditor_Click(object sender, RoutedEventArgs e) => CloseEditorPanel();

    private void CloseEditorPanel()
    {
        _selectedAgent = null;
        _isCreateMode = false;
        AgentList.SelectedItem = null;
        EditorPanel.Visibility = Visibility.Collapsed;
    }
}
```

- [ ] **Step 3: Verify it compiles**

```bash
dotnet build --configuration Debug
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: add AgentsPage with search, plugin grouping, create, and edit"
```

---

### Task 12: SkillsPage

**Files:**
- Create: `Configonaut/Views/SkillsPage.xaml`
- Create: `Configonaut/Views/SkillsPage.xaml.cs`

- [ ] **Step 1: Create SkillsPage.xaml**

```xml
<Page
    x:Class="Configonaut.Views.SkillsPage"
    xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
    xmlns:ctc="using:CommunityToolkit.WinUI.Controls"
    Background="Transparent">

    <Grid Padding="24">
        <Grid.RowDefinitions>
            <RowDefinition Height="Auto" />
            <RowDefinition Height="Auto" />
            <RowDefinition Height="*" />
            <RowDefinition Height="Auto" />
        </Grid.RowDefinitions>

        <!-- Header -->
        <Grid Grid.Row="0" Margin="0,0,0,12">
            <Grid.ColumnDefinitions>
                <ColumnDefinition Width="*" />
                <ColumnDefinition Width="Auto" />
            </Grid.ColumnDefinitions>
            <StackPanel>
                <TextBlock Text="Skills" FontSize="22" FontWeight="Bold"
                           Foreground="{StaticResource PrimaryTextBrush}" />
                <TextBlock Text="Commands &amp; slash skills"
                           FontSize="12" Foreground="{StaticResource TertiaryTextBrush}" />
            </StackPanel>
            <Button Grid.Column="1" Content="＋ New Skill" Click="NewSkill_Click"
                    VerticalAlignment="Top"
                    Background="{StaticResource NeonAmberBrush}" Foreground="Black"
                    FontWeight="SemiBold" CornerRadius="8" Padding="16,8" />
        </Grid>

        <!-- Search -->
        <TextBox Grid.Row="1" x:Name="SearchBox" PlaceholderText="Search skills..."
                 TextChanged="SearchBox_TextChanged" Margin="0,0,0,12"
                 CornerRadius="8" Background="{StaticResource CardFillBrush}"
                 Foreground="{StaticResource PrimaryTextBrush}" />

        <!-- Skill list or empty state -->
        <Grid Grid.Row="2">
            <StackPanel x:Name="EmptyState" Visibility="Collapsed"
                        HorizontalAlignment="Center" VerticalAlignment="Center" Spacing="12">
                <FontIcon Glyph="&#xE734;" FontSize="48"
                          Foreground="{StaticResource TertiaryTextBrush}"
                          HorizontalAlignment="Center" />
                <TextBlock Text="No skills found" FontSize="16" FontWeight="SemiBold"
                           HorizontalAlignment="Center"
                           Foreground="{StaticResource SecondaryTextBrush}" />
            </StackPanel>

            <ListView x:Name="SkillList" SelectionMode="Single"
                      SelectionChanged="SkillList_SelectionChanged"
                      Background="Transparent">
                <ListView.ItemTemplate>
                    <DataTemplate>
                        <Border CornerRadius="10" Padding="14,10" Margin="0,2"
                                Background="{StaticResource CardFillBrush}"
                                BorderBrush="{StaticResource CardBorderBrush}"
                                BorderThickness="1">
                            <Grid>
                                <Grid.ColumnDefinitions>
                                    <ColumnDefinition Width="Auto" />
                                    <ColumnDefinition Width="*" />
                                    <ColumnDefinition Width="Auto" />
                                </Grid.ColumnDefinitions>
                                <Ellipse Grid.Column="0" Width="8" Height="8"
                                         Fill="{StaticResource NeonAmberBrush}"
                                         Margin="0,4,10,0" VerticalAlignment="Top" />
                                <StackPanel Grid.Column="1" Spacing="2">
                                    <StackPanel Orientation="Horizontal" Spacing="8">
                                        <TextBlock Text="{Binding Name}"
                                                   FontFamily="{StaticResource MonoFont}"
                                                   FontSize="13" FontWeight="SemiBold"
                                                   Foreground="{StaticResource PrimaryTextBrush}" />
                                        <Border CornerRadius="4" Padding="4,1"
                                                Background="{StaticResource CardFillBrush}">
                                            <TextBlock Text="{Binding SourceLabel}" FontSize="10"
                                                       Foreground="{StaticResource TertiaryTextBrush}" />
                                        </Border>
                                    </StackPanel>
                                    <TextBlock Text="{Binding Description}" FontSize="11"
                                               Foreground="{StaticResource TertiaryTextBrush}"
                                               TextTrimming="CharacterEllipsis" MaxLines="1" />
                                </StackPanel>
                                <FontIcon Grid.Column="2" Glyph="&#xE76C;" FontSize="12"
                                          Foreground="{StaticResource TertiaryTextBrush}"
                                          VerticalAlignment="Center" />
                            </Grid>
                        </Border>
                    </DataTemplate>
                </ListView.ItemTemplate>
            </ListView>
        </Grid>

        <!-- Editor panel -->
        <Border Grid.Row="3" x:Name="EditorPanel" Visibility="Collapsed"
                CornerRadius="12" Margin="0,16,0,0" Padding="16"
                Background="{StaticResource CardFillBrush}"
                BorderBrush="{StaticResource CardBorderBrush}" BorderThickness="1"
                MinHeight="200" MaxHeight="400">
            <Grid>
                <Grid.RowDefinitions>
                    <RowDefinition Height="Auto" />
                    <RowDefinition Height="Auto" />
                    <RowDefinition Height="*" />
                    <RowDefinition Height="Auto" />
                </Grid.RowDefinitions>

                <Grid Grid.Row="0" Margin="0,0,0,8">
                    <Grid.ColumnDefinitions>
                        <ColumnDefinition Width="*" />
                        <ColumnDefinition Width="Auto" />
                    </Grid.ColumnDefinitions>
                    <StackPanel Orientation="Horizontal" Spacing="8">
                        <TextBlock x:Name="EditorTitle" FontSize="14" FontWeight="Bold"
                                   Foreground="{StaticResource PrimaryTextBrush}" />
                        <Border x:Name="EditorSourceBadge" CornerRadius="4" Padding="6,2"
                                Background="{StaticResource CardFillBrush}">
                            <TextBlock x:Name="EditorSourceText" FontSize="10"
                                       Foreground="{StaticResource TertiaryTextBrush}" />
                        </Border>
                    </StackPanel>
                    <Button Grid.Column="1" Content="✕" Click="CloseEditor_Click"
                            Background="Transparent" Foreground="{StaticResource TertiaryTextBrush}"
                            FontSize="16" Padding="4" />
                </Grid>

                <!-- Create mode: name + type picker -->
                <StackPanel Grid.Row="1" x:Name="CreatePanel" Visibility="Collapsed"
                            Spacing="8" Margin="0,0,0,8">
                    <StackPanel Orientation="Horizontal" Spacing="8">
                        <TextBlock Text="Name:" FontSize="13" VerticalAlignment="Center"
                                   Foreground="{StaticResource SecondaryTextBrush}" />
                        <TextBox x:Name="SkillNameField" Width="200"
                                 FontFamily="{StaticResource MonoFont}"
                                 PlaceholderText="my-skill"
                                 TextChanged="SkillNameField_TextChanged" />
                    </StackPanel>
                    <ctc:Segmented x:Name="TypeToggle" SelectionMode="Single"
                                   SelectionChanged="TypeToggle_SelectionChanged">
                        <ctc:SegmentedItem Content="Command" Tag="Command" />
                        <ctc:SegmentedItem Content="Skill" Tag="Skill" />
                    </ctc:Segmented>
                </StackPanel>

                <TextBox Grid.Row="2" x:Name="SkillEditor"
                         AcceptsReturn="True" TextWrapping="Wrap"
                         FontFamily="{StaticResource MonoFont}" FontSize="12"
                         Background="{StaticResource BgPrimaryBrush}"
                         Foreground="{StaticResource PrimaryTextBrush}"
                         BorderThickness="1" CornerRadius="8"
                         BorderBrush="{StaticResource CardBorderBrush}" />

                <StackPanel Grid.Row="3" Orientation="Horizontal" Spacing="8"
                            HorizontalAlignment="Right" Margin="0,8,0,0">
                    <Button x:Name="BtnSaveSkill" Content="Save" Click="SaveSkill_Click"
                            Visibility="Collapsed"
                            Background="{StaticResource NeonGreenBrush}" Foreground="Black"
                            FontWeight="SemiBold" CornerRadius="6" Padding="12,6" />
                    <Button x:Name="BtnToggleSkill" Click="ToggleSkill_Click"
                            Visibility="Collapsed"
                            CornerRadius="6" Padding="12,6" FontWeight="SemiBold" />
                    <Button x:Name="BtnCreateSkill" Content="Create" Click="CreateSkill_Click"
                            Visibility="Collapsed"
                            Background="{StaticResource NeonAmberBrush}" Foreground="Black"
                            FontWeight="SemiBold" CornerRadius="6" Padding="12,6" />
                </StackPanel>
            </Grid>
        </Border>
    </Grid>
</Page>
```

- [ ] **Step 2: Create SkillsPage.xaml.cs**

```csharp
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Navigation;
using CommunityToolkit.WinUI.Controls;
using Configonaut.Models;
using Configonaut.Services;

namespace Configonaut.Views;

public sealed partial class SkillsPage : Page
{
    private ConfigManager _config = null!;
    private SkillEntry? _selectedSkill;
    private bool _isCreateMode;
    private string _searchQuery = "";
    private SkillSource _newSkillType = SkillSource.Command;

    private const string CommandTemplate = "---\nname: {0}\ndescription: A custom command\n---\n\nDescribe what this command does.\n";
    private const string SkillTemplate = "---\nname: {0}\ndescription: A custom skill\n---\n\nDescribe what this skill does.\n";

    public SkillsPage()
    {
        this.InitializeComponent();
    }

    protected override void OnNavigatedTo(NavigationEventArgs e)
    {
        if (e.Parameter is ConfigManager config)
        {
            _config = config;
            _config.PropertyChanged += Config_PropertyChanged;
            RefreshUI();
        }
    }

    protected override void OnNavigatedFrom(NavigationEventArgs e)
    {
        if (_config is not null)
            _config.PropertyChanged -= Config_PropertyChanged;
    }

    private void Config_PropertyChanged(object? sender, System.ComponentModel.PropertyChangedEventArgs e)
    {
        DispatcherQueue.TryEnqueue(RefreshUI);
    }

    private void RefreshUI()
    {
        var filtered = string.IsNullOrEmpty(_searchQuery)
            ? _config.Skills.ToList()
            : _config.Skills.Where(s =>
                s.Name.Contains(_searchQuery, StringComparison.OrdinalIgnoreCase) ||
                s.Description.Contains(_searchQuery, StringComparison.OrdinalIgnoreCase)
            ).ToList();

        SkillList.ItemsSource = filtered;
        EmptyState.Visibility = filtered.Count == 0 ? Visibility.Visible : Visibility.Collapsed;
        SkillList.Visibility = filtered.Count > 0 ? Visibility.Visible : Visibility.Collapsed;
    }

    private void SearchBox_TextChanged(object sender, TextChangedEventArgs e)
    {
        _searchQuery = SearchBox.Text;
        RefreshUI();
    }

    private void SkillList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (SkillList.SelectedItem is SkillEntry skill)
            ShowDetailEditor(skill);
    }

    private void ShowDetailEditor(SkillEntry skill)
    {
        _selectedSkill = skill;
        _isCreateMode = false;

        EditorPanel.Visibility = Visibility.Visible;
        CreatePanel.Visibility = Visibility.Collapsed;
        EditorTitle.Text = skill.Name;
        EditorSourceText.Text = skill.SourceLabel;

        try { SkillEditor.Text = File.ReadAllText(skill.FilePath); }
        catch { SkillEditor.Text = ""; }

        BtnSaveSkill.Visibility = Visibility.Visible;
        BtnToggleSkill.Visibility = skill.Source != SkillSource.Plugin ? Visibility.Visible : Visibility.Collapsed;
        BtnCreateSkill.Visibility = Visibility.Collapsed;

        if (skill.Source != SkillSource.Plugin)
        {
            BtnToggleSkill.Content = skill.IsEnabled ? "Disable" : "Enable";
            BtnToggleSkill.Background = skill.IsEnabled
                ? (Microsoft.UI.Xaml.Media.SolidColorBrush)Application.Current.Resources["NeonRedBrush"]
                : (Microsoft.UI.Xaml.Media.SolidColorBrush)Application.Current.Resources["NeonGreenBrush"];
            BtnToggleSkill.Foreground = skill.IsEnabled
                ? new Microsoft.UI.Xaml.Media.SolidColorBrush(Microsoft.UI.Colors.White)
                : new Microsoft.UI.Xaml.Media.SolidColorBrush(Microsoft.UI.Colors.Black);
        }
    }

    private void NewSkill_Click(object sender, RoutedEventArgs e)
    {
        _isCreateMode = true;
        _selectedSkill = null;
        SkillList.SelectedItem = null;

        EditorPanel.Visibility = Visibility.Visible;
        CreatePanel.Visibility = Visibility.Visible;
        EditorTitle.Text = "New Skill";
        EditorSourceText.Text = "";
        SkillNameField.Text = "";
        TypeToggle.SelectedIndex = 0;
        _newSkillType = SkillSource.Command;
        SkillEditor.Text = string.Format(CommandTemplate, "my-command");

        BtnSaveSkill.Visibility = Visibility.Collapsed;
        BtnToggleSkill.Visibility = Visibility.Collapsed;
        BtnCreateSkill.Visibility = Visibility.Visible;
    }

    private void SkillNameField_TextChanged(object sender, TextChangedEventArgs e) => UpdateCreateTemplate();

    private void TypeToggle_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (TypeToggle.SelectedItem is SegmentedItem item && item.Tag is string tag)
        {
            _newSkillType = tag == "Skill" ? SkillSource.Skill : SkillSource.Command;
            UpdateCreateTemplate();
        }
    }

    private void UpdateCreateTemplate()
    {
        var name = SkillNameField.Text.ToLowerInvariant().Replace(' ', '-');
        name = new string(name.Where(c => char.IsLetterOrDigit(c) || c == '-').ToArray());
        if (string.IsNullOrEmpty(name)) name = _newSkillType == SkillSource.Command ? "my-command" : "my-skill";

        SkillEditor.Text = _newSkillType == SkillSource.Command
            ? string.Format(CommandTemplate, name)
            : string.Format(SkillTemplate, name);
    }

    private void SaveSkill_Click(object sender, RoutedEventArgs e)
    {
        if (_selectedSkill is null) return;
        try
        {
            File.WriteAllText(_selectedSkill.FilePath, SkillEditor.Text);
            _config.LoadSkills();
        }
        catch { }
    }

    private void ToggleSkill_Click(object sender, RoutedEventArgs e)
    {
        if (_selectedSkill is null) return;
        _config.ToggleSkill(_selectedSkill);
        CloseEditorPanel();
    }

    private void CreateSkill_Click(object sender, RoutedEventArgs e)
    {
        if (string.IsNullOrWhiteSpace(SkillNameField.Text)) return;
        var path = _config.CreateSkill(SkillNameField.Text, _newSkillType);
        if (path is null) return;
        File.WriteAllText(path, SkillEditor.Text);
        _config.LoadSkills();
        CloseEditorPanel();
    }

    private void CloseEditor_Click(object sender, RoutedEventArgs e) => CloseEditorPanel();

    private void CloseEditorPanel()
    {
        _selectedSkill = null;
        _isCreateMode = false;
        SkillList.SelectedItem = null;
        EditorPanel.Visibility = Visibility.Collapsed;
    }
}
```

- [ ] **Step 3: Verify it compiles**

```bash
dotnet build --configuration Debug
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: add SkillsPage with search, type picker, toggle, and create support"
```

---

### Task 13: BackupsPage

**Files:**
- Create: `Configonaut/Views/BackupsPage.xaml`
- Create: `Configonaut/Views/BackupsPage.xaml.cs`

- [ ] **Step 1: Create BackupsPage.xaml**

```xml
<Page
    x:Class="Configonaut.Views.BackupsPage"
    xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
    Background="Transparent">

    <Grid Padding="24">
        <Grid.RowDefinitions>
            <RowDefinition Height="Auto" />
            <RowDefinition Height="*" />
            <RowDefinition Height="Auto" />
        </Grid.RowDefinitions>

        <!-- Header -->
        <StackPanel Grid.Row="0" Margin="0,0,0,16">
            <TextBlock Text="Backups" FontSize="22" FontWeight="Bold"
                       Foreground="{StaticResource PrimaryTextBrush}" />
            <TextBlock Text="Config history &amp; restore"
                       FontSize="12" Foreground="{StaticResource TertiaryTextBrush}" />
        </StackPanel>

        <!-- Content area -->
        <Grid Grid.Row="1">
            <!-- Empty state -->
            <StackPanel x:Name="EmptyState" Visibility="Collapsed"
                        HorizontalAlignment="Center" VerticalAlignment="Center" Spacing="12">
                <FontIcon Glyph="&#xE81C;" FontSize="48"
                          Foreground="{StaticResource TertiaryTextBrush}"
                          HorizontalAlignment="Center" />
                <TextBlock Text="No backups yet" FontSize="16" FontWeight="SemiBold"
                           HorizontalAlignment="Center"
                           Foreground="{StaticResource SecondaryTextBrush}" />
                <TextBlock Text="Backups are created automatically when config files change"
                           FontSize="12" HorizontalAlignment="Center"
                           Foreground="{StaticResource TertiaryTextBrush}" />
            </StackPanel>

            <!-- Split view: list + preview -->
            <Grid x:Name="ContentGrid" Visibility="Collapsed">
                <Grid.ColumnDefinitions>
                    <ColumnDefinition Width="300" />
                    <ColumnDefinition Width="1" />
                    <ColumnDefinition Width="*" />
                </Grid.ColumnDefinitions>

                <!-- Backup list -->
                <ListView Grid.Column="0" x:Name="BackupList"
                          SelectionMode="Single" SelectionChanged="BackupList_SelectionChanged"
                          Background="Transparent">
                    <ListView.ItemTemplate>
                        <DataTemplate>
                            <Border CornerRadius="8" Padding="12,8" Margin="0,2"
                                    Background="{StaticResource CardFillBrush}"
                                    BorderBrush="{StaticResource CardBorderBrush}"
                                    BorderThickness="1">
                                <Grid>
                                    <Grid.ColumnDefinitions>
                                        <ColumnDefinition Width="*" />
                                        <ColumnDefinition Width="Auto" />
                                        <ColumnDefinition Width="Auto" />
                                    </Grid.ColumnDefinitions>
                                    <StackPanel Grid.Column="0">
                                        <TextBlock Text="{Binding FormattedDate}" FontSize="12"
                                                   FontWeight="SemiBold"
                                                   Foreground="{StaticResource PrimaryTextBrush}" />
                                        <TextBlock Text="{Binding FormattedSize}" FontSize="10"
                                                   Foreground="{StaticResource TertiaryTextBrush}" />
                                    </StackPanel>
                                    <Button Grid.Column="1" Content="Restore"
                                            Click="RestoreBackup_Click" Tag="{Binding}"
                                            Background="{StaticResource NeonCyanBrush}"
                                            Foreground="Black" FontSize="10" FontWeight="SemiBold"
                                            CornerRadius="4" Padding="8,3" Margin="4,0" />
                                    <Button Grid.Column="2" Click="DeleteBackup_Click"
                                            Tag="{Binding}" Background="Transparent"
                                            Padding="4">
                                        <FontIcon Glyph="&#xE74D;" FontSize="12"
                                                  Foreground="{StaticResource NeonRedBrush}" />
                                    </Button>
                                </Grid>
                            </Border>
                        </DataTemplate>
                    </ListView.ItemTemplate>
                </ListView>

                <!-- Separator -->
                <Rectangle Grid.Column="1" Fill="{StaticResource SubtleBorderBrush}" />

                <!-- Preview pane -->
                <Grid Grid.Column="2" Padding="16,0,0,0">
                    <Grid.RowDefinitions>
                        <RowDefinition Height="Auto" />
                        <RowDefinition Height="Auto" />
                        <RowDefinition Height="*" />
                    </Grid.RowDefinitions>

                    <!-- No selection message -->
                    <TextBlock x:Name="NoSelectionText" Grid.Row="0"
                               Text="Select a backup to preview"
                               FontSize="13" VerticalAlignment="Center"
                               HorizontalAlignment="Center"
                               Foreground="{StaticResource TertiaryTextBrush}"
                               Margin="0,40,0,0" />

                    <!-- Preview header -->
                    <StackPanel Grid.Row="0" x:Name="PreviewHeader" Visibility="Collapsed"
                                Spacing="4" Margin="0,0,0,12">
                        <TextBlock x:Name="PreviewDate" FontSize="14" FontWeight="SemiBold"
                                   Foreground="{StaticResource PrimaryTextBrush}" />
                        <TextBlock x:Name="PreviewSize" FontSize="11"
                                   Foreground="{StaticResource TertiaryTextBrush}" />
                    </StackPanel>

                    <!-- Diff summary -->
                    <StackPanel Grid.Row="1" x:Name="DiffSummary" Visibility="Collapsed"
                                Spacing="4" Margin="0,0,0,12"
                                Padding="12" CornerRadius="8"
                                Background="{StaticResource CardFillBrush}">
                        <TextBlock Text="Changes after this backup:" FontSize="11"
                                   FontWeight="SemiBold"
                                   Foreground="{StaticResource SecondaryTextBrush}" />
                        <ItemsControl x:Name="DiffItems">
                            <ItemsControl.ItemTemplate>
                                <DataTemplate>
                                    <StackPanel Orientation="Horizontal" Spacing="6">
                                        <TextBlock Text="{Binding Icon}" FontSize="12" />
                                        <TextBlock Text="{Binding Name}"
                                                   FontFamily="{StaticResource MonoFont}"
                                                   FontSize="11"
                                                   Foreground="{StaticResource SecondaryTextBrush}" />
                                    </StackPanel>
                                </DataTemplate>
                            </ItemsControl.ItemTemplate>
                        </ItemsControl>
                    </StackPanel>

                    <!-- Raw JSON preview -->
                    <ScrollViewer Grid.Row="2" x:Name="PreviewScroll" Visibility="Collapsed">
                        <TextBlock x:Name="PreviewContent"
                                   FontFamily="{StaticResource MonoFont}" FontSize="11"
                                   Foreground="{StaticResource SecondaryTextBrush}"
                                   IsTextSelectionEnabled="True"
                                   TextWrapping="Wrap" />
                    </ScrollViewer>
                </Grid>
            </Grid>
        </Grid>

        <!-- Footer -->
        <StackPanel Grid.Row="2" Orientation="Horizontal" Spacing="6" Margin="0,12,0,0">
            <FontIcon Glyph="&#xE8B7;" FontSize="12"
                      Foreground="{StaticResource TertiaryTextBrush}" />
            <TextBlock x:Name="BackupDirText" FontSize="11"
                       FontFamily="{StaticResource MonoFont}"
                       Foreground="{StaticResource TertiaryTextBrush}" />
        </StackPanel>
    </Grid>
</Page>
```

- [ ] **Step 2: Create BackupsPage.xaml.cs**

```csharp
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Navigation;
using System.Text.Json.Nodes;
using Configonaut.Models;
using Configonaut.Services;

namespace Configonaut.Views;

public sealed partial class BackupsPage : Page
{
    private ConfigManager _config = null!;

    public BackupsPage()
    {
        this.InitializeComponent();
    }

    protected override void OnNavigatedTo(NavigationEventArgs e)
    {
        if (e.Parameter is ConfigManager config)
        {
            _config = config;
            _config.PropertyChanged += Config_PropertyChanged;
            RefreshUI();
        }
    }

    protected override void OnNavigatedFrom(NavigationEventArgs e)
    {
        if (_config is not null)
            _config.PropertyChanged -= Config_PropertyChanged;
    }

    private void Config_PropertyChanged(object? sender, System.ComponentModel.PropertyChangedEventArgs e)
    {
        DispatcherQueue.TryEnqueue(RefreshUI);
    }

    private void RefreshUI()
    {
        var hasBackups = _config.BackupFiles.Count > 0;
        EmptyState.Visibility = hasBackups ? Visibility.Collapsed : Visibility.Visible;
        ContentGrid.Visibility = hasBackups ? Visibility.Visible : Visibility.Collapsed;
        BackupList.ItemsSource = _config.BackupFiles;
        BackupDirText.Text = PathResolver.ShortenPath(_config.BackupDir);
    }

    private void BackupList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (BackupList.SelectedItem is BackupFile backup)
            ShowPreview(backup);
        else
            HidePreview();
    }

    private void ShowPreview(BackupFile backup)
    {
        NoSelectionText.Visibility = Visibility.Collapsed;
        PreviewHeader.Visibility = Visibility.Visible;
        PreviewScroll.Visibility = Visibility.Visible;

        PreviewDate.Text = backup.FormattedDate;
        PreviewSize.Text = backup.FormattedSize;

        var content = _config.ReadBackupContent(backup);
        PreviewContent.Text = content ?? "(unable to read)";

        // Compute diff
        ComputeDiff(backup);
    }

    private void HidePreview()
    {
        NoSelectionText.Visibility = Visibility.Visible;
        PreviewHeader.Visibility = Visibility.Collapsed;
        PreviewScroll.Visibility = Visibility.Collapsed;
        DiffSummary.Visibility = Visibility.Collapsed;
    }

    private void ComputeDiff(BackupFile backup)
    {
        var backups = _config.BackupFiles.ToList();
        var idx = backups.FindIndex(b => b.FileName == backup.FileName);

        // Get this backup's servers
        var thisContent = _config.ReadBackupContent(backup);
        var thisServers = GetServerKeys(thisContent);

        // Get the next-newer backup's servers (or current config if this is newest)
        HashSet<string> newerServers;
        if (idx == 0)
        {
            // This is the newest backup — compare against current config
            var configContent = File.Exists(_config.ConfigPath) ? File.ReadAllText(_config.ConfigPath) : null;
            newerServers = GetServerKeys(configContent);
        }
        else
        {
            var newerBackup = backups[idx - 1];
            var newerContent = _config.ReadBackupContent(newerBackup);
            newerServers = GetServerKeys(newerContent);
        }

        var added = newerServers.Except(thisServers).Select(n => new DiffItem("＋", n)).ToList();
        var removed = thisServers.Except(newerServers).Select(n => new DiffItem("－", n)).ToList();
        var diffItems = removed.Concat(added).ToList();

        if (diffItems.Count > 0)
        {
            DiffSummary.Visibility = Visibility.Visible;
            DiffItems.ItemsSource = diffItems;
        }
        else
        {
            DiffSummary.Visibility = Visibility.Collapsed;
        }
    }

    private static HashSet<string> GetServerKeys(string? json)
    {
        if (string.IsNullOrEmpty(json)) return new HashSet<string>();
        try
        {
            var obj = JsonHelper.ParseJsonObject(json);
            if (obj?["mcpServers"] is JsonObject servers)
                return servers.Select(k => k.Key).ToHashSet();
        }
        catch { }
        return new HashSet<string>();
    }

    private async void RestoreBackup_Click(object sender, RoutedEventArgs e)
    {
        if (sender is Button btn && btn.Tag is BackupFile backup)
        {
            var dialog = new ContentDialog
            {
                Title = "Restore Backup",
                Content = $"Restore config from {backup.FormattedDate}? Current config will be backed up first.",
                PrimaryButtonText = "Restore",
                CloseButtonText = "Cancel",
                XamlRoot = this.XamlRoot
            };

            if (await dialog.ShowAsync() == ContentDialogResult.Primary)
            {
                _config.RestoreBackup(backup);
                _config.LoadBackups();
            }
        }
    }

    private void DeleteBackup_Click(object sender, RoutedEventArgs e)
    {
        if (sender is Button btn && btn.Tag is BackupFile backup)
        {
            _config.DeleteBackup(backup);
        }
    }
}

public record DiffItem(string Icon, string Name);
```

- [ ] **Step 3: Verify it compiles**

```bash
dotnet build --configuration Debug
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: add BackupsPage with backup list, preview, diff, and restore"
```

---

### Task 14: App Entry Point & Packaging

**Files:**
- Create: `Configonaut/App.xaml`
- Create: `Configonaut/App.xaml.cs`
- Create: `Configonaut/app.manifest`
- Create: `Configonaut/Package.appxmanifest`

- [ ] **Step 1: Create App.xaml**

```xml
<Application
    x:Class="Configonaut.App"
    xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
    RequestedTheme="Dark">
    <Application.Resources>
        <ResourceDictionary>
            <ResourceDictionary.MergedDictionaries>
                <XamlControlsResources xmlns="using:Microsoft.UI.Xaml.Controls" />
                <ResourceDictionary Source="Theme/NeonTheme.xaml" />
            </ResourceDictionary.MergedDictionaries>
        </ResourceDictionary>
    </Application.Resources>
</Application>
```

- [ ] **Step 2: Create App.xaml.cs**

```csharp
using Microsoft.UI.Xaml;

namespace Configonaut;

public partial class App : Application
{
    private Window? _window;

    public App()
    {
        this.InitializeComponent();
    }

    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        _window = new MainWindow();
        _window.Activate();
    }
}
```

- [ ] **Step 3: Create app.manifest**

```xml
<?xml version="1.0" encoding="utf-8"?>
<assembly manifestVersion="1.0" xmlns="urn:schemas-microsoft-com:asm.v1">
  <assemblyIdentity version="1.0.0.0" name="Configonaut" />
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}" />
    </application>
  </compatibility>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
    </windowsSettings>
  </application>
</assembly>
```

- [ ] **Step 4: Create Package.appxmanifest**

```xml
<?xml version="1.0" encoding="utf-8"?>
<Package
  xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
  xmlns:mp="http://schemas.microsoft.com/appx/2014/phone/manifest"
  xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
  xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"
  IgnorableNamespaces="uap rescap">

  <Identity
    Name="Configonaut"
    Publisher="CN=Configonaut"
    Version="1.0.0.0" />

  <mp:PhoneIdentity PhoneProductId="00000000-0000-0000-0000-000000000000"
                    PhonePublisherId="00000000-0000-0000-0000-000000000000"/>

  <Properties>
    <DisplayName>Configonaut</DisplayName>
    <PublisherDisplayName>Configonaut</PublisherDisplayName>
    <Logo>Assets\AppIcon.png</Logo>
  </Properties>

  <Dependencies>
    <TargetDeviceFamily Name="Windows.Desktop"
                        MinVersion="10.0.19041.0"
                        MaxVersionTested="10.0.22621.0" />
  </Dependencies>

  <Resources>
    <Resource Language="x-generate"/>
  </Resources>

  <Applications>
    <Application Id="App"
      Executable="$targetnametoken$.exe"
      EntryPoint="$targetentrypoint$">
      <uap:VisualElements
        DisplayName="Configonaut"
        Description="Claude AI Configuration Manager"
        BackgroundColor="transparent"
        Square150x150Logo="Assets\AppIcon.png"
        Square44x44Logo="Assets\AppIcon.png">
        <uap:DefaultTile Wide310x150Logo="Assets\AppIcon.png" />
      </uap:VisualElements>
    </Application>
  </Applications>

  <Capabilities>
    <rescap:Capability Name="runFullTrust" />
  </Capabilities>
</Package>
```

- [ ] **Step 5: Copy AppIcon.png from macOS project**

```bash
cp /Users/aaronellis/Configonaut/Resources/AppIcon.png Configonaut/Assets/AppIcon.png
```

- [ ] **Step 6: Verify full solution builds**

```bash
dotnet build --configuration Release
```

Expected: Build succeeds with no errors.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat: add App entry point, manifest, and MSIX packaging config"
```

- [ ] **Step 8: Final build and run verification**

```bash
dotnet run --project Configonaut
```

Expected: App launches with sidebar, mode toggle, and all pages navigable. MCP servers page shows current config (or empty state if no Claude config exists).

- [ ] **Step 9: Final commit**

```bash
git add -A
git commit -m "chore: Configonaut Windows v1.0.0 — initial release"
```

---

## Self-Review Checklist

### Spec coverage

| Spec Requirement | Task |
|---|---|
| Desktop/CLI toggle with persistence | Task 5 (ConfigManager.Mode + ApplicationData) |
| MCP server management (CRUD, drag-drop) | Task 9 (MCPPage) |
| Hooks viewer/editor with toggle | Task 10 (HooksPage) |
| Agents viewer with plugin grouping | Task 11 (AgentsPage) |
| Skills with enable/disable and create | Task 12 (SkillsPage) |
| Backups with restore and diff | Task 13 (BackupsPage) |
| MSIX path detection | Task 3 (PathResolver.IsMsixInstall) |
| Dark theme with neon accents | Task 7 (NeonTheme.xaml) |
| MVVM pattern | Task 5-6 (ConfigManager as ObservableObject) |
| Windows config paths | Task 3 (PathResolver) |
| Migration for existing users | Task 5 (ConfigManager.MigrateIfNeeded) |
| Restart banner | Task 9 (MCPPage restart banner) |
| Input parser for various JSON formats | Task 6 (ParseInput/TryClassify) |
| Atomic file writes | Task 4 (JsonHelper temp-then-rename) |
| Backup deduplication (hash) | Task 6 (CreateBackup hash check) |
| Backup debounce (5 min) | Task 6 (CreateBackup time check) |
| 30 backup limit with pruning | Task 6 (WriteBackup prune logic) |
| MSIX packaging | Task 14 (Package.appxmanifest) |

### Placeholder scan
No TBD, TODO, or "implement later" found. All code is complete.

### Type consistency
- `ServerEntry` record used consistently across ConfigManager and MCPPage
- `HookRule` class used in ConfigManager and HooksPage
- `AgentEntry` / `SkillEntry` classes used consistently
- `ParseResult` abstract record with three subtypes used in ConfigManager and MCPPage
- `AppMode` enum + extensions used consistently
