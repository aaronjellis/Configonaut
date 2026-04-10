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
