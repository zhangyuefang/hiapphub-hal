use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};

// ---------- open_file ----------
#[derive(Deserialize)]
pub struct OpenFileParams {
    pub title: Option<String>, pub filters: Option<Vec<FilterDef>>,
    pub multiple: Option<bool>, pub default_path: Option<String>,
}
#[derive(Deserialize)]
pub struct FilterDef { pub name: String, pub extensions: Vec<String> }

hap_fn!(hap_dialog_open_file, OpenFileParams, |p| {
    let mut dialog = rfd::FileDialog::new();
    if let Some(ref t) = p.title { dialog = dialog.set_title(t); }
    if let Some(ref dp) = p.default_path { dialog = dialog.set_directory(dp); }
    if let Some(ref filters) = p.filters {
        for f in filters {
            let exts: Vec<&str> = f.extensions.iter().map(|s| s.as_str()).collect();
            dialog = dialog.add_filter(&f.name, &exts);
        }
    }
    if p.multiple.unwrap_or(false) {
        let paths = dialog.pick_files().unwrap_or_default();
        let result: Vec<String> = paths.iter().map(|p| p.to_string_lossy().to_string()).collect();
        Ok(json!(result))
    } else {
        match dialog.pick_file() {
            Some(path) => Ok(json!([path.to_string_lossy().to_string()])),
            None => Ok(json!([])),
        }
    }
});

// ---------- save_file ----------
#[derive(Deserialize)]
pub struct SaveFileParams {
    pub title: Option<String>, pub filters: Option<Vec<FilterDef>>,
    pub default_name: Option<String>, pub default_path: Option<String>,
}
hap_fn!(hap_dialog_save_file, SaveFileParams, |p| {
    let mut dialog = rfd::FileDialog::new();
    if let Some(ref t) = p.title { dialog = dialog.set_title(t); }
    if let Some(ref dp) = p.default_path { dialog = dialog.set_directory(dp); }
    if let Some(ref dn) = p.default_name { dialog = dialog.set_file_name(dn); }
    if let Some(ref filters) = p.filters {
        for f in filters {
            let exts: Vec<&str> = f.extensions.iter().map(|s| s.as_str()).collect();
            dialog = dialog.add_filter(&f.name, &exts);
        }
    }
    match dialog.save_file() {
        Some(path) => Ok(json!(path.to_string_lossy().to_string())),
        None => Ok(json!("")),
    }
});

// ---------- open_directory ----------
#[derive(Deserialize)]
pub struct OpenDirParams {
    pub title: Option<String>, pub default_path: Option<String>, pub multiple: Option<bool>,
}
hap_fn!(hap_dialog_open_directory, OpenDirParams, |p| {
    let mut dialog = rfd::FileDialog::new();
    if let Some(ref t) = p.title { dialog = dialog.set_title(t); }
    if let Some(ref dp) = p.default_path { dialog = dialog.set_directory(dp); }
    if p.multiple.unwrap_or(false) {
        let paths = dialog.pick_folders().unwrap_or_default();
        let result: Vec<String> = paths.iter().map(|p| p.to_string_lossy().to_string()).collect();
        Ok(json!(result))
    } else {
        match dialog.pick_folder() {
            Some(path) => Ok(json!([path.to_string_lossy().to_string()])),
            None => Ok(json!([])),
        }
    }
});

// ---------- message_box ----------
#[derive(Deserialize)]
pub struct MessageBoxParams {
    pub title: String, pub message: String,
    pub r#type: Option<String>, pub buttons: Option<Vec<String>>,
    #[allow(dead_code)] pub default_button: Option<i32>, #[allow(dead_code)] pub icon: Option<String>,
}
hap_fn!(hap_dialog_message_box, MessageBoxParams, |p| {
    let level = match p.r#type.as_deref() {
        Some("warning") => rfd::MessageLevel::Warning,
        Some("error") => rfd::MessageLevel::Error,
        _ => rfd::MessageLevel::Info,
    };
    let btns = match p.buttons.as_ref().map(|b| b.len()) {
        Some(2..) => rfd::MessageButtons::OkCancelCustom(
            p.buttons.as_ref().unwrap()[0].clone(),
            p.buttons.as_ref().unwrap()[1].clone(),
        ),
        _ => rfd::MessageButtons::Ok,
    };
    let result = rfd::MessageDialog::new()
        .set_title(&p.title).set_description(&p.message)
        .set_level(level).set_buttons(btns).show();
    let (idx, label) = match result {
        rfd::MessageDialogResult::Ok => (0, "Ok".to_string()),
        rfd::MessageDialogResult::Cancel => (1, "Cancel".to_string()),
        rfd::MessageDialogResult::Yes => (0, "Yes".to_string()),
        rfd::MessageDialogResult::No => (1, "No".to_string()),
        rfd::MessageDialogResult::Custom(s) => {
            let i = p.buttons.as_ref().and_then(|bs| bs.iter().position(|b| b == &s)).unwrap_or(0);
            (i as i32, s)
        }
    };
    Ok(json!({"button_index": idx, "button_label": label}))
});

