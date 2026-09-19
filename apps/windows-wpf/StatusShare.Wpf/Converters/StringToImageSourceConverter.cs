using System.Globalization;
using System.IO;
using System.Windows.Data;
using System.Windows.Media.Imaging;

namespace StatusShare.WindowsApp.Converters;

public sealed class StringToImageSourceConverter : IValueConverter
{
    public object? Convert(object value, Type targetType, object parameter, CultureInfo culture)
    {
        if (value is not string text || string.IsNullOrWhiteSpace(text))
        {
            return null;
        }

        try
        {
            if (!TryCreateUri(text.Trim(), out var uri))
            {
                return null;
            }

            var bitmap = new BitmapImage();
            bitmap.BeginInit();
            bitmap.UriSource = uri;
            bitmap.CreateOptions = BitmapCreateOptions.IgnoreColorProfile;
            var isRemote = uri.Scheme == Uri.UriSchemeHttp || uri.Scheme == Uri.UriSchemeHttps;
            bitmap.CacheOption = isRemote ? BitmapCacheOption.OnDemand : BitmapCacheOption.OnLoad;
            bitmap.EndInit();
            if (!isRemote)
            {
                bitmap.Freeze();
            }
            return bitmap;
        }
        catch
        {
            return null;
        }
    }

    public object? ConvertBack(object value, Type targetType, object parameter, CultureInfo culture) => Binding.DoNothing;

    private static bool TryCreateUri(string text, out Uri uri)
    {
        if (Uri.TryCreate(text, UriKind.Absolute, out uri!)
            && (uri.Scheme == Uri.UriSchemeHttp || uri.Scheme == Uri.UriSchemeHttps || uri.Scheme == Uri.UriSchemeFile))
        {
            return true;
        }

        if (File.Exists(text))
        {
            uri = new Uri(Path.GetFullPath(text));
            return true;
        }

        uri = null!;
        return false;
    }
}
