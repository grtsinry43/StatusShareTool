//! 规则编辑器里的「Steam 查询预览」。
//!
//! 直接请求博客服务端的 `/api/v2/public/game-lookup`——
//! 与前台游戏卡片使用的是同一个接口、同一份缓存，
//! 因此这里看到的就是访客最终会看到的兜底卡片数据。

use serde::Deserialize;

#[derive(Debug, Clone, Default)]
pub struct GameLookupPreview {
    pub found: bool,
    pub name: String,
    pub short_description: String,
    /// 远端封面 URL（用于「填入卡片字段」）
    pub header_image: String,
    /// 已下载到本地缓存的封面路径（用于预览展示）
    pub header_image_path: Option<String>,
    pub store_url: String,
}

#[derive(Debug, Deserialize)]
struct ApiEnvelope {
    #[serde(default)]
    code: i32,
    #[serde(default, rename = "msg")]
    message: String,
    data: Option<GameLookupData>,
}

#[derive(Debug, Deserialize)]
struct GameLookupData {
    #[serde(default)]
    found: bool,
    #[serde(default)]
    name: String,
    #[serde(default, rename = "shortDescription")]
    short_description: String,
    #[serde(default, rename = "headerImage")]
    header_image: String,
    #[serde(default, rename = "storeUrl")]
    store_url: String,
}

/// 由 base_url 推导出 game-lookup 接口地址。
/// base_url 允许三种形态：站点根、`/api/v2`、完整的 onlineStatus 地址。
fn build_game_lookup_url(base_url: &str) -> String {
    let trimmed = base_url.trim().trim_end_matches('/');
    let api_root = if let Some(stripped) = trimmed.strip_suffix("/onlineStatus") {
        stripped
    } else if trimmed.ends_with("/api/v2") {
        trimmed
    } else {
        return format!("{trimmed}/api/v2/public/game-lookup");
    };
    format!("{api_root}/public/game-lookup")
}

pub fn fetch_game_lookup(base_url: &str, name: &str) -> Result<GameLookupPreview, String> {
    let keyword = name.trim();
    if keyword.is_empty() {
        return Err("请先填写 Game Name 或 Display Name 作为查询关键词".to_string());
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|err| err.to_string())?;

    let response = client
        .get(build_game_lookup_url(base_url))
        .query(&[("name", keyword)])
        .header(reqwest::header::USER_AGENT, "StatusShareTool/linux-gtk")
        .send()
        .map_err(|err| format!("请求失败: {err}"))?;

    if !response.status().is_success() {
        return Err(format!("服务端返回 {}", response.status()));
    }

    let envelope: ApiEnvelope = response
        .json()
        .map_err(|err| format!("响应解析失败: {err}"))?;
    if envelope.code != 0 {
        return Err(format!("服务端错误: {}", envelope.message));
    }

    let data = envelope.data.unwrap_or(GameLookupData {
        found: false,
        name: String::new(),
        short_description: String::new(),
        header_image: String::new(),
        store_url: String::new(),
    });

    let header_image_path = if data.found && !data.header_image.is_empty() {
        crate::app::cached_thumbnail_path(&data.header_image).map(|p| p.to_string_lossy().into_owned())
    } else {
        None
    };

    Ok(GameLookupPreview {
        found: data.found,
        name: data.name,
        short_description: data.short_description,
        header_image: data.header_image,
        header_image_path,
        store_url: data.store_url,
    })
}
