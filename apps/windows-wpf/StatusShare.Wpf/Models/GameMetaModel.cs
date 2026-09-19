using CommunityToolkit.Mvvm.ComponentModel;
using StatusShare.WindowsApp.Interop;

namespace StatusShare.WindowsApp.Models;

public partial class GameMetaModel : ObservableObject
{
    [ObservableProperty]
    private string _name = string.Empty;

    [ObservableProperty]
    private string _cover = string.Empty;

    [ObservableProperty]
    private string _slogan = string.Empty;

    [ObservableProperty]
    private string _desc = string.Empty;

    [ObservableProperty]
    private string _accent = string.Empty;

    [ObservableProperty]
    private string _url = string.Empty;

    public GameMetaDto ToDto() => new()
    {
        Name = Name,
        Cover = Cover,
        Slogan = Slogan,
        Desc = Desc,
        Accent = Accent,
        Url = Url,
    };

    public void ApplyDto(GameMetaDto? dto)
    {
        Name = dto?.Name ?? string.Empty;
        Cover = dto?.Cover ?? string.Empty;
        Slogan = dto?.Slogan ?? string.Empty;
        Desc = dto?.Desc ?? string.Empty;
        Accent = dto?.Accent ?? string.Empty;
        Url = dto?.Url ?? string.Empty;
    }

    public static GameMetaModel FromDto(GameMetaDto? dto)
    {
        var model = new GameMetaModel();
        model.ApplyDto(dto);
        return model;
    }
}
