using CommunityToolkit.Mvvm.ComponentModel;
using StatusShare.WindowsApp.Interop;

namespace StatusShare.WindowsApp.Models;

public sealed class CategoryOption
{
    public CategoryOption(string id, string label)
    {
        Id = id;
        Label = label;
    }

    public string Id { get; }
    public string Label { get; }
}

public partial class WindowMatchRuleModel : ObservableObject
{
    [ObservableProperty]
    private string _id = string.Empty;

    [ObservableProperty]
    private bool _enabled = true;

    [ObservableProperty]
    private MatchField _field = MatchField.AppName;

    [ObservableProperty]
    private MatchKind _kind = MatchKind.Contains;

    [ObservableProperty]
    private string _pattern = string.Empty;

    [ObservableProperty]
    private bool _caseSensitive;

    [ObservableProperty]
    private ReportPolicy _reportPolicy = ReportPolicy.Allow;

    [ObservableProperty]
    private string _displayName = string.Empty;

    [ObservableProperty]
    private string _extend = string.Empty;

    [ObservableProperty]
    private string _category = string.Empty;

    [ObservableProperty]
    private GameMetaModel _game = new();

    public bool IsGameCategory => string.Equals(Category, "game", StringComparison.OrdinalIgnoreCase);

    public string CategorySelection
    {
        get => string.IsNullOrWhiteSpace(Category) ? "app" : Category.Trim();
        set => Category = string.Equals(value, "app", StringComparison.OrdinalIgnoreCase) ? string.Empty : value ?? string.Empty;
    }

    public string Summary
    {
        get
        {
            var category = string.IsNullOrWhiteSpace(Category) ? "app" : Category.Trim();
            return $"{(Enabled ? "Enabled" : "Disabled")} | {Field} | {ReportPolicy} | {category}";
        }
    }

    partial void OnEnabledChanged(bool value) => OnPropertyChanged(nameof(Summary));
    partial void OnFieldChanged(MatchField value) => OnPropertyChanged(nameof(Summary));
    partial void OnReportPolicyChanged(ReportPolicy value) => OnPropertyChanged(nameof(Summary));
    partial void OnPatternChanged(string value) => OnPropertyChanged(nameof(Summary));
    partial void OnDisplayNameChanged(string value) => OnPropertyChanged(nameof(Summary));

    partial void OnCategoryChanged(string value)
    {
        OnPropertyChanged(nameof(Summary));
        OnPropertyChanged(nameof(IsGameCategory));
        OnPropertyChanged(nameof(CategorySelection));
    }

    public WindowMatchRuleDto ToDto() => new()
    {
        Id = Id,
        Enabled = Enabled,
        Field = Field,
        Kind = Kind,
        Pattern = Pattern,
        CaseSensitive = CaseSensitive,
        ReportPolicy = ReportPolicy,
        DisplayName = DisplayName,
        Extend = Extend,
        Category = string.Equals(Category, "app", StringComparison.OrdinalIgnoreCase)
            ? string.Empty
            : Category?.Trim() ?? string.Empty,
        Game = Game.ToDto(),
    };

    public static WindowMatchRuleModel FromDto(WindowMatchRuleDto dto) => new()
    {
        Id = dto.Id,
        Enabled = dto.Enabled,
        Field = dto.Field,
        Kind = dto.Kind,
        Pattern = dto.Pattern,
        CaseSensitive = dto.CaseSensitive,
        ReportPolicy = dto.ReportPolicy,
        DisplayName = dto.DisplayName,
        Extend = dto.Extend,
        Category = dto.Category ?? string.Empty,
        Game = GameMetaModel.FromDto(dto.Game),
    };
}
