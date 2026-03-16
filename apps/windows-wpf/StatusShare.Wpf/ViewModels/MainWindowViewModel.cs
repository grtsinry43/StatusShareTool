using System.Collections.ObjectModel;
using System.Text.Json;
using System.Windows.Threading;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using StatusShare.WindowsApp.Interop;
using StatusShare.WindowsApp.Models;
using StatusShare.WindowsApp.Services;

namespace StatusShare.WindowsApp.ViewModels;

public partial class MainWindowViewModel : ObservableObject
{
    private readonly DispatcherTimer _monitorTimer;
    private readonly WindowDetectionService _windowDetectionService = new();
    private readonly MediaDetectionService _mediaDetectionService = new();
    private bool _tickRunning;
    private SchedulerSnapshotDto _schedulerSnapshot = new();

    public ObservableCollection<WindowMatchRuleModel> Rules { get; } = [];
    public IReadOnlyList<MatchField> MatchFields { get; } = Enum.GetValues<MatchField>();
    public IReadOnlyList<MatchKind> MatchKinds { get; } = Enum.GetValues<MatchKind>();
    public IReadOnlyList<ReportPolicy> ReportPolicies { get; } = Enum.GetValues<ReportPolicy>();

    [ObservableProperty] private string _configPath = string.Empty;
    [ObservableProperty] private string _baseUrl = "http://127.0.0.1:3000";
    [ObservableProperty] private string _token = string.Empty;
    [ObservableProperty] private int _heartbeatIntervalSecs = 10;
    [ObservableProperty] private bool _defaultReport = true;
    [ObservableProperty] private string _defaultDisplayName = string.Empty;
    [ObservableProperty] private string _defaultExtend = string.Empty;
    [ObservableProperty] private string _monitorStatus = "已停止";
    [ObservableProperty] private string _backend = "暂无数据";
    [ObservableProperty] private string _pushReason = "等待中";
    [ObservableProperty] private string _intervalDisplay = "10s";
    [ObservableProperty] private string _resolvedName = "-";
    [ObservableProperty] private string _resolvedExtend = "-";
    [ObservableProperty] private string _matchedRuleId = "-";
    [ObservableProperty] private string _serverSummary = "暂无数据";
    [ObservableProperty] private string _windowTitle = "等待监控启动";
    [ObservableProperty] private string _appName = "-";
    [ObservableProperty] private string _processName = "-";
    [ObservableProperty] private string _executablePath = "-";
    [ObservableProperty] private string _bundleId = "-";
    [ObservableProperty] private string _mediaTitle = "-";
    [ObservableProperty] private string _mediaArtist = "-";
    [ObservableProperty] private string _mediaThumbnail = "-";
    [ObservableProperty] private string _mediaThumbnailPath = string.Empty;
    [ObservableProperty] private string _logOutput = string.Empty;
    [ObservableProperty] private WindowMatchRuleModel? _selectedRule;
    [ObservableProperty] private bool _isMonitoring;

    public MainWindowViewModel()
    {
        _monitorTimer = new DispatcherTimer { Interval = TimeSpan.FromSeconds(1) };
        _monitorTimer.Tick += async (_, _) => await MonitorTickAsync();

        ConfigPath = StatusShareNative.DefaultConfigFilePath();
        LoadInitialConfig();
        SetReadyMessage();
    }

    partial void OnSelectedRuleChanged(WindowMatchRuleModel? value) => DeleteRuleCommand.NotifyCanExecuteChanged();

    partial void OnHeartbeatIntervalSecsChanged(int value)
    {
        if (value < 5)
        {
            HeartbeatIntervalSecs = 5;
            return;
        }

        IntervalDisplay = $"{HeartbeatIntervalSecs}s";
        _schedulerSnapshot.HeartbeatIntervalSecs = (ulong)HeartbeatIntervalSecs;
    }

    [RelayCommand]
    private void AddRule()
    {
        var rule = new WindowMatchRuleModel { Id = $"rule-{Rules.Count + 1}" };
        Rules.Add(rule);
        SelectedRule = rule;
    }

    [RelayCommand(CanExecute = nameof(CanDeleteRule))]
    private void DeleteRule()
    {
        if (SelectedRule is null)
        {
            return;
        }

        var index = Rules.IndexOf(SelectedRule);
        if (index < 0)
        {
            return;
        }

        Rules.RemoveAt(index);
        SelectedRule = Rules.Count == 0 ? null : Rules[Math.Min(index, Rules.Count - 1)];
    }

    private bool CanDeleteRule() => SelectedRule is not null;

