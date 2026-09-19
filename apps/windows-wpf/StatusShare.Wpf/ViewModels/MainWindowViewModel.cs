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
    private readonly GameLookupService _gameLookupService = new();
    private bool _tickRunning;
    private SchedulerSnapshotDto _schedulerSnapshot = new();
    private CancellationTokenSource? _gameLookupCts;
    private GameLookupPreview? _gameLookupPreview;

    public ObservableCollection<WindowMatchRuleModel> Rules { get; } = [];
    public IReadOnlyList<MatchField> MatchFields { get; } = Enum.GetValues<MatchField>();
    public IReadOnlyList<MatchKind> MatchKinds { get; } = Enum.GetValues<MatchKind>();
    public IReadOnlyList<ReportPolicy> ReportPolicies { get; } = Enum.GetValues<ReportPolicy>();
    public IReadOnlyList<CategoryOption> Categories { get; } =
    [
        new("app", "普通应用"),
        new("game", "游戏 Game"),
        new("music", "音乐 Music"),
    ];

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
    [ObservableProperty] private string _resolvedCategory = "-";
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
    [ObservableProperty] private string _mediaProgressText = "-";
    [ObservableProperty] private string _logOutput = string.Empty;
    [ObservableProperty] private WindowMatchRuleModel? _selectedRule;
    [ObservableProperty] private bool _isMonitoring;
    [ObservableProperty] private bool _hasGameCard;
    [ObservableProperty] private string _gameName = "-";
    [ObservableProperty] private string _gameCover = string.Empty;
    [ObservableProperty] private string _gameSlogan = "-";
    [ObservableProperty] private string _gameDesc = "-";
    [ObservableProperty] private string _gameAccent = string.Empty;
    [ObservableProperty] private string _gameUrl = "-";
    [ObservableProperty] private bool _isGameLookupBusy;
    [ObservableProperty] private string _gameLookupStatus = string.Empty;
    [ObservableProperty] private bool _hasGameLookupResult;
    [ObservableProperty] private string _gameLookupName = string.Empty;
    [ObservableProperty] private string _gameLookupDescription = string.Empty;
    [ObservableProperty] private string _gameLookupCover = string.Empty;

    public MainWindowViewModel()
    {
        _monitorTimer = new DispatcherTimer { Interval = TimeSpan.FromSeconds(1) };
        _monitorTimer.Tick += async (_, _) => await MonitorTickAsync();

        ConfigPath = StatusShareNative.DefaultConfigFilePath();
        LoadInitialConfig();
        SetReadyMessage();
    }

    partial void OnSelectedRuleChanged(WindowMatchRuleModel? value)
    {
        DeleteRuleCommand.NotifyCanExecuteChanged();
        LookupGameCommand.NotifyCanExecuteChanged();
        ApplyGameLookupCommand.NotifyCanExecuteChanged();
        ClearGameLookupPreview();
    }

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

    partial void OnIsGameLookupBusyChanged(bool value)
    {
        LookupGameCommand.NotifyCanExecuteChanged();
        ApplyGameLookupCommand.NotifyCanExecuteChanged();
    }

    partial void OnHasGameLookupResultChanged(bool value) => ApplyGameLookupCommand.NotifyCanExecuteChanged();

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

    [RelayCommand(CanExecute = nameof(CanLookupGame))]
    private async Task LookupGameAsync()
    {
        if (SelectedRule is null)
        {
            return;
        }

        var keyword = string.IsNullOrWhiteSpace(SelectedRule.Game.Name)
            ? SelectedRule.DisplayName
            : SelectedRule.Game.Name;
        if (string.IsNullOrWhiteSpace(keyword))
        {
            ShowGameLookupError("请先填写 Game Name 或 Display Name");
            return;
        }

        _gameLookupCts?.Cancel();
        _gameLookupCts?.Dispose();
        _gameLookupCts = new CancellationTokenSource();
        var token = _gameLookupCts.Token;

        IsGameLookupBusy = true;
        HasGameLookupResult = false;
        GameLookupStatus = "查询中…";
        _gameLookupPreview = null;

        try
        {
            var preview = await _gameLookupService.FetchAsync(BaseUrl, keyword, token);
            if (token.IsCancellationRequested)
            {
                return;
            }

            if (!preview.Found)
            {
                ShowGameLookupError("没有查到匹配的游戏，可以换个更正式的名字试试");
                return;
            }

            _gameLookupPreview = preview;
            GameLookupName = preview.Name;
            GameLookupDescription = string.IsNullOrWhiteSpace(preview.ShortDescription) ? "暂无简介" : preview.ShortDescription;
            GameLookupCover = preview.HeaderImage;
            HasGameLookupResult = true;
            GameLookupStatus = "查询成功";
        }
        catch (OperationCanceledException)
        {
            // 被下一次查询或切规则取消。
        }
        catch (Exception ex)
        {
            ShowGameLookupError(ex.Message);
        }
        finally
        {
            if (!token.IsCancellationRequested)
            {
                IsGameLookupBusy = false;
            }
        }
    }

    private bool CanLookupGame() => SelectedRule is not null && !IsGameLookupBusy;

    [RelayCommand(CanExecute = nameof(CanApplyGameLookup))]
    private void ApplyGameLookup()
    {
        if (SelectedRule is null || _gameLookupPreview is null)
        {
            return;
        }

        if (!string.IsNullOrWhiteSpace(_gameLookupPreview.Name))
        {
            SelectedRule.Game.Name = _gameLookupPreview.Name;
        }

        if (!string.IsNullOrWhiteSpace(_gameLookupPreview.HeaderImage))
        {
            SelectedRule.Game.Cover = _gameLookupPreview.HeaderImage;
        }

        if (!string.IsNullOrWhiteSpace(_gameLookupPreview.StoreUrl))
        {
            SelectedRule.Game.Url = _gameLookupPreview.StoreUrl;
        }
    }

    private bool CanApplyGameLookup() => SelectedRule is not null && HasGameLookupResult && !IsGameLookupBusy;

    private void ShowGameLookupError(string message)
    {
        _gameLookupPreview = null;
        HasGameLookupResult = false;
        GameLookupName = string.Empty;
        GameLookupDescription = string.Empty;
        GameLookupCover = string.Empty;
        GameLookupStatus = message;
        IsGameLookupBusy = false;
    }

    private void ClearGameLookupPreview()
    {
        _gameLookupCts?.Cancel();
        _gameLookupPreview = null;
        HasGameLookupResult = false;
        IsGameLookupBusy = false;
        GameLookupStatus = string.Empty;
        GameLookupName = string.Empty;
        GameLookupDescription = string.Empty;
        GameLookupCover = string.Empty;
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
            ApplyResolved(execution.Resolve);

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
        MediaProgressText = FormatMediaProgress(media);
    }

    private void ApplyResolved(ResolveStatusResultDto resolve)
    {
        ResolvedName = DisplayOrDash(resolve.Process);
        ResolvedExtend = DisplayOrDash(resolve.Extend);
        ResolvedCategory = string.IsNullOrWhiteSpace(resolve.Category) ? "普通应用" : resolve.Category.Trim();
        MatchedRuleId = DisplayOrDash(resolve.MatchedRuleId);

        var isGame = string.Equals(resolve.Category, "game", StringComparison.OrdinalIgnoreCase);
        var game = resolve.Game;
        HasGameCard = isGame;
        GameName = DisplayOrDash(string.IsNullOrWhiteSpace(game?.Name) ? resolve.Process : game!.Name);
        GameCover = game?.Cover ?? string.Empty;
        GameSlogan = DisplayOrDash(game?.Slogan);
        GameDesc = DisplayOrDash(game?.Desc);
        GameAccent = game?.Accent ?? string.Empty;
        GameUrl = DisplayOrDash(game?.Url);
    }

    private static string SummarizeServerSnapshot(ApiCallResultDto result)
    {
        if (!result.Success || result.Snapshot is null)
        {
            return string.IsNullOrWhiteSpace(result.ErrorMessage) ? "暂无数据" : result.ErrorMessage;
        }

        var snapshot = result.Snapshot;
        var category = string.IsNullOrWhiteSpace(snapshot.Category) ? "app" : snapshot.Category.Trim();
        return $"{(snapshot.Ok == 1 ? "在线" : "离线")} | {DisplayOrDash(snapshot.Process)} | {category}";
    }

    private static string TranslateReportReason(ReportReason reason) => reason switch
    {
        ReportReason.None => "等待中",
        ReportReason.Initial => "首次上报",
        ReportReason.Changed => "内容变化",
        ReportReason.Heartbeat => "心跳到期",
        _ => reason.ToString(),
    };

    private static string FormatMediaProgress(MediaInfoDto? media)
    {
        if (media is null)
        {
            return "-";
        }

        var state = string.IsNullOrWhiteSpace(media.State) ? "" : media.State.Trim();
        if (media.Duration > 0)
        {
            var progress = $"{FormatClock(media.Position)} / {FormatClock(media.Duration)}";
            return string.IsNullOrEmpty(state) ? progress : $"{progress} · {state}";
        }

        return string.IsNullOrEmpty(state) ? "-" : state;
    }

    private static string FormatClock(double seconds)
    {
        if (double.IsNaN(seconds) || double.IsInfinity(seconds) || seconds < 0)
        {
            seconds = 0;
        }

        var value = TimeSpan.FromSeconds(seconds);
        return value.TotalHours >= 1 ? value.ToString(@"h\:mm\:ss") : value.ToString(@"m\:ss");
    }

    private static string DisplayOrDash(string? value) => string.IsNullOrWhiteSpace(value) ? "-" : value.Trim();

    private void SetReadyMessage()
    {
        LogOutput = "准备就绪。\n\n建议顺序：\n1. 确认 Base URL 和 gt_ token\n2. 先设置默认名称和默认文案，再补充窗口规则\n3. 游戏规则把 Category 设为 game，并填写卡片字段（可用 Steam 查询预览）\n4. 点击开始监控，进入自动上报流程";
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
