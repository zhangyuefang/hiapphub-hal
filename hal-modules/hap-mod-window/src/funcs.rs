use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::{json, Value};
use std::process::Command;

#[cfg(target_os = "macos")]
fn list_windows_macos(visible_only: bool) -> Result<Value, HapError> {
    let script = if visible_only {
        r#"tell application "System Events" to get {name, unix id} of every process whose visible is true"#
    } else {
        r#"tell application "System Events" to get {name, unix id} of every process"#
    };
    let output = Command::new("osascript")
        .args(["-e", script])
        .output()
        .map_err(|e| HapError::internal(format!("osascript failed: {e}")))?;
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(json!({ "raw": text.trim(), "windows": [] }))
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ListParams {
    visible_only: Option<bool>,
}

hap_fn!(hap_window_list, ListParams, |params| {
    let visible = params.visible_only.unwrap_or(true);
    #[cfg(target_os = "macos")]
    { return list_windows_macos(visible); }
    #[cfg(not(target_os = "macos"))]
    { Ok(json!([])) }
});

#[derive(Deserialize)]
#[allow(dead_code)]
struct EmptyParams {}

hap_fn!(hap_window_get_active, EmptyParams, |_params| {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("osascript")
            .args(["-e", r#"tell application "System Events" to get name of first process whose frontmost is true"#])
            .output()
            .map_err(|e| HapError::internal(format!("{e}")))?;
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return Ok(json!({ "name": name, "window_id": name }));
    }
    #[cfg(not(target_os = "macos"))]
    { Ok(json!({})) }
});

#[derive(Deserialize)]
#[allow(dead_code)]
struct WindowIdParams {
    window_id: String,
}

hap_fn!(hap_window_focus, WindowIdParams, |params| {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            r#"tell application "{}" to activate"#,
            params.window_id.replace('"', r#"\""#)
        );
        Command::new("osascript").args(["-e", &script]).output()
            .map_err(|e| HapError::internal(format!("{e}")))?;
        return Ok(json!(true));
    }
    #[cfg(not(target_os = "macos"))]
    { Ok(json!(true)) }
});

#[derive(Deserialize)]
#[allow(dead_code)]
struct MoveToParams {
    window_id: String,
    x: i32,
    y: i32,
}

hap_fn!(hap_window_move_to, MoveToParams, |params| {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            r#"tell application "System Events" to tell process "{}" to set position of front window to {{{}, {}}}"#,
            params.window_id.replace('"', r#"\""#), params.x, params.y
        );
        Command::new("osascript").args(["-e", &script]).output()
            .map_err(|e| HapError::internal(format!("{e}")))?;
        return Ok(json!(true));
    }
    #[cfg(not(target_os = "macos"))]
    { Ok(json!(true)) }
});

#[derive(Deserialize)]
#[allow(dead_code)]
struct ResizeParams {
    window_id: String,
    width: i32,
    height: i32,
}

hap_fn!(hap_window_resize, ResizeParams, |params| {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            r#"tell application "System Events" to tell process "{}" to set size of front window to {{{}, {}}}"#,
            params.window_id.replace('"', r#"\""#), params.width, params.height
        );
        Command::new("osascript").args(["-e", &script]).output()
            .map_err(|e| HapError::internal(format!("{e}")))?;
        return Ok(json!(true));
    }
    #[cfg(not(target_os = "macos"))]
    { Ok(json!(true)) }
});

hap_fn!(hap_window_minimize, WindowIdParams, |params| {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            r#"tell application "System Events" to tell process "{}" to set miniaturized of front window to true"#,
            params.window_id.replace('"', r#"\""#)
        );
        Command::new("osascript").args(["-e", &script]).output()
            .map_err(|e| HapError::internal(format!("{e}")))?;
        return Ok(json!(true));
    }
    #[cfg(not(target_os = "macos"))]
    { Ok(json!(true)) }
});

