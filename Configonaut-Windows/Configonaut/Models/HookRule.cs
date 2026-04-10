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
