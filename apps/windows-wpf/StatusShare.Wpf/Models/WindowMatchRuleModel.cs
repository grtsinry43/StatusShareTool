using CommunityToolkit.Mvvm.ComponentModel;
using StatusShare.WindowsApp.Interop;

namespace StatusShare.WindowsApp.Models;

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

    public string Summary => $"{(Enabled ? "Enabled" : "Disabled")} | {Field} | {ReportPolicy}";

    partial void OnEnabledChanged(bool value) => OnPropertyChanged(nameof(Summary));
    partial void OnFieldChanged(MatchField value) => OnPropertyChanged(nameof(Summary));
    partial void OnReportPolicyChanged(ReportPolicy value) => OnPropertyChanged(nameof(Summary));
    partial void OnPatternChanged(string value) => OnPropertyChanged(nameof(Summary));
    partial void OnDisplayNameChanged(string value) => OnPropertyChanged(nameof(Summary));

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
    };
}
