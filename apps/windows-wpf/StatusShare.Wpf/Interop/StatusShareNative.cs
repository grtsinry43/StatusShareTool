using System.Runtime.InteropServices;

namespace StatusShare.WindowsApp.Interop;

internal static class StatusShareNative
{
    [DllImport("windows_pinvoke", CallingConvention = CallingConvention.Cdecl)] private static extern nint ss_fetch_status([MarshalAs(UnmanagedType.LPUTF8Str)] string configJson);
    [DllImport("windows_pinvoke", CallingConvention = CallingConvention.Cdecl)] private static extern nint ss_push_status([MarshalAs(UnmanagedType.LPUTF8Str)] string configJson, [MarshalAs(UnmanagedType.LPUTF8Str)] string updateJson);
    [DllImport("windows_pinvoke", CallingConvention = CallingConvention.Cdecl)] private static extern nint ss_default_config_file_path();
    [DllImport("windows_pinvoke", CallingConvention = CallingConvention.Cdecl)] private static extern nint ss_default_persisted_config();
    [DllImport("windows_pinvoke", CallingConvention = CallingConvention.Cdecl)] private static extern nint ss_load_persisted_config([MarshalAs(UnmanagedType.LPUTF8Str)] string path);
    [DllImport("windows_pinvoke", CallingConvention = CallingConvention.Cdecl)] private static extern nint ss_save_persisted_config([MarshalAs(UnmanagedType.LPUTF8Str)] string path, [MarshalAs(UnmanagedType.LPUTF8Str)] string configJson);
    [DllImport("windows_pinvoke", CallingConvention = CallingConvention.Cdecl)] private static extern nint ss_resolve_status_update([MarshalAs(UnmanagedType.LPUTF8Str)] string matchingJson, [MarshalAs(UnmanagedType.LPUTF8Str)] string inputJson);
    [DllImport("windows_pinvoke", CallingConvention = CallingConvention.Cdecl)] private static extern nint ss_plan_status_update([MarshalAs(UnmanagedType.LPUTF8Str)] string snapshotJson, [MarshalAs(UnmanagedType.LPUTF8Str)] string updateJson, long nowSecs);
    [DllImport("windows_pinvoke", CallingConvention = CallingConvention.Cdecl)] private static extern nint ss_mark_status_pushed([MarshalAs(UnmanagedType.LPUTF8Str)] string snapshotJson, [MarshalAs(UnmanagedType.LPUTF8Str)] string fingerprint, long nowSecs);
    [DllImport("windows_pinvoke", CallingConvention = CallingConvention.Cdecl)] private static extern nint ss_detect_active_window();
    [DllImport("windows_pinvoke", CallingConvention = CallingConvention.Cdecl)] private static extern nint ss_detect_media();
    [DllImport("windows_pinvoke", CallingConvention = CallingConvention.Cdecl)] private static extern void ss_string_free(nint ptr);

    public static string DefaultConfigFilePath() => ReadAndFree(ss_default_config_file_path());
    public static PersistedConfigDto DefaultPersistedConfig() => BridgeJson.Deserialize<PersistedConfigDto>(ReadAndFree(ss_default_persisted_config()));
    public static PersistedConfigResultDto LoadPersistedConfig(string path) => BridgeJson.Deserialize<PersistedConfigResultDto>(ReadAndFree(ss_load_persisted_config(path)));
    public static PersistedConfigResultDto SavePersistedConfig(string path, PersistedConfigDto config) => BridgeJson.Deserialize<PersistedConfigResultDto>(ReadAndFree(ss_save_persisted_config(path, BridgeJson.Serialize(config))));
    public static ApiCallResultDto FetchStatus(CoreConfigDto config) => BridgeJson.Deserialize<ApiCallResultDto>(ReadAndFree(ss_fetch_status(BridgeJson.Serialize(config))));
    public static ApiCallResultDto PushStatus(CoreConfigDto config, StatusUpdateDto update) => BridgeJson.Deserialize<ApiCallResultDto>(ReadAndFree(ss_push_status(BridgeJson.Serialize(config), BridgeJson.Serialize(update))));
    public static ResolveStatusResultDto ResolveStatusUpdate(MatchEngineConfigDto matching, ResolveStatusInputDto input) => DeserializeWithPayload<ResolveStatusResultDto>(ReadAndFree(ss_resolve_status_update(BridgeJson.Serialize(matching), BridgeJson.Serialize(input))), "ResolveStatusResultDto");
    public static SchedulerPlanResultDto PlanStatusUpdate(SchedulerSnapshotDto snapshot, StatusUpdateDto? update, long nowSecs) => BridgeJson.Deserialize<SchedulerPlanResultDto>(ReadAndFree(ss_plan_status_update(BridgeJson.Serialize(snapshot), update is null ? "null" : BridgeJson.Serialize(update), nowSecs)));
    public static SchedulerSnapshotDto MarkStatusPushed(SchedulerSnapshotDto snapshot, string fingerprint, long nowSecs) => BridgeJson.Deserialize<SchedulerSnapshotDto>(ReadAndFree(ss_mark_status_pushed(BridgeJson.Serialize(snapshot), fingerprint, nowSecs)));
    public static WindowDetectResultDto DetectActiveWindow() => DeserializeWithPayload<WindowDetectResultDto>(ReadAndFree(ss_detect_active_window()), "WindowDetectResultDto");
    public static MediaDetectResultDto DetectMedia() => DeserializeWithPayload<MediaDetectResultDto>(ReadAndFree(ss_detect_media()), "MediaDetectResultDto");

    private static T DeserializeWithPayload<T>(string json, string name)
    {
        try
        {
            return BridgeJson.Deserialize<T>(json);
        }
        catch (Exception ex)
        {
            throw new InvalidOperationException($"Failed to deserialize {name}. Raw JSON: {json}", ex);
        }
    }

    private static string ReadAndFree(nint ptr)
    {
        if (ptr == nint.Zero) throw new InvalidOperationException("native bridge returned null");
        try { return Marshal.PtrToStringUTF8(ptr) ?? string.Empty; }
        finally { ss_string_free(ptr); }
    }
}
