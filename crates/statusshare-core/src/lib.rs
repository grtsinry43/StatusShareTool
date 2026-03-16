mod scheduler;

pub use scheduler::{PushScheduler, ReportReason, ScheduleDecision, SchedulerPlanResult, SchedulerSnapshot, mark_status_pushed, plan_status_update};

use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use std::{env, fs};

use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue, USER_AGENT};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct CoreConfig {
    pub base_url: String,
    pub token: String,
    pub heartbeat_interval_secs: u64,
    pub user_agent: String,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:3000".to_string(),
            token: String::new(),
            heartbeat_interval_secs: 5,
            user_agent: "StatusShareTool/0.1.0".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, uniffi::Record)]
pub struct MediaInfo {
    pub title: String,
    pub artist: String,
    pub thumbnail: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, uniffi::Enum)]
pub enum MatchField {
    #[default]
    WindowTitle,
    AppName,
    ProcessName,
    ExecutablePath,
    BundleId,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, uniffi::Enum)]
pub enum MatchKind {
    #[default]
    Contains,
    Exact,
    Prefix,
    Suffix,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, uniffi::Enum)]
pub enum ReportPolicy {
    #[default]
    Allow,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, uniffi::Record)]
pub struct WindowInfo {
    pub window_title: String,
    pub app_name: String,
    pub process_name: String,
    pub executable_path: String,
    pub bundle_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, uniffi::Record)]
