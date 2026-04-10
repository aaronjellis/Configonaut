using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Navigation;
using Windows.ApplicationModel.DataTransfer;
using Configonaut.Models;
using Configonaut.Services;

namespace Configonaut.Views;

public sealed partial class MCPPage : Page
{
    private ConfigManager _config = null!;
    private ServerEntry? _selectedServer;
    private ServerSource _selectedSource;
    private bool _isAddMode;
    private bool _needsName;

    public MCPPage()
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
        ActiveList.ItemsSource = _config.ActiveServers;
        StoredList.ItemsSource = _config.StoredServers;
        ActiveCount.Text = $"({_config.ActiveServers.Count})";
        StoredCount.Text = $"({_config.StoredServers.Count})";
        CountSummary.Text = $"{_config.ActiveServers.Count} active, {_config.StoredServers.Count} inactive";
        StatusText.Text = _config.StatusMessage;
        StatusDot.Fill = _config.StatusIsError
            ? (Microsoft.UI.Xaml.Media.SolidColorBrush)Application.Current.Resources["NeonRedBrush"]
            : (Microsoft.UI.Xaml.Media.SolidColorBrush)Application.Current.Resources["NeonGreenBrush"];
        ConfigPathText.Text = PathResolver.ShortenPath(_config.ConfigPath);

