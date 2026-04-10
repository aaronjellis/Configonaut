using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Navigation;
using Configonaut.Models;
using Configonaut.Services;

namespace Configonaut.Views;

public sealed partial class AgentsPage : Page
{
    private ConfigManager _config = null!;
    private AgentEntry? _selectedAgent;
    private bool _isCreateMode;
    private string _searchQuery = "";

    private const string AgentTemplate = "---\nname: {0}\ndescription: A custom agent\ntools: Read, Edit, Write, Bash, Glob, Grep\nmodel: sonnet\ncolor: blue\n---\n\nDescribe this agent's purpose and behavior here.\n";

    public AgentsPage()
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
            ? _config.Agents.ToList()
            : _config.Agents.Where(a =>
                a.Name.Contains(_searchQuery, StringComparison.OrdinalIgnoreCase) ||
                a.Description.Contains(_searchQuery, StringComparison.OrdinalIgnoreCase) ||
                a.PluginName.Contains(_searchQuery, StringComparison.OrdinalIgnoreCase)
            ).ToList();

        AgentList.ItemsSource = filtered;
        EmptyState.Visibility = filtered.Count == 0 ? Visibility.Visible : Visibility.Collapsed;
        AgentList.Visibility = filtered.Count > 0 ? Visibility.Visible : Visibility.Collapsed;
    }

    private void SearchBox_TextChanged(object sender, TextChangedEventArgs e)
    {
        _searchQuery = SearchBox.Text;
        RefreshUI();
    }

    private void AgentList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (AgentList.SelectedItem is AgentEntry agent)
            ShowDetailEditor(agent);
    }

    private void ShowDetailEditor(AgentEntry agent)
    {
        _selectedAgent = agent;
        _isCreateMode = false;

        EditorPanel.Visibility = Visibility.Visible;
        NamePanel.Visibility = Visibility.Collapsed;
        EditorTitle.Text = agent.Name;
        EditorSourceText.Text = agent.Source == AgentSource.Personal ? "Personal" : agent.PluginName;

        try { AgentEditor.Text = File.ReadAllText(agent.FilePath); }
        catch { AgentEditor.Text = ""; }

        BtnSaveAgent.Visibility = Visibility.Visible;
        BtnDeleteAgent.Visibility = agent.Source == AgentSource.Personal ? Visibility.Visible : Visibility.Collapsed;
        BtnTogglePlugin.Visibility = agent.Source == AgentSource.Plugin ? Visibility.Visible : Visibility.Collapsed;
        BtnCreateAgent.Visibility = Visibility.Collapsed;

        if (agent.Source == AgentSource.Plugin)
        {
            var pluginKey = $"{agent.PluginName}@claude-plugins-official";
            var isEnabled = _config.EnabledPlugins.TryGetValue(pluginKey, out var e2) && e2;
            BtnTogglePlugin.Content = isEnabled ? "Disable Plugin" : "Enable Plugin";
        }
    }

    private void NewAgent_Click(object sender, RoutedEventArgs e)
    {
        _isCreateMode = true;
        _selectedAgent = null;
        AgentList.SelectedItem = null;

        EditorPanel.Visibility = Visibility.Visible;
        NamePanel.Visibility = Visibility.Visible;
        EditorTitle.Text = "New Agent";
        EditorSourceText.Text = "Personal";
        AgentNameField.Text = "";
        AgentEditor.Text = string.Format(AgentTemplate, "my-agent");

        BtnSaveAgent.Visibility = Visibility.Collapsed;
        BtnDeleteAgent.Visibility = Visibility.Collapsed;
        BtnTogglePlugin.Visibility = Visibility.Collapsed;
        BtnCreateAgent.Visibility = Visibility.Visible;
    }

    private void AgentNameField_TextChanged(object sender, TextChangedEventArgs e)
    {
        var name = AgentNameField.Text.ToLowerInvariant().Replace(' ', '-');
        name = new string(name.Where(c => char.IsLetterOrDigit(c) || c == '-').ToArray());
        if (!string.IsNullOrEmpty(name))
            AgentEditor.Text = string.Format(AgentTemplate, name);
    }

    private void SaveAgent_Click(object sender, RoutedEventArgs e)
    {
        if (_selectedAgent is null) return;
        try
        {
            File.WriteAllText(_selectedAgent.FilePath, AgentEditor.Text);
            _config.LoadAgents();
        }
        catch { }
    }

    private void CreateAgent_Click(object sender, RoutedEventArgs e)
    {
        if (string.IsNullOrWhiteSpace(AgentNameField.Text)) return;
        var path = _config.CreateAgent(AgentNameField.Text);
        if (path is null) return;
        File.WriteAllText(path, AgentEditor.Text);
        _config.LoadAgents();
        CloseEditorPanel();
    }

    private async void DeleteAgent_Click(object sender, RoutedEventArgs e)
    {
        if (_selectedAgent is null) return;
        var dialog = new ContentDialog
        {
            Title = "Delete Agent",
            Content = $"Delete '{_selectedAgent.Name}'?",
            PrimaryButtonText = "Delete",
            CloseButtonText = "Cancel",
            XamlRoot = this.XamlRoot
        };
        if (await dialog.ShowAsync() == ContentDialogResult.Primary)
        {
            _config.DeleteAgent(_selectedAgent);
            CloseEditorPanel();
        }
    }

    private void TogglePlugin_Click(object sender, RoutedEventArgs e)
    {
        if (_selectedAgent is null) return;
        var pluginKey = $"{_selectedAgent.PluginName}@claude-plugins-official";
        _config.TogglePlugin(pluginKey);
        CloseEditorPanel();
    }

    private void CloseEditor_Click(object sender, RoutedEventArgs e) => CloseEditorPanel();

    private void CloseEditorPanel()
    {
        _selectedAgent = null;
        _isCreateMode = false;
        AgentList.SelectedItem = null;
        EditorPanel.Visibility = Visibility.Collapsed;
    }
}