    [RelayCommand]
    private void LoadConfig()
    {
        var result = StatusShareNative.LoadPersistedConfig(ConfigPath);
        if (result.Success && result.Config is not null)
        {
            ApplyConfig(result.Config);
        }
        else if (!result.Success)
        {
            ApplyConfig(StatusShareNative.DefaultPersistedConfig());
        }

        LogOutput = BridgeJson.Serialize(result);
    }

    [RelayCommand]
    private void SaveConfig() => LogOutput = BridgeJson.Serialize(StatusShareNative.SavePersistedConfig(ConfigPath, BuildPersistedConfig()));

    [RelayCommand]
    private async Task FetchServerStatusAsync()
    {
        var result = await Task.Run(() => StatusShareNative.FetchStatus(BuildCoreConfig()));
        ServerSummary = SummarizeServerSnapshot(result);
        LogOutput = BridgeJson.Serialize(result);
    }

    [RelayCommand]
    private async Task StartMonitorAsync()
    {
        if (IsMonitoring)
        {
            return;
        }

        _schedulerSnapshot = new SchedulerSnapshotDto
        {
            HeartbeatIntervalSecs = (ulong)Math.Max(5, HeartbeatIntervalSecs),
            LastFingerprint = string.Empty,
            LastReportAt = 0,
        };

        MonitorStatus = "启动中";
        PushReason = "等待中";
        IsMonitoring = true;
        _monitorTimer.Start();
        await MonitorTickAsync();
    }

    [RelayCommand]
    private void StopMonitor()
    {
        _monitorTimer.Stop();
        IsMonitoring = false;
        MonitorStatus = "已停止";
        PushReason = "已停止";
        LogOutput = "监控已停止";
    }

    private void LoadInitialConfig()
    {
        var loaded = StatusShareNative.LoadPersistedConfig(ConfigPath);
        ApplyConfig(loaded.Success && loaded.Config is not null ? loaded.Config : StatusShareNative.DefaultPersistedConfig());
    }

    private void ApplyConfig(PersistedConfigDto config)
    {
        BaseUrl = config.Core.BaseUrl;
        Token = config.Core.Token;
        HeartbeatIntervalSecs = (int)config.Core.HeartbeatIntervalSecs;
        DefaultReport = config.Matching.DefaultReport;
        DefaultDisplayName = config.Matching.DefaultDisplayName;
        DefaultExtend = config.Matching.DefaultExtend;

        Rules.Clear();
        foreach (var rule in config.Matching.Rules.Select(WindowMatchRuleModel.FromDto))
        {
            Rules.Add(rule);
        }

        SelectedRule = Rules.FirstOrDefault();
        _schedulerSnapshot = new SchedulerSnapshotDto { HeartbeatIntervalSecs = (ulong)Math.Max(5, HeartbeatIntervalSecs) };
        IntervalDisplay = $"{HeartbeatIntervalSecs}s";
    }

    private PersistedConfigDto BuildPersistedConfig() => new()
    {
        SchemaVersion = 1,
        Core = BuildCoreConfig(),
        Matching = BuildMatchingConfig(),
    };

    private CoreConfigDto BuildCoreConfig() => new()
    {
        BaseUrl = BaseUrl,
        Token = Token,
        HeartbeatIntervalSecs = (ulong)Math.Max(5, HeartbeatIntervalSecs),
        UserAgent = "StatusShare WPF/0.1.0",
    };

    private MatchEngineConfigDto BuildMatchingConfig() => new()
    {
        DefaultReport = DefaultReport,
        DefaultDisplayName = DefaultDisplayName,
        DefaultExtend = DefaultExtend,
        Rules = Rules.Select(rule => rule.ToDto()).ToList(),
    };

    private async Task MonitorTickAsync()
    {
        if (!IsMonitoring || _tickRunning)
        {
            return;
        }

        _tickRunning = true;
        try
        {
            var nowSecs = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
            var coreConfig = BuildCoreConfig();
            var matchingConfig = BuildMatchingConfig();
            var schedulerSnapshot = new SchedulerSnapshotDto
            {
                HeartbeatIntervalSecs = (ulong)Math.Max(5, HeartbeatIntervalSecs),
                LastFingerprint = _schedulerSnapshot.LastFingerprint,
                LastReportAt = _schedulerSnapshot.LastReportAt,
            };

            var execution = await Task.Run(() => ExecuteMonitorTick(coreConfig, matchingConfig, schedulerSnapshot, nowSecs));
            if (!IsMonitoring)
            {
                return;
            }

            _schedulerSnapshot = execution.Snapshot;
            Backend = execution.Backend;
            MonitorStatus = "运行中";
            ApplyWindow(execution.Window);
            ApplyMedia(execution.Media);

            ResolvedName = DisplayOrDash(execution.Resolve.Process);
            ResolvedExtend = DisplayOrDash(execution.Resolve.Extend);
            MatchedRuleId = DisplayOrDash(execution.Resolve.MatchedRuleId);
            PushReason = TranslateReportReason(execution.Plan.Decision.Reason);

            if (execution.ApiResult is not null)
            {
                ServerSummary = SummarizeServerSnapshot(execution.ApiResult);
            }

            LogOutput = JsonSerializer.Serialize(new { resolve = execution.Resolve, schedule = execution.Plan, api = execution.ApiResult }, BridgeJson.Options);
        }
        catch (Exception ex)
        {
            MonitorStatus = "运行中但有错误";
            PushReason = "错误";
            LogOutput = $"监控错误\n\n{ex}";
        }
        finally
        {
            _tickRunning = false;
        }
    }

