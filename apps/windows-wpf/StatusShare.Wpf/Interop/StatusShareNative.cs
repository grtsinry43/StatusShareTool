using System.Runtime.InteropServices;
using System.Text.Json;

namespace StatusShare.Wpf.Interop;

internal static class StatusShareNative
{
    [DllImport("windows_pinvoke", CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    private static extern nint ss_fetch_status(string configJson);

    [DllImport("windows_pinvoke", CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    private static extern nint ss_push_status(string configJson, string updateJson);

    [DllImport("windows_pinvoke", CallingConvention = CallingConvention.Cdecl)]
    private static extern void ss_string_free(nint ptr);

    public static string FetchStatus(string configJson) => ReadAndFree(ss_fetch_status(configJson));

    public static string PushStatus(string configJson, string updateJson) => ReadAndFree(ss_push_status(configJson, updateJson));

    private static string ReadAndFree(nint ptr)
    {
        if (ptr == nint.Zero)
        {
            return JsonSerializer.Serialize(new
            {
                success = false,
                error_message = "native bridge returned null"
            });
        }

        try
        {
            return Marshal.PtrToStringAnsi(ptr) ?? string.Empty;
        }
        finally
        {
            ss_string_free(ptr);
        }
    }
}

