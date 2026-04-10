using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Navigation;
using CommunityToolkit.WinUI.Controls;
using Configonaut.Models;
using Configonaut.Services;

namespace Configonaut.Views;

public sealed partial class SkillsPage : Page
{
    private ConfigManager _config = null!;
    private SkillEntry? _selectedSkill;
    private bool _isCreateMode;
    private string _searchQuery = "";
    private SkillSource _newSkillType = SkillSource.Command;

    private const string CommandTemplate = "---\nname: {0}\ndescription: A custom command\n---\n\nDescribe what this command does.\n";
    private const string SkillTemplate = "---\nname: {0}\ndescription: A custom skill\n---\n\nDescribe what this skill does.\n";

    public SkillsPage()
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
        var filtered = string.IsNullOrEmpty(_searchQuery)
            ? _config.Skills.ToList()
            : _config.Skills.Where(s =>
                s.Name.Contains(_searchQuery, StringComparison.OrdinalIgnoreCase) ||
                s.Description.Contains(_searchQuery, StringComparison.OrdinalIgnoreCase)
            ).ToList();

        SkillList.ItemsSource = filtered;
        EmptyState.Visibility = filtered.Count == 0 ? Visibility.Visible : Visibility.Collapsed;
        SkillList.Visibility = filtered.Count > 0 ? Visibility.Visible : Visibility.Collapsed;
    }

    private void SearchBox_TextChanged(object sender, TextChangedEventArgs e)
    {
        _searchQuery = SearchBox.Text;
        RefreshUI();
    }

    private void SkillList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (SkillList.SelectedItem is SkillEntry skill)
            ShowDetailEditor(skill);
    }

    private void ShowDetailEditor(SkillEntry skill)
    {
        _selectedSkill = skill;
        _isCreateMode = false;

        EditorPanel.Visibility = Visibility.Visible;
        CreatePanel.Visibility = Visibility.Collapsed;
        EditorTitle.Text = skill.Name;
        EditorSourceText.Text = skill.SourceLabel;

        try { SkillEditor.Text = File.ReadAllText(skill.FilePath); }
        catch { SkillEditor.Text = ""; }

        BtnSaveSkill.Visibility = Visibility.Visible;
        BtnToggleSkill.Visibility = skill.Source != SkillSource.Plugin ? Visibility.Visible : Visibility.Collapsed;
        BtnCreateSkill.Visibility = Visibility.Collapsed;

        if (skill.Source != SkillSource.Plugin)
        {
            BtnToggleSkill.Content = skill.IsEnabled ? "Disable" : "Enable";
            BtnToggleSkill.Background = skill.IsEnabled
                ? (Microsoft.UI.Xaml.Media.SolidColorBrush)Application.Current.Resources["NeonRedBrush"]
                : (Microsoft.UI.Xaml.Media.SolidColorBrush)Application.Current.Resources["NeonGreenBrush"];
            BtnToggleSkill.Foreground = skill.IsEnabled
                ? new Microsoft.UI.Xaml.Media.SolidColorBrush(Microsoft.UI.Colors.White)
                : new Microsoft.UI.Xaml.Media.SolidColorBrush(Microsoft.UI.Colors.Black);
        }
    }

    private void NewSkill_Click(object sender, RoutedEventArgs e)
    {
        _isCreateMode = true;
        _selectedSkill = null;
        SkillList.SelectedItem = null;

        EditorPanel.Visibility = Visibility.Visible;
        CreatePanel.Visibility = Visibility.Visible;
        EditorTitle.Text = "New Skill";
        EditorSourceText.Text = "";
        SkillNameField.Text = "";
        TypeToggle.SelectedIndex = 0;
        _newSkillType = SkillSource.Command;
        SkillEditor.Text = string.Format(CommandTemplate, "my-command");

        BtnSaveSkill.Visibility = Visibility.Collapsed;
        BtnToggleSkill.Visibility = Visibility.Collapsed;
        BtnCreateSkill.Visibility = Visibility.Visible;
    }

    private void SkillNameField_TextChanged(object sender, TextChangedEventArgs e) => UpdateCreateTemplate();

    private void TypeToggle_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (TypeToggle.SelectedItem is SegmentedItem item && item.Tag is string tag)
        {
            _newSkillType = tag == "Skill" ? SkillSource.Skill : SkillSource.Command;
            UpdateCreateTemplate();
        }
    }

    private void UpdateCreateTemplate()
    {
        var name = SkillNameField.Text.ToLowerInvariant().Replace(' ', '-');
        name = new string(name.Where(c => char.IsLetterOrDigit(c) || c == '-').ToArray());
        if (string.IsNullOrEmpty(name)) name = _newSkillType == SkillSource.Command ? "my-command" : "my-skill";

        SkillEditor.Text = _newSkillType == SkillSource.Command
            ? string.Format(CommandTemplate, name)
            : string.Format(SkillTemplate, name);
    }

    private void SaveSkill_Click(object sender, RoutedEventArgs e)
    {
        if (_selectedSkill is null) return;
        try
        {
            File.WriteAllText(_selectedSkill.FilePath, SkillEditor.Text);
            _config.LoadSkills();
        }
        catch { }
    }

    private void ToggleSkill_Click(object sender, RoutedEventArgs e)
    {
        if (_selectedSkill is null) return;
        _config.ToggleSkill(_selectedSkill);
        CloseEditorPanel();
    }

    private void CreateSkill_Click(object sender, RoutedEventArgs e)
    {
        if (string.IsNullOrWhiteSpace(SkillNameField.Text)) return;
        var path = _config.CreateSkill(SkillNameField.Text, _newSkillType);
        if (path is null) return;
        File.WriteAllText(path, SkillEditor.Text);
        _config.LoadSkills();
        CloseEditorPanel();
    }

    private void CloseEditor_Click(object sender, RoutedEventArgs e) => CloseEditorPanel();

    private void CloseEditorPanel()
    {
        _selectedSkill = null;
        _isCreateMode = false;
        SkillList.SelectedItem = null;
        EditorPanel.Visibility = Visibility.Collapsed;
    }
}