    private MonitorExecutionResult ExecuteMonitorTick(
        CoreConfigDto coreConfig,
        MatchEngineConfigDto matchingConfig,
        SchedulerSnapshotDto schedulerSnapshot,
        long nowSecs)
    {
        var (backend, window) = _windowDetectionService.DetectActiveWindow();
        var media = _mediaDetectionService.DetectMedia();

        var resolve = StatusShareNative.ResolveStatusUpdate(matchingConfig, new ResolveStatusInputDto
        {
            Window = window,
            Media = media,
            Timestamp = nowSecs,
        });

        var plan = StatusShareNative.PlanStatusUpdate(schedulerSnapshot, resolve.Update, nowSecs);
        var nextSnapshot = plan.Snapshot;
        ApiCallResultDto? apiResult = null;

        if (resolve.ShouldReport && plan.Decision.ShouldPush && resolve.Update is not null)
        {
            apiResult = StatusShareNative.PushStatus(coreConfig, resolve.Update);
            if (apiResult.Success)
            {
                nextSnapshot = StatusShareNative.MarkStatusPushed(nextSnapshot, plan.Decision.Fingerprint, nowSecs);
            }
        }

        return new MonitorExecutionResult
        {
            Backend = backend,
            Window = window,
            Media = media,
            Resolve = resolve,
            Plan = plan,
            ApiResult = apiResult,
            Snapshot = nextSnapshot,
        };
    }

    private void ApplyWindow(WindowInfoDto window)
    {
        WindowTitle = DisplayOrDash(window.WindowTitle);
        AppName = DisplayOrDash(window.AppName);
        ProcessName = DisplayOrDash(window.ProcessName);
        ExecutablePath = DisplayOrDash(window.ExecutablePath);
        BundleId = DisplayOrDash(window.BundleId);
    }

    private void ApplyMedia(MediaInfoDto? media)
    {
        MediaTitle = media is null ? "-" : DisplayOrDash(media.Title);
        MediaArtist = media is null ? "-" : DisplayOrDash(media.Artist);
        MediaThumbnail = media is null ? "-" : DisplayOrDash(media.Thumbnail);
        MediaThumbnailPath = media?.Thumbnail ?? string.Empty;
    }

    private static string SummarizeServerSnapshot(ApiCallResultDto result)
    {
        if (!result.Success || result.Snapshot is null)
        {
            return string.IsNullOrWhiteSpace(result.ErrorMessage) ? "暂无数据" : result.ErrorMessage;
        }

        var snapshot = result.Snapshot;
        return $"{(snapshot.Ok == 1 ? "在线" : "离线")} | {DisplayOrDash(snapshot.Process)}";
    }

    private static string TranslateReportReason(ReportReason reason) => reason switch
    {
        ReportReason.None => "等待中",
        ReportReason.Initial => "首次上报",
        ReportReason.Changed => "内容变化",
        ReportReason.Heartbeat => "心跳到期",
        _ => reason.ToString(),
    };

    private static string DisplayOrDash(string? value) => string.IsNullOrWhiteSpace(value) ? "-" : value.Trim();

    private void SetReadyMessage()
    {
        LogOutput = "准备就绪。\n\n建议顺序：\n1. 确认 Base URL 和 gt_ token\n2. 先设置默认名称和默认文案，再补充窗口规则\n3. 点击开始监控，进入自动上报流程";
    }

    private sealed class MonitorExecutionResult
    {
        public string Backend { get; init; } = string.Empty;
        public WindowInfoDto Window { get; init; } = new();
        public MediaInfoDto? Media { get; init; }
        public ResolveStatusResultDto Resolve { get; init; } = new();
        public SchedulerPlanResultDto Plan { get; init; } = new();
        public ApiCallResultDto? ApiResult { get; init; }
        public SchedulerSnapshotDto Snapshot { get; init; } = new();
    }
}