// ---------- confirm ----------
#[derive(Deserialize)]
pub struct ConfirmParams {
    pub title: String, pub message: String,
    pub ok_label: Option<String>, pub cancel_label: Option<String>,
    #[allow(dead_code)] pub r#type: Option<String>,
}
hap_fn!(hap_dialog_confirm, ConfirmParams, |p| {
    let ok = p.ok_label.as_deref().unwrap_or("确定");
    let cancel = p.cancel_label.as_deref().unwrap_or("取消");
    let result = rfd::MessageDialog::new()
        .set_title(&p.title).set_description(&p.message)
        .set_buttons(rfd::MessageButtons::OkCancelCustom(ok.to_string(), cancel.to_string()))
        .show();
    Ok(json!(matches!(result, rfd::MessageDialogResult::Custom(ref s) if s == ok) || matches!(result, rfd::MessageDialogResult::Ok)))
});

// ---------- input_box ----------
#[derive(Deserialize)]
pub struct InputBoxParams {
    pub title: String, pub message: Option<String>,
    pub default_value: Option<String>, #[allow(dead_code)] pub placeholder: Option<String>,
    pub password: Option<bool>, #[allow(dead_code)] pub multiline: Option<bool>,
}
hap_fn!(hap_dialog_input_box, InputBoxParams, |p| {
    #[cfg(target_os = "macos")]
    {
        let msg = p.message.as_deref().unwrap_or("");
        let default = p.default_value.as_deref().unwrap_or("");
        let hidden = if p.password.unwrap_or(false) { " with hidden answer" } else { "" };
        let script = format!(
            r#"display dialog "{}" default answer "{}" with title "{}"{} buttons {{"取消","确定"}} default button "确定""#,
            msg.replace('"', "\\\""), default.replace('"', "\\\""), p.title.replace('"', "\\\""), hidden
        );
        let output = std::process::Command::new("osascript").arg("-e").arg(&script).output();
        match output {
            Ok(o) if o.status.success() => {
                let s = String::from_utf8_lossy(&o.stdout);
                let value = s.split("text returned:").nth(1).unwrap_or("").trim().to_string();
                Ok(json!({"confirmed": true, "value": value}))
            }
            _ => Ok(json!({"confirmed": false, "value": ""})),
        }
    }
    #[cfg(not(target_os = "macos"))]
    { let _ = &p; Ok(json!({"confirmed": false, "value": ""})) }
});

