use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::{json, Value};

fn with_clipboard<F, R>(f: F) -> Result<R, HapError>
where F: FnOnce(&mut arboard::Clipboard) -> Result<R, HapError> {
    let mut cb = arboard::Clipboard::new().map_err(|e| HapError::internal(e.to_string()))?;
    f(&mut cb)
}

// ---------- read_text ----------
hap_fn!(hap_clipboard_read_text, Value, |_p| {
    with_clipboard(|cb| {
        Ok(json!(cb.get_text().unwrap_or_default()))
    })
});

// ---------- write_text ----------
#[derive(Deserialize)]
pub struct WriteTextParams { pub text: String }
hap_fn!(hap_clipboard_write_text, WriteTextParams, |p| {
    with_clipboard(|cb| {
        cb.set_text(&p.text).map_err(|e| HapError::internal(e.to_string()))?;
        Ok(json!(true))
    })
});

// ---------- has_text ----------
hap_fn!(hap_clipboard_has_text, Value, |_p| {
    with_clipboard(|cb| Ok(json!(cb.get_text().is_ok())))
});

// ---------- read_image ----------
#[derive(Deserialize)]
pub struct ReadImageParams { pub format: Option<String> }
hap_fn!(hap_clipboard_read_image, ReadImageParams, |_p| {
    with_clipboard(|cb| {
        match cb.get_image() {
            Ok(img) => {
                let width = img.width as u32;
                let height = img.height as u32;
                let rgba = img.bytes.to_vec();
                let img_buf = image::RgbaImage::from_raw(width, height, rgba);
                if let Some(buf) = img_buf {
                    let mut cursor = std::io::Cursor::new(Vec::new());
                    buf.write_to(&mut cursor, image::ImageFormat::Png)
                        .map_err(|e| HapError::internal(e.to_string()))?;
                    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, cursor.into_inner());
                    Ok(json!({"data": b64, "width": width, "height": height}))
                } else {
                    Ok(json!(null))
                }
            }
            Err(_) => Ok(json!(null)),
        }
    })
});

// ---------- write_image ----------
#[derive(Deserialize)]
pub struct WriteImageParams { pub data: String, pub format: Option<String> }
hap_fn!(hap_clipboard_write_image, WriteImageParams, |p| {
    with_clipboard(|cb| {
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &p.data)
            .map_err(|e| HapError::invalid_param(e.to_string()))?;
        let img = image::load_from_memory(&bytes).map_err(|e| HapError::internal(e.to_string()))?;
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        let img_data = arboard::ImageData {
            width: w as usize,
            height: h as usize,
            bytes: std::borrow::Cow::Owned(rgba.into_raw()),
        };
        cb.set_image(img_data).map_err(|e| HapError::internal(e.to_string()))?;
        Ok(json!(true))
    })
});

// ---------- has_image ----------
hap_fn!(hap_clipboard_has_image, Value, |_p| {
    with_clipboard(|cb| Ok(json!(cb.get_image().is_ok())))
});

// ---------- read_html ----------
hap_fn!(hap_clipboard_read_html, Value, |_p| {
    with_clipboard(|cb| {
        Ok(json!(cb.get().html().unwrap_or_default()))
    })
});

// ---------- write_html ----------
#[derive(Deserialize)]
pub struct WriteHtmlParams { pub html: String, pub plain_text: Option<String> }
hap_fn!(hap_clipboard_write_html, WriteHtmlParams, |p| {
    with_clipboard(|cb| {
        let alt = p.plain_text.clone().unwrap_or_default();
        cb.set_html(&p.html, Some(&alt)).map_err(|e| HapError::internal(e.to_string()))?;
        Ok(json!(true))
    })
});

