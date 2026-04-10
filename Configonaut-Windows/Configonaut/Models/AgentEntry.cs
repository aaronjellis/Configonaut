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
