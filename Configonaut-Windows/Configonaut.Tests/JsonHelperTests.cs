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