// ---------- color_picker ----------
#[derive(Deserialize)]
pub struct ColorPickerParams { #[allow(dead_code)] pub default_color: Option<String>, #[allow(dead_code)] pub show_alpha: Option<bool> }
hap_fn!(hap_dialog_color_picker, ColorPickerParams, |_p| {
    #[cfg(target_os = "macos")]
    {
        let script = r#"choose color"#;
        let output = std::process::Command::new("osascript").arg("-e").arg(script).output();
        match output {
            Ok(o) if o.status.success() => {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                let parts: Vec<u16> = s.split(", ").filter_map(|p| p.trim().parse().ok()).collect();
                if parts.len() >= 3 {
                    let hex = format!("#{:02X}{:02X}{:02X}", parts[0] / 256, parts[1] / 256, parts[2] / 256);
                    Ok(json!(hex))
                } else { Ok(json!("")) }
            }
            _ => Ok(json!("")),
        }
    }
    #[cfg(not(target_os = "macos"))]
    { let _ = &p; Ok(json!("")) }
});

// ---------- progress ----------
#[allow(dead_code)]
struct ProgressEntry {
    title: String,
    message: String,
    value: f64,
    cancellable: bool,
    indeterminate: bool,
    cancelled: bool,
}
static PROGRESS_MAP: LazyLock<Mutex<HashMap<String, ProgressEntry>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static PROGRESS_SEQ: AtomicU64 = AtomicU64::new(1);

#[derive(Deserialize)]
pub struct ProgressParams {
    pub title: String, pub message: Option<String>,
    pub cancellable: Option<bool>, pub indeterminate: Option<bool>,
    pub callback_id: Option<String>,
}
hap_fn!(hap_dialog_progress, ProgressParams, |p| {
    let did = format!("prog_{}", PROGRESS_SEQ.fetch_add(1, Ordering::Relaxed));
    PROGRESS_MAP.lock().unwrap().insert(did.clone(), ProgressEntry {
        title: p.title.clone(),
        message: p.message.clone().unwrap_or_default(),
        value: 0.0,
        cancellable: p.cancellable.unwrap_or(false),
        indeterminate: p.indeterminate.unwrap_or(false),
        cancelled: false,
    });
    Ok(json!({"dialog_id": did}))
});

// ---------- progress_update ----------
#[derive(Deserialize)]
pub struct ProgressUpdateParams { pub dialog_id: String, pub value: f64, pub message: Option<String> }
hap_fn!(hap_dialog_progress_update, ProgressUpdateParams, |p| {
    let mut map = PROGRESS_MAP.lock().unwrap();
    let entry = map.get_mut(&p.dialog_id)
        .ok_or_else(|| HapError::invalid_param(format!("dialog_id '{}' not found", p.dialog_id)))?;
    if entry.cancelled {
        return Ok(json!({"cancelled": true}));
    }
    entry.value = p.value;
    if let Some(ref msg) = p.message {
        entry.message = msg.clone();
    }
    Ok(json!({"cancelled": false, "value": entry.value}))
});

// ---------- progress_close ----------
#[derive(Deserialize)]
pub struct ProgressCloseParams { pub dialog_id: String }
hap_fn!(hap_dialog_progress_close, ProgressCloseParams, |p| {
    PROGRESS_MAP.lock().unwrap().remove(&p.dialog_id);
    Ok(json!(true))
});

// ---------- date_picker ----------
#[derive(Deserialize)]
pub struct DatePickerParams {
    pub title: Option<String>, pub default_date: Option<String>,
    #[allow(dead_code)] pub min_date: Option<String>, #[allow(dead_code)] pub max_date: Option<String>,
}
hap_fn!(hap_dialog_date_picker, DatePickerParams, |p| {
    #[cfg(target_os = "macos")]
    {
        let title = p.title.as_deref().unwrap_or("Select Date");
        let default = p.default_date.as_deref().unwrap_or("");
        let default_clause = if default.is_empty() {
            "set theDate to current date".to_string()
        } else {
            format!(r#"set theDate to date "{}""#, default)
        };
        let script = format!(
            r#"
            {default_clause}
            set theResult to (choose from list {{"OK"}} with title "{title}" with prompt "Enter date (YYYY-MM-DD):" default items {{"OK"}})
            if theResult is false then
                return ""
            end if
            set y to year of theDate as integer
            set m to month of theDate as integer
            set d to day of theDate as integer
            set mStr to text -2 thru -1 of ("0" & m)
            set dStr to text -2 thru -1 of ("0" & d)
            return (y as text) & "-" & mStr & "-" & dStr
            "#
        );
        let output = std::process::Command::new("osascript").arg("-e").arg(&script).output();
        match output {
            Ok(o) if o.status.success() => {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                Ok(json!(s))
            }
            _ => Ok(json!("")),
        }
    }
    #[cfg(not(target_os = "macos"))]
    { let _ = &p; Ok(json!("")) }
});

// ---------- list_select ----------
#[derive(Deserialize)]
pub struct ListSelectParams {
    pub title: String, pub items: Vec<Value>,
    pub multiple: Option<bool>, #[allow(dead_code)] pub search: Option<bool>,
}
hap_fn!(hap_dialog_list_select, ListSelectParams, |p| {
    #[cfg(target_os = "macos")]
    {
        let item_strs: Vec<String> = p.items.iter().map(|v| {
            v.as_str().unwrap_or(&v.to_string()).to_string()
        }).collect();
        let items_list = item_strs.iter().map(|s| format!("\"{}\"", s.replace('"', "\\\""))).collect::<Vec<_>>().join(", ");
        let multi = if p.multiple.unwrap_or(false) { " with multiple selections allowed" } else { "" };
        let script = format!(
            r#"choose from list {{{items_list}}} with title "{}" with prompt "请选择："{multi}"#,
            p.title.replace('"', "\\\"")
        );
        let output = std::process::Command::new("osascript").arg("-e").arg(&script).output();
        match output {
            Ok(o) if o.status.success() => {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if s == "false" { return Ok(json!([])); }
                let selected: Vec<Value> = s.split(", ").map(|s| json!(s.trim())).collect();
                Ok(json!(selected))
            }
            _ => Ok(json!([])),
        }
    }
    #[cfg(not(target_os = "macos"))]
    { let _ = &p; Ok(json!([])) }
});

// ---------- notify ----------
#[derive(Deserialize)]
pub struct NotifyParams {
    pub message: String, pub r#type: Option<String>,
    #[allow(dead_code)] pub duration_ms: Option<u32>,
}
hap_fn!(hap_dialog_notify, NotifyParams, |p| {
    #[cfg(target_os = "macos")]
    {
        let title = p.r#type.as_deref().unwrap_or("通知");
        let script = format!(
            r#"display notification "{}" with title "{}""#,
            p.message.replace('"', "\\\""), title.replace('"', "\\\"")
        );
        std::process::Command::new("osascript").arg("-e").arg(&script).output().ok();
        Ok(json!(null))
    }
    #[cfg(not(target_os = "macos"))]
    { let _ = &p; Ok(json!(null)) }
});

// ---------- time_picker ----------
#[derive(Deserialize)]
pub struct TimePickerParams {
    pub title: Option<String>, pub default_time: Option<String>,
    #[allow(dead_code)] pub is_24h: Option<bool>,
}
hap_fn!(hap_dialog_time_picker, TimePickerParams, |p| {
    #[cfg(target_os = "macos")]
    {
        let title = p.title.as_deref().unwrap_or("Select Time");
        let default = p.default_time.as_deref().unwrap_or("");
        let default_clause = if default.is_empty() {
            "set t to time string of (current date)".to_string()
        } else {
            format!(r#"set t to "{}""#, default)
        };
        let script = format!(
            r#"
            {default_clause}
            set theResult to (display dialog "Enter time (HH:MM):" with title "{title}" default answer t buttons {{"Cancel","OK"}} default button "OK")
            if button returned of theResult is "OK" then
                return text returned of theResult
            else
                return ""
            end if
            "#
        );
        let output = std::process::Command::new("osascript").arg("-e").arg(&script).output();
        match output {
            Ok(o) if o.status.success() => {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                Ok(json!(s))
            }
            _ => Ok(json!("")),
        }
    }
    #[cfg(not(target_os = "macos"))]
    { let _ = &p; Ok(json!("")) }
});

// ---------- datetime_picker ----------
#[derive(Deserialize)]
pub struct DatetimePickerParams {
    pub title: Option<String>, pub default_value: Option<String>,
    #[allow(dead_code)] pub min: Option<String>, #[allow(dead_code)] pub max: Option<String>,
    #[allow(dead_code)] pub is_24h: Option<bool>,
}
hap_fn!(hap_dialog_datetime_picker, DatetimePickerParams, |p| {
    #[cfg(target_os = "macos")]
    {
        let title = p.title.as_deref().unwrap_or("Select Date & Time");
        let default = p.default_value.as_deref().unwrap_or("");
        let default_val = if default.is_empty() {
            let now = chrono::Local::now();
            now.format("%Y-%m-%d %H:%M").to_string()
        } else {
            default.to_string()
        };
        let script = format!(
            r#"display dialog "Enter date and time (YYYY-MM-DD HH:MM):" with title "{title}" default answer "{default_val}" buttons {{"Cancel","OK"}} default button "OK""#
        );
        let output = std::process::Command::new("osascript").arg("-e").arg(&script).output();
        match output {
            Ok(o) if o.status.success() => {
                let s = String::from_utf8_lossy(&o.stdout);
                let value = s.split("text returned:").nth(1).unwrap_or("").trim().to_string();
                Ok(json!(value))
            }
            _ => Ok(json!("")),
        }
    }
    #[cfg(not(target_os = "macos"))]
    { let _ = &p; Ok(json!("")) }
});
