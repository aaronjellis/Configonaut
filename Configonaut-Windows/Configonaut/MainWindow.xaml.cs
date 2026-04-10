using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Configonaut.Models;
using Configonaut.Services;
using Configonaut.Views;
using CommunityToolkit.WinUI.Controls;
using Windows.UI;

namespace Configonaut;

public sealed partial class MainWindow : Window
{
    public ConfigManager Config { get; }
    private string _currentSection = "Servers";

    public MainWindow()
    {
        this.InitializeComponent();

        // Set dark theme
        if (Content is FrameworkElement root)
            root.RequestedTheme = ElementTheme.Dark;

        // Set initial size
        var appWindow = this.AppWindow;
        appWindow.Resize(new Windows.Graphics.SizeInt32(960, 700));

        Config = new ConfigManager();

        // Set initial mode toggle state
        ModeToggle.SelectedIndex = Config.Mode == AppMode.Desktop ? 0 : 1;
        UpdateModeSubtitle();
        UpdateBadges();

        // Subscribe to property changes for badge updates
        Config.PropertyChanged += (s, e) => DispatcherQueue.TryEnqueue(UpdateBadges);

        // Navigate to default page
        NavigateToSection("Servers");
        UpdateSidebarSelection();
    }

    private void ModeToggle_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (ModeToggle.SelectedItem is SegmentedItem item && item.Tag is string tag)
        {
            Config.Mode = tag == "CLI" ? AppMode.Cli : AppMode.Desktop;
            UpdateModeSubtitle();
            UpdateBadges();
            NavigateToSection(_currentSection);
        }
    }

    private void SidebarItem_Click(object sender, RoutedEventArgs e)
    {
        if (sender is Button btn && btn.Tag is string section)
        {
            _currentSection = section;
            NavigateToSection(section);
            UpdateSidebarSelection();
        }
    }

    private void NavigateToSection(string section)
    {
        switch (section)
        {
            case "Servers":
                ContentFrame.Navigate(typeof(MCPPage), Config);
                break;
            case "Hooks":
                ContentFrame.Navigate(typeof(HooksPage), Config);
                break;
            case "Agents":
                ContentFrame.Navigate(typeof(AgentsPage), Config);
                break;
            case "Skills":
                ContentFrame.Navigate(typeof(SkillsPage), Config);
                break;
            case "Backups":
                ContentFrame.Navigate(typeof(BackupsPage), Config);
                break;
        }
    }

    private void UpdateModeSubtitle()
    {
        ModeSubtitle.Text = Config.Mode == AppMode.Desktop ? "Desktop Config" : "CLI Config";
        ModeSubtitle.Foreground = Config.Mode == AppMode.Desktop
            ? (SolidColorBrush)Application.Current.Resources["NeonGreenBrush"]
            : (SolidColorBrush)Application.Current.Resources["NeonBlueBrush"];
    }

    private void UpdateBadges()
    {
        SetBadge(BadgeServers, BadgeServersCount, Config.ActiveServers.Count);
        SetBadge(BadgeHooks, BadgeHooksCount, Config.HookRules.Count);
        SetBadge(BadgeAgents, BadgeAgentsCount, Config.Agents.Count);
        SetBadge(BadgeSkills, BadgeSkillsCount, Config.Skills.Count);
        SetBadge(BadgeBackups, BadgeBackupsCount, Config.BackupFiles.Count);
    }

    private static void SetBadge(Border border, TextBlock text, int count)
    {
        border.Visibility = count > 0 ? Visibility.Visible : Visibility.Collapsed;
        text.Text = count.ToString();
    }

    private void UpdateSidebarSelection()
    {
        var buttons = new[] { BtnServers, BtnHooks, BtnAgents, BtnSkills, BtnBackups };
        var sections = new[] { "Servers", "Hooks", "Agents", "Skills", "Backups" };

        for (int i = 0; i < buttons.Length; i++)
        {
            var isSelected = sections[i] == _currentSection;
            buttons[i].Background = isSelected
                ? new SolidColorBrush(Color.FromArgb(0x14, 0xFF, 0xFF, 0xFF))
                : new SolidColorBrush(Colors.Transparent);
        }
    }
}