// ---------- read_files ----------
hap_fn!(hap_clipboard_read_files, Value, |_p| {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("osascript")
            .args(["-e", r#"tell application "System Events" to return (the clipboard as «class furl») as text"#])
            .output();
        match output {
            Ok(o) if o.status.success() => {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if s.is_empty() { return Ok(json!([])); }
                let paths: Vec<Value> = s.split(", ").map(|p| json!(p.trim())).collect();
                Ok(json!(paths))
            }
            _ => Ok(json!([])),
        }
    }
    #[cfg(not(target_os = "macos"))]
    { Ok(json!([])) }
});

// ---------- write_files ----------
#[derive(Deserialize)]
pub struct WriteFilesParams { pub paths: Vec<String>, #[allow(dead_code)] pub cut: Option<bool> }
hap_fn!(hap_clipboard_write_files, WriteFilesParams, |p| {
    #[cfg(target_os = "macos")]
    {
        let posix_files = p.paths.iter()
            .map(|path| format!("POSIX file \"{}\"", path.replace('"', "\\\"")))
            .collect::<Vec<_>>().join(", ");
        let script = format!(r#"set the clipboard to {{{posix_files}}}"#);
        std::process::Command::new("osascript").arg("-e").arg(&script).output().ok();
        Ok(json!(true))
    }
    #[cfg(not(target_os = "macos"))]
    { let _ = &p; Ok(json!(true)) }
});

// ---------- available_formats ----------
hap_fn!(hap_clipboard_available_formats, Value, |_p| {
    with_clipboard(|cb| {
        let mut formats = vec![];
        if cb.get_text().is_ok() { formats.push("text/plain"); }
        if cb.get_image().is_ok() { formats.push("image/png"); }
        // arboard 不支持独立 get_html 检测
        Ok(json!(formats))
    })
});

// ---------- clear ----------
hap_fn!(hap_clipboard_clear, Value, |_p| {
    with_clipboard(|cb| {
        cb.clear().map_err(|e| HapError::internal(e.to_string()))?;
        Ok(json!(true))
    })
});

// ---------- on_change ----------
use std::sync::{Arc, Mutex, LazyLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashMap;

struct CbWatcher {
    stop_flag: Arc<AtomicBool>,
    _handle: std::thread::JoinHandle<()>,
}

static CB_WATCHERS: LazyLock<Mutex<HashMap<String, CbWatcher>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static CB_WATCHER_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

#[derive(Deserialize)]
pub struct OnChangeParams { pub callback_id: String }
hap_fn!(hap_clipboard_on_change, OnChangeParams, |_p| {
    let wid = format!("cbw_{}", CB_WATCHER_COUNTER.fetch_add(1, Ordering::Relaxed));
    let stop = Arc::new(AtomicBool::new(false));
    let stop_ref = stop.clone();
    let handle = std::thread::spawn(move || {
        #[cfg(target_os = "macos")]
        {
            let mut prev_count: i64 = -1;
            while !stop_ref.load(Ordering::Relaxed) {
                let output = std::process::Command::new("osascript")
                    .args(["-e", r#"tell application "System Events" to return (the clipboard info)"#])
                    .output();
                let curr = match output {
                    Ok(o) => {
                        let s = String::from_utf8_lossy(&o.stdout);
                        s.len() as i64
                    }
                    _ => 0,
                };
                if prev_count >= 0 && curr != prev_count {
                    // clipboard changed
                }
                prev_count = curr;
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            while !stop_ref.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
    });
    CB_WATCHERS.lock().unwrap().insert(wid.clone(), CbWatcher { stop_flag: stop, _handle: handle });
    Ok(json!({"watcher_id": wid}))
});

// ---------- off_change ----------
#[derive(Deserialize)]
pub struct OffChangeParams { pub watcher_id: String }
hap_fn!(hap_clipboard_off_change, OffChangeParams, |p| {
    if let Some(w) = CB_WATCHERS.lock().unwrap().remove(&p.watcher_id) {
        w.stop_flag.store(true, Ordering::Relaxed);
        Ok(json!(true))
    } else {
        Ok(json!(false))
    }
});

// ---------- read_rtf ----------
hap_fn!(hap_clipboard_read_rtf, Value, |_p| {
    #[cfg(target_os = "macos")]
    {
        let script = r#"use framework "AppKit"
set pb to current application's NSPasteboard's generalPasteboard()
set rtfData to pb's dataForType:"public.rtf"
if rtfData is missing value then return ""
set rtfStr to (current application's NSString's alloc()'s initWithData:rtfData encoding:(current application's NSUTF8StringEncoding))
return rtfStr as text"#;
        let output = std::process::Command::new("osascript").arg("-e").arg(script).output();
        match output {
            Ok(o) if o.status.success() => {
                Ok(json!(String::from_utf8_lossy(&o.stdout).trim().to_string()))
            }
            _ => Ok(json!("")),
        }
    }
    #[cfg(not(target_os = "macos"))]
    { Ok(json!("")) }
});

// ---------- write_rtf ----------
#[derive(Deserialize)]
pub struct WriteRtfParams { pub rtf: String }
hap_fn!(hap_clipboard_write_rtf, WriteRtfParams, |p| {
    #[cfg(target_os = "macos")]
    {
        let tmp = std::env::temp_dir().join("hap_clipboard_rtf.tmp");
        std::fs::write(&tmp, &p.rtf)?;
        let tmp_path = tmp.to_string_lossy();
        let script = format!(r#"use framework "AppKit"
set rtfData to (current application's NSData's dataWithContentsOfFile:"{tmp_path}")
set pb to current application's NSPasteboard's generalPasteboard()
pb's clearContents()
pb's setData:rtfData forType:"public.rtf""#);
        std::process::Command::new("osascript").arg("-e").arg(&script).output().ok();
        std::fs::remove_file(&tmp).ok();
        Ok(json!(true))
    }
    #[cfg(not(target_os = "macos"))]
    { let _ = &p; Ok(json!(true)) }
});

// ---------- read_custom ----------
#[derive(Deserialize)]
pub struct ReadCustomParams { pub format: String }
hap_fn!(hap_clipboard_read_custom, ReadCustomParams, |p| {
    #[cfg(target_os = "macos")]
    {
        let script = format!(r#"use framework "AppKit"
set pb to current application's NSPasteboard's generalPasteboard()
set d to pb's dataForType:"{}"
if d is missing value then return ""
set s to (current application's NSString's alloc()'s initWithData:d encoding:(current application's NSUTF8StringEncoding))
return s as text"#, p.format.replace('"', "\\\""));
        let output = std::process::Command::new("osascript").arg("-e").arg(&script).output();
        match output {
            Ok(o) if o.status.success() => Ok(json!(String::from_utf8_lossy(&o.stdout).trim().to_string())),
            _ => Ok(json!("")),
        }
    }
    #[cfg(not(target_os = "macos"))]
    { let _ = &p; Ok(json!("")) }
});

// ---------- write_custom ----------
#[derive(Deserialize)]
pub struct WriteCustomParams { pub format: String, pub data: String }
hap_fn!(hap_clipboard_write_custom, WriteCustomParams, |p| {
    #[cfg(target_os = "macos")]
    {
        let tmp = std::env::temp_dir().join("hap_clipboard_custom.tmp");
        std::fs::write(&tmp, &p.data)?;
        let tmp_path = tmp.to_string_lossy();
        let script = format!(r#"use framework "AppKit"
set d to (current application's NSData's dataWithContentsOfFile:"{tmp_path}")
set pb to current application's NSPasteboard's generalPasteboard()
pb's clearContents()
pb's setData:d forType:"{}"
"#, p.format.replace('"', "\\\""));
        std::process::Command::new("osascript").arg("-e").arg(&script).output().ok();
        std::fs::remove_file(&tmp).ok();
        Ok(json!(true))
    }
    #[cfg(not(target_os = "macos"))]
    { let _ = &p; Ok(json!(true)) }
});

// ---------- write_multi ----------
#[derive(Deserialize)]
pub struct WriteMultiParams { pub formats: Value }
hap_fn!(hap_clipboard_write_multi, WriteMultiParams, |p| {
    if let Some(text) = p.formats.get("text").and_then(|v| v.as_str()) {
        with_clipboard(|cb| {
            cb.set_text(text).map_err(|e| HapError::internal(e.to_string()))?;
            Ok(())
        })?;
    }
    if let Some(html) = p.formats.get("html").and_then(|v| v.as_str()) {
        let plain = p.formats.get("text").and_then(|v| v.as_str()).unwrap_or("");
        with_clipboard(|cb| {
            cb.set_html(html, Some(plain)).map_err(|e| HapError::internal(e.to_string()))?;
            Ok(())
        })?;
    }
    Ok(json!(true))
});
