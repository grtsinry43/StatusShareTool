using StatusShare.WindowsApp.Interop;

namespace StatusShare.WindowsApp.Services;

internal sealed class MediaDetectionService
{
    public MediaInfoDto? DetectMedia()
    {
        try
        {
            var result = StatusShareNative.DetectMedia();
            return result.Success ? result.Media : null;
        }
        catch
        {
            return null;
        }
    }

    public Task<MediaInfoDto?> DetectMediaAsync() => Task.FromResult(DetectMedia());
}
