using System.Text.Json;
using System.Threading.Tasks;
using System.Windows.Threading;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using StatusShare.Wpf.Interop;

namespace StatusShare.Wpf.ViewModels;

public partial class MainWindowViewModel : ObservableObject
{
    private readonly DispatcherTimer _heartbeatTimer;

    [ObservableProperty]
    private string _baseUrl = "http://127.0.0.1:3000";

    [ObservableProperty]
    private string _token = string.Empty;

    [ObservableProperty]
    private string _heartbeatIntervalSecs = "60";

    [ObservableProperty]
    private bool _ok = true;

    [ObservableProperty]
    private string _process = string.Empty;

    [ObservableProperty]
    private string _extend = string.Empty;

    [ObservableProperty]
    private string _mediaTitle = string.Empty;

    [ObservableProperty]
    private string _mediaArtist = string.Empty;

    [ObservableProperty]
    private string _mediaThumbnail = string.Empty;

    [ObservableProperty]
    private string _output = "Ready.";

    public MainWindowViewModel()
    {
        _heartbeatTimer = new DispatcherTimer();
        _heartbeatTimer.Interval = TimeSpan.FromSeconds(60);
        _heartbeatTimer.Tick += async (_, _) => await PushOnceAsync();
    }

    [RelayCommand]
    private async Task FetchAsync()
    {
        Output = await Task.Run(() => FormatJson(StatusShareNative.FetchStatus(BuildConfigJson())));
    }

    [RelayCommand]
    private async Task PushAsync()
    {
        await PushOnceAsync();
    }

    [RelayCommand]
    private void StartHeartbeat()
    {
        _heartbeatTimer.Interval = TimeSpan.FromSeconds(ParseHeartbeatSeconds());
        _heartbeatTimer.Start();
        Output = "Heartbeat started.";
    }

    [RelayCommand]
    private void StopHeartbeat()
    {
        _heartbeatTimer.Stop();
        Output = "Heartbeat stopped.";
    }

    private async Task PushOnceAsync()
    {
        Output = await Task.Run(() => FormatJson(StatusShareNative.PushStatus(BuildConfigJson(), BuildUpdateJson())));
    }

    private string BuildConfigJson()
    {
        return JsonSerializer.Serialize(new
        {
            base_url = BaseUrl,
            token = Token,
            heartbeat_interval_secs = ParseHeartbeatSeconds(),
            user_agent = "StatusShare WPF/0.1.0"
        });
    }

    private string BuildUpdateJson()
    {
        var media = string.IsNullOrWhiteSpace(MediaTitle) &&
                    string.IsNullOrWhiteSpace(MediaArtist) &&
                    string.IsNullOrWhiteSpace(MediaThumbnail)
            ? null
            : new
            {
                title = MediaTitle,
                artist = MediaArtist,
                thumbnail = MediaThumbnail
            };

        return JsonSerializer.Serialize(new
        {
            ok = Ok ? 1 : 0,
            process = NullIfWhiteSpace(Process),
            extend = NullIfWhiteSpace(Extend),
            media,
            timestamp = (long?)null
        });
    }

    private int ParseHeartbeatSeconds()
    {
        return int.TryParse(HeartbeatIntervalSecs, out var value) && value >= 5 ? value : 60;
    }

    private static string? NullIfWhiteSpace(string value) => string.IsNullOrWhiteSpace(value) ? null : value;

    private static string FormatJson(string raw)
    {
        try
        {
            using var document = JsonDocument.Parse(raw);
            return JsonSerializer.Serialize(document, new JsonSerializerOptions
            {
                WriteIndented = true
            });
        }
        catch
        {
            return raw;
        }
    }
}

