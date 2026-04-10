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
