//! Minimal C ABI used by the native Apple applications.

use std::{
    ffi::{CStr, CString, c_char},
    panic::{AssertUnwindSafe, catch_unwind},
    time::Duration,
};

use open_edifier::{Device, ModelId, PlaybackAction, Source, connect_host, discover};
use serde::Deserialize;
use serde_json::{Value, json};

const CONTROL_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_DISCOVERY_MS: u64 = 2_000;
const MAX_DISCOVERY_MS: u64 = 10_000;

#[derive(Debug, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
enum Command {
    Discover {
        #[serde(default = "default_discovery_ms")]
        timeout_ms: u64,
    },
    Status {
        #[serde(flatten)]
        target: Target,
    },
    Source {
        #[serde(flatten)]
        target: Target,
        source: String,
    },
    Volume {
        #[serde(flatten)]
        target: Target,
        level: u8,
    },
    Equalizer {
        #[serde(flatten)]
        target: Target,
        preset: u8,
    },
    Playback {
        #[serde(flatten)]
        target: Target,
        action: PlaybackAction,
    },
}

#[derive(Debug, Deserialize)]
struct Target {
    host: String,
    model: String,
    port: Option<u16>,
}

fn default_discovery_ms() -> u64 {
    DEFAULT_DISCOVERY_MS
}

fn execute(request: &str) -> Result<Value, String> {
    let command: Command =
        serde_json::from_str(request).map_err(|error| format!("无效请求：{error}"))?;

    match command {
        Command::Discover { timeout_ms } => {
            if !(1..=MAX_DISCOVERY_MS).contains(&timeout_ms) {
                return Err(format!("发现超时必须在 1..={MAX_DISCOVERY_MS} 毫秒内"));
            }
            let devices =
                discover(Duration::from_millis(timeout_ms)).map_err(|error| error.to_string())?;
            serde_json::to_value(devices).map_err(|error| error.to_string())
        }
        Command::Status { target } => {
            let mut device = connect_target(target)?;
            to_value(device.status())
        }
        Command::Source { target, source } => {
            let mut device = connect_target(target)?;
            to_value(device.set_source(Source::new(source)))
        }
        Command::Volume { target, level } => {
            let mut device = connect_target(target)?;
            to_value(device.set_volume(level))
        }
        Command::Equalizer { target, preset } => {
            let mut device = connect_target(target)?;
            to_value(device.set_eq_preset(preset))
        }
        Command::Playback { target, action } => {
            let mut device = connect_target(target)?;
            device.playback(action).map_err(|error| error.to_string())?;
            to_value(device.status())
        }
    }
}

fn connect_target(target: Target) -> Result<Box<dyn Device>, String> {
    if target.host.trim().is_empty() {
        return Err("设备地址不能为空".to_owned());
    }
    connect_host(
        &ModelId::new(target.model),
        target.host,
        target.port,
        CONTROL_TIMEOUT,
    )
    .map_err(|error| error.to_string())
}

fn to_value<T: serde::Serialize>(result: open_edifier::Result<T>) -> Result<Value, String> {
    let value = result.map_err(|error| error.to_string())?;
    serde_json::to_value(value).map_err(|error| error.to_string())
}

unsafe fn handle(request: *const c_char) -> String {
    let result = catch_unwind(AssertUnwindSafe(|| {
        if request.is_null() {
            return Err("请求指针不能为空".to_owned());
        }
        // SAFETY: The caller contract requires a valid, NUL-terminated string for this call.
        let request = unsafe { CStr::from_ptr(request) }
            .to_str()
            .map_err(|_| "请求必须是 UTF-8".to_owned())?;
        execute(request)
    }))
    .unwrap_or_else(|_| Err("Rust 控制层发生意外错误".to_owned()));

    match result {
        Ok(data) => json!({"ok": true, "data": data}).to_string(),
        Err(error) => json!({"ok": false, "error": error}).to_string(),
    }
}

/// Executes one JSON command and returns an owned JSON response string.
///
/// The caller must release the result with [`open_edifier_string_free`].
///
/// # Safety
///
/// `request` must point to a valid, NUL-terminated string for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn open_edifier_command(request: *const c_char) -> *mut c_char {
    // SAFETY: The caller upholds the pointer contract documented above.
    CString::new(unsafe { handle(request) })
        .expect("serialized JSON never contains an interior NUL")
        .into_raw()
}

/// Releases a string returned by [`open_edifier_command`].
///
/// # Safety
///
/// `value` must be null or an unfreed pointer returned by [`open_edifier_command`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn open_edifier_string_free(value: *mut c_char) {
    if value.is_null() {
        return;
    }
    // SAFETY: The caller must only pass a pointer returned by open_edifier_command once.
    unsafe { drop(CString::from_raw(value)) };
}

#[cfg(test)]
mod tests {
    use std::ffi::{CStr, CString};

    use super::{open_edifier_command, open_edifier_string_free};

    #[test]
    fn ffi_returns_structured_errors() {
        let request = CString::new(r#"{"command":"nope"}"#).unwrap();
        // SAFETY: The CString is valid for the duration of the call.
        let response = unsafe { open_edifier_command(request.as_ptr()) };
        // SAFETY: The bridge returned a valid string that remains owned until freed below.
        let response_text = unsafe { CStr::from_ptr(response) }.to_str().unwrap();
        assert!(response_text.contains(r#""ok":false"#));
        assert!(response_text.contains("无效请求"));
        // SAFETY: The response came from the bridge and has not been freed yet.
        unsafe { open_edifier_string_free(response) };
    }
}
