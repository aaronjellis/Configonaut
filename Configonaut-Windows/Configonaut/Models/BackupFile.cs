namespace Configonaut.Models;

public class BackupFile
{
    public string FileName { get; }
    public DateTime Date { get; }
    public long SizeBytes { get; }
    public string Id => FileName;

    public string FormattedDate => Date.ToString("MMM d, yyyy h:mm tt");

    public string FormattedSize
    {
        get
        {
            if (SizeBytes < 1024) return $"{SizeBytes} B";
            if (SizeBytes < 1024 * 1024) return $"{SizeBytes / 1024.0:F1} KB";
            return $"{SizeBytes / (1024.0 * 1024.0):F1} MB";
        }
    }

    public BackupFile(string fileName, DateTime date, long sizeBytes)
    {
        FileName = fileName;
        Date = date;
        SizeBytes = sizeBytes;
    }
}