        RestartBanner.Visibility = _config.NeedsRestart ? Visibility.Visible : Visibility.Collapsed;
        RestartText.Text = $"Changes saved! Quit and reopen {(_config.Mode == AppMode.Desktop ? "Claude Desktop" : "Claude Code")} to apply.";
    }

    // --- Sidebar selection ---

    private void ActiveList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (ActiveList.SelectedItem is ServerEntry server)
        {
            StoredList.SelectedItem = null;
            ShowDetail(server, ServerSource.Active);
        }
    }

    private void StoredList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (StoredList.SelectedItem is ServerEntry server)
        {
            ActiveList.SelectedItem = null;
            ShowDetail(server, ServerSource.Stored);
        }
    }

    private void ShowDetail(ServerEntry server, ServerSource source)
    {
        _selectedServer = server;
        _selectedSource = source;
        _isAddMode = false;

        DetailPanel.Visibility = Visibility.Visible;
        DetailHeader.Visibility = Visibility.Visible;
        AddHeader.Visibility = Visibility.Collapsed;
        NameFieldPanel.Visibility = Visibility.Collapsed;

        DetailServerName.Text = server.Name;
        DetailSourceText.Text = source == ServerSource.Active ? "Active" : "Inactive";
        DetailSourceBadge.Background = source == ServerSource.Active
            ? (Microsoft.UI.Xaml.Media.SolidColorBrush)Application.Current.Resources["NeonGreenBrush"]
            : (Microsoft.UI.Xaml.Media.SolidColorBrush)Application.Current.Resources["NeonRedBrush"];

        JsonEditor.Text = server.ConfigJson;

        // Show detail buttons, hide add buttons
        BtnSave.Visibility = Visibility.Visible;
        BtnToggle.Visibility = Visibility.Visible;
        BtnCopy.Visibility = Visibility.Visible;
        BtnDelete.Visibility = Visibility.Visible;
        BtnCancel.Visibility = Visibility.Collapsed;
        BtnSaveOnly.Visibility = Visibility.Collapsed;
        BtnTurnOn.Visibility = Visibility.Collapsed;

        BtnToggle.Content = source == ServerSource.Active ? "Turn Off" : "Turn On";
        BtnToggle.Background = source == ServerSource.Active
            ? (Microsoft.UI.Xaml.Media.SolidColorBrush)Application.Current.Resources["NeonRedBrush"]
            : (Microsoft.UI.Xaml.Media.SolidColorBrush)Application.Current.Resources["NeonGreenBrush"];
        BtnToggle.Foreground = source == ServerSource.Active
            ? new Microsoft.UI.Xaml.Media.SolidColorBrush(Microsoft.UI.Colors.White)
            : new Microsoft.UI.Xaml.Media.SolidColorBrush(Microsoft.UI.Colors.Black);
    }

    // --- Add server ---

    private void AddServer_Click(object sender, RoutedEventArgs e)
    {
        _isAddMode = true;
        _selectedServer = null;
        ActiveList.SelectedItem = null;
        StoredList.SelectedItem = null;

        DetailPanel.Visibility = Visibility.Visible;
        DetailHeader.Visibility = Visibility.Collapsed;
        AddHeader.Visibility = Visibility.Visible;
        NameFieldPanel.Visibility = Visibility.Collapsed;

        JsonEditor.Text = "";
        ValidationBadge.Visibility = Visibility.Collapsed;

        // Show add buttons, hide detail buttons
        BtnSave.Visibility = Visibility.Collapsed;
        BtnToggle.Visibility = Visibility.Collapsed;
        BtnCopy.Visibility = Visibility.Collapsed;
        BtnDelete.Visibility = Visibility.Collapsed;
        BtnCancel.Visibility = Visibility.Visible;
        BtnSaveOnly.Visibility = Visibility.Visible;
        BtnTurnOn.Visibility = Visibility.Visible;
        BtnSaveOnly.IsEnabled = false;
        BtnTurnOn.IsEnabled = false;
    }

    private void JsonEditor_TextChanged(object sender, TextChangedEventArgs e)
    {
        if (!_isAddMode) return;

        var text = JsonEditor.Text;
        if (string.IsNullOrWhiteSpace(text))
        {
            ValidationBadge.Visibility = Visibility.Collapsed;
            BtnSaveOnly.IsEnabled = false;
            BtnTurnOn.IsEnabled = false;
            NameFieldPanel.Visibility = Visibility.Collapsed;
            return;
        }

        var result = _config.ParseInput(text);
        ValidationBadge.Visibility = Visibility.Visible;

        switch (result)
        {
            case ParseResult.Servers servers:
                ValidationText.Text = $"Ready to add ({servers.Entries.Count})";
                ValidationBadge.Background = (Microsoft.UI.Xaml.Media.SolidColorBrush)Application.Current.Resources["NeonGreenBrush"];
                BtnSaveOnly.IsEnabled = true;
                BtnTurnOn.IsEnabled = true;
                NameFieldPanel.Visibility = Visibility.Collapsed;
                _needsName = false;
                break;
            case ParseResult.NeedsName:
                ValidationText.Text = "Enter a server name";
                ValidationBadge.Background = (Microsoft.UI.Xaml.Media.SolidColorBrush)Application.Current.Resources["NeonAmberBrush"];
                BtnSaveOnly.IsEnabled = !string.IsNullOrWhiteSpace(ServerNameField.Text);
                BtnTurnOn.IsEnabled = !string.IsNullOrWhiteSpace(ServerNameField.Text);
                NameFieldPanel.Visibility = Visibility.Visible;
                _needsName = true;
                break;
            case ParseResult.Error err:
                ValidationText.Text = err.Message;
                ValidationBadge.Background = (Microsoft.UI.Xaml.Media.SolidColorBrush)Application.Current.Resources["NeonRedBrush"];
                BtnSaveOnly.IsEnabled = false;
                BtnTurnOn.IsEnabled = false;
                NameFieldPanel.Visibility = Visibility.Collapsed;
                _needsName = false;
                break;
        }
    }

    private List<ServerEntry>? GetParsedEntries()
    {
        var result = _config.ParseInput(JsonEditor.Text);
        return result switch
        {
            ParseResult.Servers s => s.Entries,
            ParseResult.NeedsName n when !string.IsNullOrWhiteSpace(ServerNameField.Text) =>
                new List<ServerEntry> { new(ServerNameField.Text.Trim(), n.ConfigJson) },
            _ => null
        };
    }

    private void TurnOn_Click(object sender, RoutedEventArgs e)
    {
        var entries = GetParsedEntries();
        if (entries is null) return;
        _config.AddToActive(entries);
        CloseAddPanel();
    }

    private void SaveOnly_Click(object sender, RoutedEventArgs e)
    {
        var entries = GetParsedEntries();
        if (entries is null) return;
        _config.AddToStored(entries);
        CloseAddPanel();
    }

    // --- Detail actions ---

    private void SaveServer_Click(object sender, RoutedEventArgs e)
    {
        if (_selectedServer is null) return;
        _config.UpdateServerConfig(_selectedServer.Name, _selectedSource, JsonEditor.Text);
    }

    private void ToggleServer_Click(object sender, RoutedEventArgs e)
    {
        if (_selectedServer is null) return;
        if (_selectedSource == ServerSource.Active)
            _config.MoveToStored(_selectedServer.Name);
        else
            _config.MoveToActive(_selectedServer.Name);
        CloseDetailPanel();
    }

    private void CopyJson_Click(object sender, RoutedEventArgs e)
    {
        var dp = new DataPackage();
        dp.SetText(JsonEditor.Text);
        Clipboard.SetContent(dp);
    }

    private async void DeleteServer_Click(object sender, RoutedEventArgs e)
    {
        if (_selectedServer is null) return;

        var dialog = new ContentDialog
        {
            Title = "Delete Server",
            Content = $"Delete '{_selectedServer.Name}'? This cannot be undone.",
            PrimaryButtonText = "Delete",
            CloseButtonText = "Cancel",
            XamlRoot = this.XamlRoot
        };

        if (await dialog.ShowAsync() == ContentDialogResult.Primary)
        {
            _config.DeleteServer(_selectedServer.Name, _selectedSource);
            CloseDetailPanel();
        }
    }

    private void DismissRestart_Click(object sender, RoutedEventArgs e) =>
        _config.NeedsRestart = false;

    private void CloseDetail_Click(object sender, RoutedEventArgs e) => CloseDetailPanel();
    private void CloseAdd_Click(object sender, RoutedEventArgs e) => CloseAddPanel();

    private void CloseDetailPanel()
    {
        _selectedServer = null;
        ActiveList.SelectedItem = null;
        StoredList.SelectedItem = null;
        DetailPanel.Visibility = Visibility.Collapsed;
    }

    private void CloseAddPanel()
    {
        _isAddMode = false;
        JsonEditor.Text = "";
        ServerNameField.Text = "";
        DetailPanel.Visibility = Visibility.Collapsed;
    }

    // --- Drag and drop ---

    private void ActiveList_DragStarting(object sender, DragItemsStartingEventArgs e)
    {
        if (e.Items.FirstOrDefault() is ServerEntry server)
        {
            e.Data.SetText($"active:{server.Name}");
            e.Data.RequestedOperation = DataPackageOperation.Move;
        }
    }

    private void StoredList_DragStarting(object sender, DragItemsStartingEventArgs e)
    {
        if (e.Items.FirstOrDefault() is ServerEntry server)
        {
            e.Data.SetText($"stored:{server.Name}");
            e.Data.RequestedOperation = DataPackageOperation.Move;
        }
    }

    private void ActiveColumn_DragOver(object sender, DragEventArgs e)
    {
        e.AcceptedOperation = DataPackageOperation.Move;
    }

    private async void ActiveColumn_Drop(object sender, DragEventArgs e)
    {
        if (e.DataView.Contains(StandardDataFormats.Text))
        {
            var text = await e.DataView.GetTextAsync();
            if (text.StartsWith("stored:"))
            {
                var name = text["stored:".Length..];
                _config.MoveToActive(name);
            }
        }
    }

    private void StoredColumn_DragOver(object sender, DragEventArgs e)
    {
        e.AcceptedOperation = DataPackageOperation.Move;
    }

    private async void StoredColumn_Drop(object sender, DragEventArgs e)
    {
        if (e.DataView.Contains(StandardDataFormats.Text))
        {
            var text = await e.DataView.GetTextAsync();
            if (text.StartsWith("active:"))
            {
                var name = text["active:".Length..];
                _config.MoveToStored(name);
            }
        }
    }
}
