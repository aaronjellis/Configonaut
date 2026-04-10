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
