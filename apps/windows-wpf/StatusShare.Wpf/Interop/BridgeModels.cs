using System.Text.Json;
using System.Text.Json.Serialization;

namespace StatusShare.WindowsApp.Interop;

internal static class BridgeJson
{
    public static readonly JsonSerializerOptions Options = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
        WriteIndented = true,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
        Converters = { new JsonStringEnumConverter() }
    };

    public static string Serialize<T>(T value) => JsonSerializer.Serialize(value, Options);

    public static T Deserialize<T>(string json)
    {
        var value = JsonSerializer.Deserialize<T>(json, Options);
        if (value is null)
        {
            throw new InvalidOperationException($"Failed to deserialize {typeof(T).Name}.");
        }

        return value;
    }
}

public enum MatchField { WindowTitle, AppName, ProcessName, ExecutablePath, BundleId }
public enum MatchKind { Contains, Exact, Prefix, Suffix }
public enum ReportPolicy { Allow, Deny }
public enum ReportReason { None, Initial, Changed, Heartbeat }

public sealed class CoreConfigDto { public string BaseUrl { get; set; } = "http://127.0.0.1:3000"; public string Token { get; set; } = string.Empty; public ulong HeartbeatIntervalSecs { get; set; } = 10; public string UserAgent { get; set; } = "StatusShare WPF/0.1.0"; }
public sealed class MediaInfoDto { public string Title { get; set; } = string.Empty; public string Artist { get; set; } = string.Empty; public string Thumbnail { get; set; } = string.Empty; }
public sealed class WindowInfoDto { public string WindowTitle { get; set; } = string.Empty; public string AppName { get; set; } = string.Empty; public string ProcessName { get; set; } = string.Empty; public string ExecutablePath { get; set; } = string.Empty; public string BundleId { get; set; } = string.Empty; }
public sealed class WindowDetectResultDto { public bool Success { get; set; } public string Backend { get; set; } = string.Empty; public string ErrorMessage { get; set; } = string.Empty; public WindowInfoDto? Window { get; set; } }
public sealed class MediaDetectResultDto { public bool Success { get; set; } public string Backend { get; set; } = string.Empty; public string ErrorMessage { get; set; } = string.Empty; public MediaInfoDto? Media { get; set; } }
public sealed class WindowMatchRuleDto { public string Id { get; set; } = string.Empty; public bool Enabled { get; set; } = true; public MatchField Field { get; set; } = MatchField.AppName; public MatchKind Kind { get; set; } = MatchKind.Contains; public string Pattern { get; set; } = string.Empty; public bool CaseSensitive { get; set; } public ReportPolicy ReportPolicy { get; set; } = ReportPolicy.Allow; public string DisplayName { get; set; } = string.Empty; public string Extend { get; set; } = string.Empty; }
public sealed class MatchEngineConfigDto { public bool DefaultReport { get; set; } = true; public string DefaultDisplayName { get; set; } = string.Empty; public string DefaultExtend { get; set; } = string.Empty; public List<WindowMatchRuleDto> Rules { get; set; } = []; }
public sealed class PersistedConfigDto { public uint SchemaVersion { get; set; } = 1; public CoreConfigDto Core { get; set; } = new(); public MatchEngineConfigDto Matching { get; set; } = new(); }
public sealed class PersistedConfigResultDto { public bool Success { get; set; } public string Path { get; set; } = string.Empty; public string ErrorMessage { get; set; } = string.Empty; public PersistedConfigDto? Config { get; set; } }
public sealed class ResolveStatusInputDto { public WindowInfoDto Window { get; set; } = new(); public MediaInfoDto? Media { get; set; } public long? Timestamp { get; set; } }
public sealed class StatusUpdateDto { public int? Ok { get; set; } public string? Process { get; set; } public string? Extend { get; set; } public MediaInfoDto? Media { get; set; } public long? Timestamp { get; set; } }
public sealed class ResolveStatusResultDto { public bool ShouldReport { get; set; } public string MatchedRuleId { get; set; } = string.Empty; public string Process { get; set; } = string.Empty; public string Extend { get; set; } = string.Empty; public MediaInfoDto? Media { get; set; } public StatusUpdateDto? Update { get; set; } public string ErrorMessage { get; set; } = string.Empty; }
public sealed class StatusSnapshotDto { public int Ok { get; set; } public string Process { get; set; } = string.Empty; public string Extend { get; set; } = string.Empty; public MediaInfoDto? Media { get; set; } public long Timestamp { get; set; } public bool AdminPanelOnline { get; set; } }
public sealed class ApiCallResultDto { public bool Success { get; set; } public int HttpStatus { get; set; } public int Code { get; set; } public string BizErr { get; set; } = string.Empty; public string Message { get; set; } = string.Empty; public string ErrorMessage { get; set; } = string.Empty; public string RequestId { get; set; } = string.Empty; public string ResponseTimestamp { get; set; } = string.Empty; public StatusSnapshotDto? Snapshot { get; set; } }
public sealed class ScheduleDecisionDto { public bool ShouldPush { get; set; } public ReportReason Reason { get; set; } = ReportReason.None; public string Fingerprint { get; set; } = string.Empty; }
public sealed class SchedulerSnapshotDto { public ulong HeartbeatIntervalSecs { get; set; } = 10; public string LastFingerprint { get; set; } = string.Empty; public long LastReportAt { get; set; } }
public sealed class SchedulerPlanResultDto { public ScheduleDecisionDto Decision { get; set; } = new(); public SchedulerSnapshotDto Snapshot { get; set; } = new(); }

