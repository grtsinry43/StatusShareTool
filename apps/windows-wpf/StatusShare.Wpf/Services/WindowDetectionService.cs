using StatusShare.WindowsApp.Interop;

namespace StatusShare.WindowsApp.Services;

internal sealed class WindowDetectionService
{
    public (string Backend, WindowInfoDto Window) DetectActiveWindow()
    {
        var result = StatusShareNative.DetectActiveWindow();
        if (result.Success && result.Window is not null)
        {
            return (result.Backend, result.Window);
        }

        return (string.IsNullOrWhiteSpace(result.Backend) ? "win32-foreground" : result.Backend, new WindowInfoDto());
    }
}
