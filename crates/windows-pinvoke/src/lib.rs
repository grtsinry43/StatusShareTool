use std::ffi::{CStr, CString, c_char};
use std::path::Path;
use std::ptr;

use serde::Serialize;
use statusshare_core::{
    ApiCallResult, CoreConfig, MatchEngineConfig, MediaInfo, PersistedConfig, ResolveStatusInput,
    ResolveStatusResult, SchedulerPlanResult, SchedulerSnapshot, StatusShareClient, StatusUpdate,
    WindowInfo, default_config_file_path, default_persisted_config, load_persisted_config,
    mark_status_pushed, plan_status_update, resolve_status_update, save_persisted_config,
};
use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager;
use windows::Win32::Foundation::{CloseHandle, HWND, MAX_PATH};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
};
use windows::core::HSTRING;

#[unsafe(no_mangle)]
pub extern "C" fn ss_fetch_status(config_json: *const c_char) -> *mut c_char {
    with_config(config_json, |config| {
        let client = StatusShareClient::new(config);
        client.fetch_status()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn ss_push_status(
    config_json: *const c_char,
    update_json: *const c_char,
) -> *mut c_char {
    if config_json.is_null() || update_json.is_null() {
        return to_json_ptr(error_result("config_json or update_json was null"));
    }

    let config = match parse_json_ptr::<CoreConfig>(config_json) {
        Ok(config) => config,
        Err(err) => return to_json_ptr(error_result(&err)),
    };
    let update = match parse_json_ptr::<StatusUpdate>(update_json) {
        Ok(update) => update,
        Err(err) => return to_json_ptr(error_result(&err)),
    };

    let client = StatusShareClient::new(config);
    to_json_ptr(client.push_status(update))
}

#[unsafe(no_mangle)]
pub extern "C" fn ss_default_config_file_path() -> *mut c_char {
    to_string_ptr(default_config_file_path())
}

#[unsafe(no_mangle)]
pub extern "C" fn ss_default_persisted_config() -> *mut c_char {
    to_json_ptr(default_persisted_config())
}

#[unsafe(no_mangle)]
pub extern "C" fn ss_load_persisted_config(path: *const c_char) -> *mut c_char {
    let path = parse_string_ptr(path).unwrap_or_default();
    to_json_ptr(load_persisted_config(path))
}

#[unsafe(no_mangle)]
pub extern "C" fn ss_save_persisted_config(
    path: *const c_char,
    config_json: *const c_char,
) -> *mut c_char {
    if config_json.is_null() {
        return to_json_ptr(error_result("config_json was null"));
    }

    let path = parse_string_ptr(path).unwrap_or_default();
    match parse_json_ptr::<PersistedConfig>(config_json) {
        Ok(config) => to_json_ptr(save_persisted_config(path, config)),
        Err(err) => to_json_ptr(error_result(&err)),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ss_resolve_status_update(
    matching_json: *const c_char,
    input_json: *const c_char,
) -> *mut c_char {
    if matching_json.is_null() || input_json.is_null() {
        return to_json_ptr(resolve_error_result("matching_json or input_json was null"));
    }

    let matching = match parse_json_ptr::<MatchEngineConfig>(matching_json) {
        Ok(value) => value,
        Err(err) => return to_json_ptr(resolve_error_result(&err)),
    };
    let input = match parse_json_ptr::<ResolveStatusInput>(input_json) {
        Ok(value) => value,
        Err(err) => return to_json_ptr(resolve_error_result(&err)),
    };

    to_json_ptr(resolve_status_update(matching, input))
}

#[unsafe(no_mangle)]
pub extern "C" fn ss_plan_status_update(
    snapshot_json: *const c_char,
    update_json: *const c_char,
    now_secs: i64,
) -> *mut c_char {
    if snapshot_json.is_null() {
        return to_json_ptr(schedule_error_result());
    }

    let snapshot = match parse_json_ptr::<SchedulerSnapshot>(snapshot_json) {
        Ok(value) => value,
        Err(_) => return to_json_ptr(schedule_error_result()),
    };

    let update = if update_json.is_null() {
        None
    } else {
        match parse_json_ptr::<Option<StatusUpdate>>(update_json) {
            Ok(value) => value,
            Err(_) => match parse_json_ptr::<StatusUpdate>(update_json) {
                Ok(value) => Some(value),
                Err(_) => return to_json_ptr(schedule_error_result()),
            },
        }
    };

    to_json_ptr(plan_status_update(snapshot, update, now_secs))
}

#[unsafe(no_mangle)]
pub extern "C" fn ss_mark_status_pushed(
    snapshot_json: *const c_char,
    fingerprint_json: *const c_char,
    now_secs: i64,
) -> *mut c_char {
    if snapshot_json.is_null() || fingerprint_json.is_null() {
        return to_json_ptr(SchedulerSnapshot::default());
    }

    let snapshot = match parse_json_ptr::<SchedulerSnapshot>(snapshot_json) {
        Ok(value) => value,
        Err(_) => return to_json_ptr(SchedulerSnapshot::default()),
    };
    let fingerprint = match parse_string_ptr(fingerprint_json) {
        Ok(value) => value,
        Err(_) => return to_json_ptr(SchedulerSnapshot::default()),
    };

    to_json_ptr(mark_status_pushed(snapshot, fingerprint, now_secs))
}

#[unsafe(no_mangle)]
pub extern "C" fn ss_detect_active_window() -> *mut c_char {
    match detect_active_window() {
        Ok(window) => to_json_ptr(WindowDetectResult {
            success: true,
            backend: "win32-foreground".to_string(),
            error_message: String::new(),
            window: Some(window),
        }),
        Err(err) => to_json_ptr(window_error_result(&err)),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ss_detect_media() -> *mut c_char {
    match detect_media() {
        Ok(media) => to_json_ptr(MediaDetectResult {
            success: true,
            backend: "winrt-gsmtc".to_string(),
            error_message: String::new(),
            media,
        }),
        Err(err) => to_json_ptr(media_error_result(&err)),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ss_string_free(ptr_value: *mut c_char) {
    if ptr_value.is_null() {
        return;
    }

    unsafe {
        drop(CString::from_raw(ptr_value));
    }
}

#[derive(Serialize)]
struct WindowDetectResult {
    success: bool,
    backend: String,
    error_message: String,
    window: Option<WindowInfo>,
}

#[derive(Serialize)]
struct MediaDetectResult {
    success: bool,
    backend: String,
    error_message: String,
    media: Option<MediaInfo>,
}

fn detect_active_window() -> Result<WindowInfo, String> {
    unsafe {
        let hwnd: HWND = GetForegroundWindow();
        if hwnd.0.is_null() {
            return Ok(WindowInfo::default());
        }

        let title_len = GetWindowTextLengthW(hwnd);
        let mut title_buf = vec![0u16; title_len as usize + 1];
        if title_len > 0 {
            let _ = GetWindowTextW(hwnd, &mut title_buf);
        }
        let window_title = utf16_to_string(&title_buf);

        let mut process_id = 0u32;
        let _ = GetWindowThreadProcessId(hwnd, Some(&mut process_id));
        let executable_path = executable_path_from_pid(process_id);
        let process_name = process_name_from_executable_path(&executable_path);
        let app_name = process_name.clone();
        let bundle_id = executable_path.clone();

        Ok(WindowInfo {
            window_title,
            app_name,
            process_name,
            executable_path,
            bundle_id,
        })
    }
}

fn detect_media() -> Result<Option<MediaInfo>, String> {
    let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .map_err(|err| err.to_string())?
        .get()
        .map_err(|err| err.to_string())?;

    let Some(session) = manager.GetCurrentSession().ok() else {
        return Ok(None);
    };

    let properties = session
        .TryGetMediaPropertiesAsync()
        .map_err(|err| err.to_string())?
        .get()
        .map_err(|err| err.to_string())?;

    let title = hstring_to_string(&properties.Title().map_err(|err| err.to_string())?);
    let artist = hstring_to_string(&properties.Artist().map_err(|err| err.to_string())?);
    let thumbnail = String::new();

    if title.is_empty() && artist.is_empty() && thumbnail.is_empty() {
        Ok(None)
    } else {
        Ok(Some(MediaInfo { title, artist, thumbnail }))
    }
}

fn hstring_to_string(value: &HSTRING) -> String {
    value.to_string_lossy().trim().to_string()
}

fn process_name_from_executable_path(executable_path: &str) -> String {
    Path::new(executable_path)
        .file_stem()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn executable_path_from_pid(process_id: u32) -> String {
    if process_id == 0 {
        return String::new();
    }

    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok();
        let Some(process) = process else {
            return String::new();
        };

        let mut capacity = MAX_PATH as usize;
        loop {
            let mut buffer = vec![0u16; capacity];
            let mut size = capacity as u32;
            let ok = windows::Win32::System::Threading::QueryFullProcessImageNameW(
                process,
                windows::Win32::System::Threading::PROCESS_NAME_FORMAT(0),
                windows::core::PWSTR(buffer.as_mut_ptr()),
                &mut size,
            )
            .is_ok();
            if ok {
                let _ = CloseHandle(process);
                return String::from_utf16_lossy(&buffer[..size as usize]).trim().to_string();
            }
            if capacity >= 32768 {
                let _ = CloseHandle(process);
                return String::new();
            }
            capacity *= 2;
        }
    }
}

fn utf16_to_string(buffer: &[u16]) -> String {
    let nul = buffer.iter().position(|value| *value == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..nul]).trim().to_string()
}

fn window_error_result(message: &str) -> WindowDetectResult {
    WindowDetectResult {
        success: false,
        backend: "win32-foreground".to_string(),
        error_message: message.to_string(),
        window: None,
    }
}

fn media_error_result(message: &str) -> MediaDetectResult {
    MediaDetectResult {
        success: false,
        backend: "winrt-gsmtc".to_string(),
        error_message: message.to_string(),
        media: None,
    }
}

fn with_config<F>(config_json: *const c_char, f: F) -> *mut c_char
where
    F: FnOnce(CoreConfig) -> ApiCallResult,
{
    if config_json.is_null() {
        return to_json_ptr(error_result("config_json was null"));
    }

    match parse_json_ptr::<CoreConfig>(config_json) {
        Ok(config) => to_json_ptr(f(config)),
        Err(err) => to_json_ptr(error_result(&err)),
    }
}

fn parse_json_ptr<T>(ptr_value: *const c_char) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    let raw = parse_string_ptr(ptr_value)?;
    serde_json::from_str(&raw).map_err(|err| err.to_string())
}

fn parse_string_ptr(ptr_value: *const c_char) -> Result<String, String> {
    if ptr_value.is_null() {
        return Err("string pointer was null".to_string());
    }

    let raw = unsafe { CStr::from_ptr(ptr_value) }
        .to_str()
        .map_err(|err| err.to_string())?;

    Ok(raw.to_string())
}

fn to_json_ptr<T>(value: T) -> *mut c_char
where
    T: Serialize,
{
    match serde_json::to_string(&value) {
        Ok(json) => CString::new(json)
            .map(CString::into_raw)
            .unwrap_or(ptr::null_mut()),
        Err(_) => ptr::null_mut(),
    }
}

fn to_string_ptr(value: String) -> *mut c_char {
    CString::new(value)
        .map(CString::into_raw)
        .unwrap_or(ptr::null_mut())
}

fn error_result(message: &str) -> ApiCallResult {
    ApiCallResult {
        success: false,
        error_message: message.to_string(),
        ..ApiCallResult::default()
    }
}

fn resolve_error_result(message: &str) -> ResolveStatusResult {
    ResolveStatusResult {
        should_report: false,
        matched_rule_id: String::new(),
        process: String::new(),
        extend: String::new(),
        media: None,
        update: None,
        error_message: message.to_string(),
    }
}

fn schedule_error_result() -> SchedulerPlanResult {
    SchedulerPlanResult {
        decision: statusshare_core::ScheduleDecision {
            should_push: false,
            reason: statusshare_core::ReportReason::None,
            fingerprint: String::new(),
        },
        snapshot: SchedulerSnapshot {
            heartbeat_interval_secs: 5,
            last_fingerprint: String::new(),
            last_report_at: 0,
        },
    }
}


