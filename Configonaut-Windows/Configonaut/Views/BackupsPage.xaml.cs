using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Navigation;
using System.Text.Json.Nodes;
using Configonaut.Models;
using Configonaut.Services;

namespace Configonaut.Views;

public sealed partial class BackupsPage : Page
{
    private ConfigManager _config = null!;

    public BackupsPage()
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
        var hasBackups = _config.BackupFiles.Count > 0;
        EmptyState.Visibility = hasBackups ? Visibility.Collapsed : Visibility.Visible;
        ContentGrid.Visibility = hasBackups ? Visibility.Visible : Visibility.Collapsed;
        BackupList.ItemsSource = _config.BackupFiles;
        BackupDirText.Text = PathResolver.ShortenPath(_config.BackupDir);
    }

    private void BackupList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (BackupList.SelectedItem is BackupFile backup)
            ShowPreview(backup);
        else
            HidePreview();
    }

    private void ShowPreview(BackupFile backup)
    {
        NoSelectionText.Visibility = Visibility.Collapsed;
        PreviewHeader.Visibility = Visibility.Visible;
        PreviewScroll.Visibility = Visibility.Visible;

        PreviewDate.Text = backup.FormattedDate;
        PreviewSize.Text = backup.FormattedSize;

        var content = _config.ReadBackupContent(backup);
        PreviewContent.Text = content ?? "(unable to read)";

        ComputeDiff(backup);
    }

    private void HidePreview()
    {
        NoSelectionText.Visibility = Visibility.Visible;
        PreviewHeader.Visibility = Visibility.Collapsed;
        PreviewScroll.Visibility = Visibility.Collapsed;
        DiffSummary.Visibility = Visibility.Collapsed;
    }

    private void ComputeDiff(BackupFile backup)
    {
        var backups = _config.BackupFiles.ToList();
        var idx = backups.FindIndex(b => b.FileName == backup.FileName);

        var thisContent = _config.ReadBackupContent(backup);
        var thisServers = GetServerKeys(thisContent);

        HashSet<string> newerServers;
        if (idx == 0)
        {
            var configContent = File.Exists(_config.ConfigPath) ? File.ReadAllText(_config.ConfigPath) : null;
            newerServers = GetServerKeys(configContent);
        }
        else
        {
            var newerBackup = backups[idx - 1];
            var newerContent = _config.ReadBackupContent(newerBackup);
            newerServers = GetServerKeys(newerContent);
        }

        var added = newerServers.Except(thisServers).Select(n => new DiffItem("＋", n)).ToList();
        var removed = thisServers.Except(newerServers).Select(n => new DiffItem("－", n)).ToList();
        var diffItems = removed.Concat(added).ToList();

        if (diffItems.Count > 0)
        {
            DiffSummary.Visibility = Visibility.Visible;
            DiffItems.ItemsSource = diffItems;
        }
        else
        {
            DiffSummary.Visibility = Visibility.Collapsed;
        }
    }

    private static HashSet<string> GetServerKeys(string? json)
    {
        if (string.IsNullOrEmpty(json)) return new HashSet<string>();
        try
        {
            var obj = JsonHelper.ParseJsonObject(json);
            if (obj?["mcpServers"] is JsonObject servers)
                return servers.Select(k => k.Key).ToHashSet();
        }
        catch { }
        return new HashSet<string>();
    }

    private async void RestoreBackup_Click(object sender, RoutedEventArgs e)
    {
        if (sender is Button btn && btn.Tag is BackupFile backup)
        {
            var dialog = new ContentDialog
            {
                Title = "Restore Backup",
                Content = $"Restore config from {backup.FormattedDate}? Current config will be backed up first.",
                PrimaryButtonText = "Restore",
                CloseButtonText = "Cancel",
                XamlRoot = this.XamlRoot
            };

            if (await dialog.ShowAsync() == ContentDialogResult.Primary)
            {
                _config.RestoreBackup(backup);
                _config.LoadBackups();
            }
        }
    }

    private void DeleteBackup_Click(object sender, RoutedEventArgs e)
    {
        if (sender is Button btn && btn.Tag is BackupFile backup)
        {
            _config.DeleteBackup(backup);
        }
    }
}

public record DiffItem(string Icon, string Name);
