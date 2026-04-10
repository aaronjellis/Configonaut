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
