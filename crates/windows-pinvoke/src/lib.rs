use std::ffi::{CStr, CString, c_char};
use std::ptr;

use serde::Serialize;
use statusshare_core::{
    ApiCallResult, CoreConfig, MatchEngineConfig, PersistedConfig, ResolveStatusInput,
    ResolveStatusResult, StatusShareClient, StatusUpdate, default_config_file_path,
    default_persisted_config, load_persisted_config, resolve_status_update, save_persisted_config,
};

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
pub extern "C" fn ss_string_free(ptr_value: *mut c_char) {
    if ptr_value.is_null() {
        return;
    }

    unsafe {
        drop(CString::from_raw(ptr_value));
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
