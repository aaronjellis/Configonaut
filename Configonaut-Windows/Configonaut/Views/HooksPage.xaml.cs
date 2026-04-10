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