pub struct WindowMatchRule {
    pub id: String,
    pub enabled: bool,
    pub field: MatchField,
    pub kind: MatchKind,
    pub pattern: String,
    pub case_sensitive: bool,
    pub report_policy: ReportPolicy,
    pub display_name: String,
    pub extend: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct MatchEngineConfig {
    pub default_report: bool,
    pub default_display_name: String,
    pub default_extend: String,
    pub rules: Vec<WindowMatchRule>,
}

impl Default for MatchEngineConfig {
    fn default() -> Self {
        Self {
            default_report: true,
            default_display_name: String::new(),
            default_extend: String::new(),
            rules: default_match_rules(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct PersistedConfig {
    pub schema_version: u32,
    pub core: CoreConfig,
    pub matching: MatchEngineConfig,
}

impl Default for PersistedConfig {
    fn default() -> Self {
        Self {
            schema_version: 1,
            core: CoreConfig::default(),
            matching: MatchEngineConfig::default(),
        }
    }
}

fn default_match_rules() -> Vec<WindowMatchRule> {
    vec![
        rule("chrome", "chrome", "Chrome", "Lighthouse跑分专用浏览器，只要关掉插件，我的网站就天下第一"),
        rule("firefox", "firefox", "Firefox", "CSS调试唯一指定亲爹，但产品经理的电脑上没有它"),
        rule("kitty", "kitty", "Kitty", "美化半天，结果99%的时间都在看 `pnpm install` 的进度条"),
        rule("konsole", "konsole", "Konsole", "说不定在 yay -Syyu ，没准一会儿就 grub > 了"),
        rule("code", "code", "VS Code", "ESLint和Prettier天天在我的配置文件里打架"),
        rule("idea", "idea", "IntelliJ IDEA", "要么享受着kt的爽，要么就是面向Spring开发中"),
        rule("clion", "clion", "CLion", "不会有人不喜欢C++吧？ 唉依赖，也是念起CMake vcpkg conan的好了"),
        rule("goland", "goland", "GoLand", "新潮后端们的圣杯，据说能用interface{}写出JavaScript的感觉"),
        rule("pycharm", "pycharm", "PyCharm", "后端同事的快乐老家，据说那里的缩进能决定项目死活"),
        rule("webstorm", "webstorm", "WebStorm", "自动导入一时爽，索引项目火葬场，专治各种 'any' 写法"),
        rule("discord", "discord", "Discord", "React/Vue/Svelte 官方指定撕逼广场"),
        rule("telegram", "telegram", "Telegram", "Vite作者的日常茶馆，前端前沿资讯的第一手信源"),
        rule("wechat", "wechat", "WeChat", "前端兼容性噩梦的始作俑者，梦回IE6"),
        rule("spotify", "spotify", "Spotify", "专注码字BGM生成器，一首歌的时间刚好够我命名一个CSS class"),
        rule("typora", "typora", "Typora", "写README.md的唯一动力，毕竟它排版比我写的UI好看多了"),
        rule("obs", "obs", "OBS", "录制 Bug 复现视频专用，顺便幻想自己是 live-coding 大神"),
        rule("reqable", "reqable", "Reqable", "API 不好用了吧...可能正在倍受折磨"),
    ]
}

fn rule(id: &str, pattern: &str, display_name: &str, extend: &str) -> WindowMatchRule {
    WindowMatchRule {
        id: id.to_string(),
        enabled: true,
        field: MatchField::AppName,
        kind: MatchKind::Contains,
        pattern: pattern.to_string(),
        case_sensitive: false,
        report_policy: ReportPolicy::Allow,
        display_name: display_name.to_string(),
        extend: extend.to_string(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, uniffi::Record)]
pub struct PersistedConfigResult {
    pub success: bool,
    pub path: String,
    pub error_message: String,
    pub config: Option<PersistedConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, uniffi::Record)]
pub struct ResolveStatusInput {
    pub window: WindowInfo,
    pub media: Option<MediaInfo>,
    pub timestamp: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, uniffi::Record)]
pub struct ResolveStatusResult {
    pub should_report: bool,
    pub matched_rule_id: String,
    pub process: String,
    pub extend: String,
    pub media: Option<MediaInfo>,
    pub update: Option<StatusUpdate>,
    pub error_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, uniffi::Record)]
pub struct StatusUpdate {
    pub ok: Option<i32>,
    pub process: Option<String>,
    pub extend: Option<String>,
    pub media: Option<MediaInfo>,
    pub timestamp: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, uniffi::Record)]
pub struct StatusSnapshot {
    pub ok: i32,
    pub process: String,
    pub extend: String,
    pub media: Option<MediaInfo>,
    pub timestamp: i64,
    pub admin_panel_online: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, uniffi::Record)]
pub struct ApiCallResult {
    pub success: bool,
    pub http_status: i32,
    pub code: i32,
    pub biz_err: String,
    pub message: String,
    pub error_message: String,
    pub request_id: String,
    pub response_timestamp: String,
    pub snapshot: Option<StatusSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiMeta {
    #[serde(default, rename = "requestId")]
    request_id: String,
    #[serde(default)]
    timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiEnvelope<T> {
    #[serde(default)]
    code: i32,
    #[serde(default, rename = "bizErr")]
    biz_err: String,
    #[serde(default, rename = "msg")]
    message: String,
    data: Option<T>,
    #[serde(default)]
    meta: Option<ApiMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiStatusSnapshot {
    #[serde(default)]
    ok: i32,
    #[serde(default)]
    process: String,
    #[serde(default)]
    extend: String,
    #[serde(default)]
    media: Option<MediaInfo>,
    #[serde(default)]
    timestamp: i64,
    #[serde(default, rename = "adminPanelOnline")]
    admin_panel_online: bool,
}

impl From<ApiStatusSnapshot> for StatusSnapshot {
    fn from(value: ApiStatusSnapshot) -> Self {
        Self {
            ok: value.ok,
            process: value.process,
            extend: value.extend,
            media: value.media,
            timestamp: value.timestamp,
            admin_panel_online: value.admin_panel_online,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PushStatusBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    ok: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    process: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extend: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    media: Option<MediaInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<i64>,
}

impl From<StatusUpdate> for PushStatusBody {
    fn from(value: StatusUpdate) -> Self {
        Self {
            ok: value.ok,
            process: value.process.and_then(|v| non_empty(v)),
            extend: value.extend.and_then(|v| non_empty(v)),
            media: value.media.and_then(clean_media),
            timestamp: value.timestamp,
        }
    }
}

#[derive(Debug)]
struct HeartbeatControl {
    stop_tx: Sender<()>,
    join_handle: JoinHandle<()>,
}

#[derive(Debug)]
struct SharedState {
    config: Mutex<CoreConfig>,
    last_heartbeat_result: Mutex<ApiCallResult>,
}

#[derive(Debug, uniffi::Object)]
pub struct StatusShareClient {
    http: Client,
    shared: Arc<SharedState>,
    heartbeat: Mutex<Option<HeartbeatControl>>,
}

#[uniffi::export]
impl StatusShareClient {
    #[uniffi::constructor]
    pub fn new(config: CoreConfig) -> Arc<Self> {
        Arc::new(Self {
            http: Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .unwrap_or_else(|_| Client::new()),
            shared: Arc::new(SharedState {
                config: Mutex::new(normalize_config(config)),
                last_heartbeat_result: Mutex::new(ApiCallResult::default()),
            }),
            heartbeat: Mutex::new(None),
        })
    }

    pub fn get_config(&self) -> CoreConfig {
        self.shared.config.lock().unwrap().clone()
    }

    pub fn update_config(&self, config: CoreConfig) {
        *self.shared.config.lock().unwrap() = normalize_config(config);
    }

    pub fn fetch_status(&self) -> ApiCallResult {
        perform_fetch(&self.http, &self.shared.config.lock().unwrap().clone())
    }

    pub fn push_status(&self, update: StatusUpdate) -> ApiCallResult {
        perform_push(
            &self.http,
            &self.shared.config.lock().unwrap().clone(),
            update,
        )
    }

    pub fn start_heartbeat(&self, update: StatusUpdate) -> bool {
        self.stop_heartbeat();

        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let http = self.http.clone();
        let shared = Arc::clone(&self.shared);
        let initial_update = update.clone();

        let join_handle = thread::spawn(move || {
            loop {
                let config = shared.config.lock().unwrap().clone();
                let result = perform_push(&http, &config, initial_update.clone());
                *shared.last_heartbeat_result.lock().unwrap() = result;

                let sleep_for = Duration::from_secs(config.heartbeat_interval_secs.max(5));
                match stop_rx.recv_timeout(sleep_for) {
                    Ok(_) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        });

        *self.heartbeat.lock().unwrap() = Some(HeartbeatControl {
            stop_tx,
            join_handle,
        });
        true
    }

    pub fn stop_heartbeat(&self) {
        if let Some(control) = self.heartbeat.lock().unwrap().take() {
            let _ = control.stop_tx.send(());
            let _ = control.join_handle.join();
        }
    }

    pub fn heartbeat_running(&self) -> bool {
        self.heartbeat.lock().unwrap().is_some()
    }

    pub fn last_heartbeat_result(&self) -> ApiCallResult {
        self.shared.last_heartbeat_result.lock().unwrap().clone()
    }
}

impl Drop for StatusShareClient {
    fn drop(&mut self) {
        let control = match self.heartbeat.get_mut() {
            Ok(slot) => slot.take(),
            Err(_) => None,
        };

        if let Some(control) = control {
            let _ = control.stop_tx.send(());
            let _ = control.join_handle.join();
        }
    }
}

#[uniffi::export]
pub fn default_config() -> CoreConfig {
    CoreConfig::default()
}

#[uniffi::export]
pub fn default_persisted_config() -> PersistedConfig {
    normalize_persisted_config(PersistedConfig::default())
}

#[uniffi::export]
pub fn online_status_endpoint(base_url: String) -> String {
    build_online_status_url(&base_url)
}

#[uniffi::export]
pub fn default_config_file_path() -> String {
    default_config_file_path_inner()
}

#[uniffi::export]
pub fn load_persisted_config(path: String) -> PersistedConfigResult {
    let resolved_path = resolve_config_path(&path);
    match fs::read_to_string(&resolved_path) {
        Ok(content) => match serde_json::from_str::<PersistedConfig>(&content) {
            Ok(config) => PersistedConfigResult {
                success: true,
                path: resolved_path,
                error_message: String::new(),
                config: Some(normalize_persisted_config(config)),
            },
            Err(err) => PersistedConfigResult {
                success: false,
                path: resolved_path,
                error_message: err.to_string(),
                config: None,
            },
        },
        Err(err) => PersistedConfigResult {
            success: false,
            path: resolved_path,
            error_message: err.to_string(),
            config: None,
        },
    }
}

#[uniffi::export]
pub fn save_persisted_config(path: String, config: PersistedConfig) -> PersistedConfigResult {
    let resolved_path = resolve_config_path(&path);
    let normalized = normalize_persisted_config(config);

    if let Some(parent) = std::path::Path::new(&resolved_path).parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            return PersistedConfigResult {
                success: false,
                path: resolved_path,
                error_message: err.to_string(),
                config: None,
            };
        }
    }

    match serde_json::to_string_pretty(&normalized) {
        Ok(serialized) => match fs::write(&resolved_path, serialized) {
            Ok(_) => PersistedConfigResult {
                success: true,
                path: resolved_path,
                error_message: String::new(),
                config: Some(normalized),
            },
            Err(err) => PersistedConfigResult {
                success: false,
                path: resolved_path,
                error_message: err.to_string(),
                config: None,
            },
        },
        Err(err) => PersistedConfigResult {
            success: false,
            path: resolved_path,
            error_message: err.to_string(),
            config: None,
        },
    }
}

#[uniffi::export]
pub fn resolve_status_update(
    config: MatchEngineConfig,
    input: ResolveStatusInput,
) -> ResolveStatusResult {
    resolve_status_update_inner(config, input)
}

fn normalize_config(config: CoreConfig) -> CoreConfig {
    CoreConfig {
        base_url: normalize_base_url(&config.base_url),
        token: config.token.trim().to_string(),
        heartbeat_interval_secs: config.heartbeat_interval_secs.max(5),
        user_agent: non_empty(config.user_agent)
            .unwrap_or_else(|| "StatusShareTool/0.1.0".to_string()),
    }
}

fn normalize_persisted_config(config: PersistedConfig) -> PersistedConfig {
    PersistedConfig {
        schema_version: config.schema_version.max(1),
        core: normalize_config(config.core),
        matching: config.matching,
    }
}

fn resolve_config_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        default_config_file_path_inner()
    } else {
        trimmed.to_string()
    }
}

fn default_config_file_path_inner() -> String {
    let app_dir = "StatusShareTool";
    let file_name = "config.json";

    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = env::var("APPDATA") {
            return format!("{appdata}\\{app_dir}\\{file_name}");
        }
        if let Ok(home) = env::var("USERPROFILE") {
            return format!("{home}\\AppData\\Roaming\\{app_dir}\\{file_name}");
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = env::var("HOME") {
            return format!("{home}/Library/Application Support/{app_dir}/{file_name}");
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Ok(xdg_config_home) = env::var("XDG_CONFIG_HOME") {
            return format!("{xdg_config_home}/{app_dir}/{file_name}");
        }
        if let Ok(home) = env::var("HOME") {
            return format!("{home}/.config/{app_dir}/{file_name}");
        }
    }

    format!("./{file_name}")
}

fn resolve_status_update_inner(
    config: MatchEngineConfig,
    input: ResolveStatusInput,
) -> ResolveStatusResult {
    let normalized_window = normalize_window_info(input.window);
    let media = input.media.and_then(clean_media);

    for rule in config.rules {
        if !rule.enabled || !rule_matches(&rule, &normalized_window) {
            continue;
        }

        let process = choose_process_name(&rule.display_name, &normalized_window);
        let extend = choose_extend(&rule.extend, &config.default_extend);

        if matches!(rule.report_policy, ReportPolicy::Deny) {
            return ResolveStatusResult {
                should_report: false,
                matched_rule_id: rule.id,
                process,
                extend,
                media,
                update: None,
                error_message: String::new(),
            };
        }

        let update = StatusUpdate {
            ok: Some(1),
            process: Some(process.clone()),
            extend: non_empty(extend.clone()),
            media: media.clone(),
            timestamp: input.timestamp,
        };

        return ResolveStatusResult {
            should_report: true,
            matched_rule_id: rule.id,
            process,
            extend,
            media,
            update: Some(update),
            error_message: String::new(),
        };
    }

    let process = choose_process_name(&config.default_display_name, &normalized_window);
    let extend = config.default_extend.trim().to_string();

    if !config.default_report {
        return ResolveStatusResult {
            should_report: false,
            matched_rule_id: String::new(),
            process,
            extend,
            media,
            update: None,
            error_message: String::new(),
        };
    }

    let update = StatusUpdate {
        ok: Some(1),
        process: non_empty(process.clone()),
        extend: non_empty(extend.clone()),
        media: media.clone(),
        timestamp: input.timestamp,
    };

    ResolveStatusResult {
        should_report: true,
        matched_rule_id: String::new(),
        process,
        extend,
        media,
        update: Some(update),
        error_message: String::new(),
    }
}

fn normalize_base_url(input: &str) -> String {
    input.trim().trim_end_matches('/').to_string()
}

fn build_online_status_url(base_url: &str) -> String {
    let trimmed = normalize_base_url(base_url);
    if trimmed.ends_with("/api/v2/onlineStatus") {
        return trimmed;
    }
    if trimmed.ends_with("/api/v2") {
        return format!("{trimmed}/onlineStatus");
    }
    format!("{trimmed}/api/v2/onlineStatus")
}

fn build_headers(config: &CoreConfig) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    if let Ok(user_agent) = HeaderValue::from_str(&config.user_agent) {
        headers.insert(USER_AGENT, user_agent);
    }

    if is_valid_gt_token(&config.token) {
        if let Ok(value) = HeaderValue::from_str(&config.token) {
            headers.insert(AUTHORIZATION, value);
        }
    }

    headers
}

fn normalize_window_info(window: WindowInfo) -> WindowInfo {
    WindowInfo {
        window_title: window.window_title.trim().to_string(),
        app_name: window.app_name.trim().to_string(),
        process_name: window.process_name.trim().to_string(),
        executable_path: window.executable_path.trim().to_string(),
        bundle_id: window.bundle_id.trim().to_string(),
    }
}

fn rule_matches(rule: &WindowMatchRule, window: &WindowInfo) -> bool {
    let pattern = rule.pattern.trim();
    if pattern.is_empty() {
        return false;
    }

    let candidate = match rule.field {
        MatchField::WindowTitle => &window.window_title,
        MatchField::AppName => &window.app_name,
        MatchField::ProcessName => &window.process_name,
        MatchField::ExecutablePath => &window.executable_path,
        MatchField::BundleId => &window.bundle_id,
    };

    matches_pattern(candidate, pattern, rule.kind, rule.case_sensitive)
}

fn matches_pattern(candidate: &str, pattern: &str, kind: MatchKind, case_sensitive: bool) -> bool {
    let left = if case_sensitive {
        candidate.to_string()
    } else {
        candidate.to_lowercase()
    };
    let right = if case_sensitive {
        pattern.to_string()
    } else {
        pattern.to_lowercase()
    };

    match kind {
        MatchKind::Contains => left.contains(&right),
        MatchKind::Exact => left == right,
        MatchKind::Prefix => left.starts_with(&right),
        MatchKind::Suffix => left.ends_with(&right),
    }
}

fn choose_process_name(display_name: &str, window: &WindowInfo) -> String {
    non_empty(display_name.to_string())
        .or_else(|| non_empty(window.app_name.clone()))
        .or_else(|| non_empty(window.process_name.clone()))
        .or_else(|| non_empty(window.window_title.clone()))
        .unwrap_or_else(|| "Unknown".to_string())
}

fn choose_extend(rule_extend: &str, default_extend: &str) -> String {
    non_empty(rule_extend.to_string())
        .or_else(|| non_empty(default_extend.to_string()))
        .unwrap_or_default()
}

fn perform_fetch(http: &Client, config: &CoreConfig) -> ApiCallResult {
    let url = build_online_status_url(&config.base_url);
    let response = http.get(url).headers(build_headers(config)).send();

    match response {
        Ok(resp) => parse_response(resp.status().as_u16(), resp.text().unwrap_or_default()),
        Err(err) => ApiCallResult {
            success: false,
            error_message: err.to_string(),
            ..ApiCallResult::default()
        },
    }
}

fn perform_push(http: &Client, config: &CoreConfig, update: StatusUpdate) -> ApiCallResult {
    if !is_valid_gt_token(&config.token) {
        return ApiCallResult {
            success: false,
            error_message: "push_status requires a gt_ admin token".to_string(),
            ..ApiCallResult::default()
        };
    }

    let url = build_online_status_url(&config.base_url);
    let payload: PushStatusBody = update.into();

    let response = http
        .post(url)
        .headers(build_headers(config))
        .json(&payload)
        .send();

    match response {
        Ok(resp) => parse_response(resp.status().as_u16(), resp.text().unwrap_or_default()),
        Err(err) => ApiCallResult {
            success: false,
            error_message: err.to_string(),
            ..ApiCallResult::default()
        },
    }
}

fn is_valid_gt_token(token: &str) -> bool {
    let trimmed = token.trim();
    trimmed.starts_with("gt_") && trimmed.len() > 3
}

fn parse_response(http_status: u16, body: String) -> ApiCallResult {
    let parsed = serde_json::from_str::<ApiEnvelope<ApiStatusSnapshot>>(&body);
    match parsed {
        Ok(envelope) => ApiCallResult {
            success: (200..300).contains(&http_status),
            http_status: i32::from(http_status),
            code: envelope.code,
            biz_err: envelope.biz_err,
            message: envelope.message,
            request_id: envelope
                .meta
                .as_ref()
                .map(|meta| meta.request_id.clone())
                .unwrap_or_default(),
            response_timestamp: envelope
                .meta
                .as_ref()
                .map(|meta| meta.timestamp.clone())
                .unwrap_or_default(),
            snapshot: envelope.data.map(Into::into),
            ..ApiCallResult::default()
        },
        Err(err) => ApiCallResult {
            success: false,
            http_status: i32::from(http_status),
            error_message: format!("failed to parse response: {err}; body={body}"),
            ..ApiCallResult::default()
        },
    }
}

fn non_empty(value: String) -> Option<String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn clean_media(media: MediaInfo) -> Option<MediaInfo> {
    let title = media.title.trim().to_string();
    let artist = media.artist.trim().to_string();
    let thumbnail = media.thumbnail.trim().to_string();

    if title.is_empty() && artist.is_empty() && thumbnail.is_empty() {
        None
    } else {
        Some(MediaInfo {
            title,
            artist,
            thumbnail,
        })
    }
}

uniffi::setup_scaffolding!();

#[cfg(test)]
mod tests {
    use super::{
        CoreConfig, MatchEngineConfig, MatchField, MatchKind, ReportPolicy, ResolveStatusInput,
        WindowInfo, WindowMatchRule, build_headers, build_online_status_url,
        resolve_status_update_inner,
    };
    use reqwest::header::AUTHORIZATION;

    #[test]
    fn base_url_is_normalized_to_online_status() {
        assert_eq!(
            build_online_status_url("https://example.com"),
            "https://example.com/api/v2/onlineStatus"
        );
        assert_eq!(
            build_online_status_url("https://example.com/api/v2"),
            "https://example.com/api/v2/onlineStatus"
        );
        assert_eq!(
            build_online_status_url("https://example.com/api/v2/onlineStatus"),
            "https://example.com/api/v2/onlineStatus"
        );
    }

    #[test]
    fn gt_token_is_sent_without_bearer_prefix() {
        let headers = build_headers(&CoreConfig {
            token: "gt_example".to_string(),
            ..CoreConfig::default()
        });

        assert_eq!(
            headers.get(AUTHORIZATION).unwrap().to_str().unwrap(),
            "gt_example"
        );
    }

    #[test]
    fn non_gt_token_is_not_sent() {
        let headers = build_headers(&CoreConfig {
            token: "jwt_like_value".to_string(),
            ..CoreConfig::default()
        });

        assert!(headers.get(AUTHORIZATION).is_none());
    }

    #[test]
    fn allow_rule_maps_display_name_and_extend() {
        let result = resolve_status_update_inner(
            MatchEngineConfig {
                default_report: true,
                default_display_name: String::new(),
                default_extend: String::new(),
                rules: vec![WindowMatchRule {
                    id: "kitty".to_string(),
                    enabled: true,
                    field: MatchField::AppName,
                    kind: MatchKind::Exact,
                    pattern: "kitty".to_string(),
                    case_sensitive: false,
                    report_policy: ReportPolicy::Allow,
                    display_name: "Kitty".to_string(),
                    extend: "没准正在 yay -Syyu，希望不要 grub>".to_string(),
                }],
            },
            ResolveStatusInput {
                window: WindowInfo {
                    app_name: "kitty".to_string(),
                    ..WindowInfo::default()
                },
                media: None,
                timestamp: Some(1),
            },
        );

        assert!(result.should_report);
        assert_eq!(result.matched_rule_id, "kitty");
        assert_eq!(result.process, "Kitty");
        assert_eq!(result.extend, "没准正在 yay -Syyu，希望不要 grub>");
        assert!(result.update.is_some());
    }

    #[test]
    fn deny_rule_blocks_reporting() {
        let result = resolve_status_update_inner(
            MatchEngineConfig {
                default_report: true,
                default_display_name: String::new(),
                default_extend: String::new(),
                rules: vec![WindowMatchRule {
                    id: "ignore".to_string(),
                    enabled: true,
                    field: MatchField::WindowTitle,
                    kind: MatchKind::Contains,
                    pattern: "secret".to_string(),
                    case_sensitive: false,
                    report_policy: ReportPolicy::Deny,
                    display_name: String::new(),
                    extend: String::new(),
                }],
            },
            ResolveStatusInput {
                window: WindowInfo {
                    window_title: "my secret notes".to_string(),
                    app_name: "editor".to_string(),
                    ..WindowInfo::default()
                },
                media: None,
                timestamp: None,
            },
        );

        assert!(!result.should_report);
        assert!(result.update.is_none());
    }
}



