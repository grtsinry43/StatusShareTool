using System.Net.Http;
using System.Net.Http.Json;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace StatusShare.WindowsApp.Services;

internal sealed class GameLookupPreview
{
    public bool Found { get; init; }
    public string Name { get; init; } = string.Empty;
    public string ShortDescription { get; init; } = string.Empty;
    public string HeaderImage { get; init; } = string.Empty;
    public string StoreUrl { get; init; } = string.Empty;
}

internal sealed class GameLookupService
{
    private static readonly HttpClient HttpClient = CreateClient();
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNameCaseInsensitive = true,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
    };

    public async Task<GameLookupPreview> FetchAsync(string baseUrl, string name, CancellationToken cancellationToken = default)
    {
        var keyword = name.Trim();
        if (keyword.Length == 0)
        {
            throw new InvalidOperationException("请先填写 Game Name 或 Display Name 作为查询关键词");
        }

        var requestUri = $"{BuildGameLookupUrl(baseUrl)}?name={Uri.EscapeDataString(keyword)}";
        using var response = await HttpClient.GetAsync(requestUri, cancellationToken);
        if (!response.IsSuccessStatusCode)
        {
            throw new InvalidOperationException($"服务端返回 {(int)response.StatusCode}");
        }

        var envelope = await response.Content.ReadFromJsonAsync<GameLookupEnvelope>(JsonOptions, cancellationToken)
            ?? throw new InvalidOperationException("响应解析失败");
        if (envelope.Code != 0)
        {
            var message = string.IsNullOrWhiteSpace(envelope.Message) ? "未知错误" : envelope.Message;
            throw new InvalidOperationException($"服务端错误: {message}");
        }

        var data = envelope.Data ?? new GameLookupData();
        return new GameLookupPreview
        {
            Found = data.Found,
            Name = data.Name?.Trim() ?? string.Empty,
            ShortDescription = data.ShortDescription?.Trim() ?? string.Empty,
            HeaderImage = data.HeaderImage?.Trim() ?? string.Empty,
            StoreUrl = data.StoreUrl?.Trim() ?? string.Empty,
        };
    }

    internal static string BuildGameLookupUrl(string baseUrl)
    {
        var trimmed = baseUrl.Trim().TrimEnd('/');
        if (trimmed.EndsWith("/onlineStatus", StringComparison.OrdinalIgnoreCase))
        {
            trimmed = trimmed[..^"/onlineStatus".Length].TrimEnd('/');
        }

        if (trimmed.EndsWith("/api/v2", StringComparison.OrdinalIgnoreCase))
        {
            return $"{trimmed}/public/game-lookup";
        }

        return $"{trimmed}/api/v2/public/game-lookup";
    }

    private static HttpClient CreateClient()
    {
        var client = new HttpClient { Timeout = TimeSpan.FromSeconds(10) };
        client.DefaultRequestHeaders.UserAgent.ParseAdd("StatusShareTool/windows-wpf");
        return client;
    }

    private sealed class GameLookupEnvelope
    {
        public int Code { get; set; }

        [JsonPropertyName("msg")]
        public string Message { get; set; } = string.Empty;

        public GameLookupData? Data { get; set; }
    }

    private sealed class GameLookupData
    {
        public bool Found { get; set; }
        public string Name { get; set; } = string.Empty;
        public string ShortDescription { get; set; } = string.Empty;
        public string HeaderImage { get; set; } = string.Empty;
        public string StoreUrl { get; set; } = string.Empty;
    }
}
