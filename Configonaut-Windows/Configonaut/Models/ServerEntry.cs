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