hap_fn!(hap_window_maximize, WindowIdParams, |params| {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "tell application \"System Events\" to tell process \"{}\" to set position of front window to {{0, 0}}\ntell application \"System Events\" to tell process \"{}\" to set size of front window to {{1920, 1080}}",
            params.window_id.replace('"', "\\\""),
            params.window_id.replace('"', "\\\"")
        );
        Command::new("osascript").args(["-e", &script]).output()
            .map_err(|e| HapError::internal(format!("{e}")))?;
        return Ok(json!(true));
    }
    #[cfg(not(target_os = "macos"))]
    { Ok(json!(true)) }
});

hap_fn!(hap_window_restore, WindowIdParams, |params| {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            r#"tell application "System Events" to tell process "{}" to set miniaturized of front window to false"#,
            params.window_id.replace('"', r#"\""#)
        );
        Command::new("osascript").args(["-e", &script]).output()
            .map_err(|e| HapError::internal(format!("{e}")))?;
        return Ok(json!(true));
    }
    #[cfg(not(target_os = "macos"))]
    { Ok(json!(true)) }
});

#[derive(Deserialize)]
#[allow(dead_code)]
struct SetTopmostParams {
    window_id: String,
    topmost: bool,
}

hap_fn!(hap_window_set_topmost, SetTopmostParams, |_params| {
    Ok(json!(true))
});

hap_fn!(hap_window_close, WindowIdParams, |params| {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            r#"tell application "System Events" to tell process "{}" to click button 1 of front window"#,
            params.window_id.replace('"', r#"\""#)
        );
        Command::new("osascript").args(["-e", &script]).output()
            .map_err(|e| HapError::internal(format!("{e}")))?;
        return Ok(json!(true));
    }
    #[cfg(not(target_os = "macos"))]
    { Ok(json!(true)) }
});

#[derive(Deserialize)]
#[allow(dead_code)]
struct ScreenshotParams {
    window_id: String,
    path: Option<String>,
}

hap_fn!(hap_window_screenshot, ScreenshotParams, |params| {
    let save_path = params.path.unwrap_or_else(|| {
        std::env::temp_dir().join(format!("hap_win_ss_{}.png", std::process::id()))
            .to_string_lossy().to_string()
    });

    #[cfg(target_os = "macos")]
    {
        let script = format!(
            r#"tell application "{}" to activate
            delay 0.3"#,
            params.window_id.replace('"', r#"\""#)
        );
        Command::new("osascript").args(["-e", &script]).output().ok();
        Command::new("screencapture").args(["-w", "-x", &save_path]).output()
            .map_err(|e| HapError::internal(format!("{e}")))?;
        return Ok(json!({ "path": save_path, "success": true }));
    }
    #[cfg(not(target_os = "macos"))]
    { Ok(json!({ "path": save_path, "success": false, "error": "not supported" })) }
});

hap_fn!(hap_window_get_bounds, WindowIdParams, |params| {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            r#"tell application "System Events" to tell process "{}"
                set p to position of front window
                set s to size of front window
                return (item 1 of p) & "," & (item 2 of p) & "," & (item 1 of s) & "," & (item 2 of s)
            end tell"#,
            params.window_id.replace('"', r#"\""#)
        );
        let output = Command::new("osascript").args(["-e", &script]).output()
            .map_err(|e| HapError::internal(format!("{e}")))?;
        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let parts: Vec<i32> = text.split(',').filter_map(|s| s.trim().parse().ok()).collect();
        if parts.len() == 4 {
            return Ok(json!({ "x": parts[0], "y": parts[1], "width": parts[2], "height": parts[3] }));
        }
        return Ok(json!({ "x": 0, "y": 0, "width": 0, "height": 0 }));
    }
    #[cfg(not(target_os = "macos"))]
    { Ok(json!({ "x": 0, "y": 0, "width": 0, "height": 0 })) }
});
