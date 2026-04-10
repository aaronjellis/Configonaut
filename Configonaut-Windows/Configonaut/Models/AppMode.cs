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
