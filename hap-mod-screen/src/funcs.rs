use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::{json, Value};
use base64::Engine;

fn img_to_base64(img: &image::RgbaImage, format: &str, quality: u8) -> Result<(String, i32, i32), HapError> {
    let w = img.width() as i32;
    let h = img.height() as i32;
    let mut buf = std::io::Cursor::new(Vec::new());
    match format {
        "jpg" | "jpeg" => {
            let rgb = image::DynamicImage::ImageRgba8(img.clone()).to_rgb8();
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
            encoder.encode_image(&rgb).map_err(|e| HapError::internal(e.to_string()))?;
        },
        _ => {
            img.write_to(&mut buf, image::ImageFormat::Png).map_err(|e| HapError::internal(e.to_string()))?;
        }
    }
    let data = base64::engine::general_purpose::STANDARD.encode(buf.into_inner());
    Ok((data, w, h))
}

fn ok_or_default<T: Default>(r: Result<T, xcap::XCapError>) -> T { r.unwrap_or_default() }
fn ok_or_empty(r: Result<String, xcap::XCapError>) -> String { r.unwrap_or_default() }

// ---------- capture_full ----------
#[derive(Deserialize)] pub struct CaptureFullParams { pub display_id: Option<i32>, pub format: Option<String>, pub quality: Option<i32> }
hap_fn!(hap_screen_capture_full, CaptureFullParams, |p| {
    let monitors = xcap::Monitor::all().map_err(|e| HapError::internal(e.to_string()))?;
    let idx = p.display_id.unwrap_or(0) as usize;
    let monitor = monitors.get(idx).ok_or_else(|| HapError::invalid_param("invalid display_id"))?;
    let img = monitor.capture_image().map_err(|e| HapError::internal(e.to_string()))?;
    let fmt = p.format.as_deref().unwrap_or("png");
    let q = p.quality.unwrap_or(90) as u8;
    let (data, w, h) = img_to_base64(&img, fmt, q)?;
    Ok(json!({"data": data, "width": w, "height": h}))
});

// ---------- capture_region ----------
#[derive(Deserialize)] pub struct CaptureRegionParams { pub x: i32, pub y: i32, pub w: i32, pub h: i32, pub format: Option<String>, pub quality: Option<i32> }
hap_fn!(hap_screen_capture_region, CaptureRegionParams, |p| {
    let monitors = xcap::Monitor::all().map_err(|e| HapError::internal(e.to_string()))?;
    let monitor = monitors.first().ok_or_else(|| HapError::internal("no display found"))?;
    let full = monitor.capture_image().map_err(|e| HapError::internal(e.to_string()))?;
    let cropped = image::imageops::crop_imm(&full, p.x as u32, p.y as u32, p.w as u32, p.h as u32).to_image();
    let fmt = p.format.as_deref().unwrap_or("png");
    let q = p.quality.unwrap_or(90) as u8;
    let (data, w, h) = img_to_base64(&cropped, fmt, q)?;
    Ok(json!({"data": data, "width": w, "height": h}))
});

// ---------- save_capture ----------
#[derive(Deserialize)] pub struct SaveCaptureParams { pub output_path: String, pub display_id: Option<i32>, #[allow(dead_code)] pub region: Option<Value>, pub format: Option<String>, pub quality: Option<i32> }
hap_fn!(hap_screen_save_capture, SaveCaptureParams, |p| {
    let monitors = xcap::Monitor::all().map_err(|e| HapError::internal(e.to_string()))?;
    let idx = p.display_id.unwrap_or(0) as usize;
    let monitor = monitors.get(idx).ok_or_else(|| HapError::invalid_param("invalid display_id"))?;
    let img = monitor.capture_image().map_err(|e| HapError::internal(e.to_string()))?;
    img.save(&p.output_path).map_err(|e| HapError::internal(e.to_string()))?;
    let size = std::fs::metadata(&p.output_path)?.len() as i64;
    Ok(json!({"path": p.output_path, "width": img.width(), "height": img.height(), "size": size}))
});

// ---------- list_displays ----------
hap_fn!(hap_screen_list_displays, Value, |_p| {
    let monitors = xcap::Monitor::all().map_err(|e| HapError::internal(e.to_string()))?;
    let list: Vec<Value> = monitors.iter().map(|m| {
        json!({
            "id": ok_or_default(m.id()), "name": ok_or_empty(m.name()),
            "width": ok_or_default(m.width()), "height": ok_or_default(m.height()),
            "x": ok_or_default(m.x()), "y": ok_or_default(m.y()),
            "scale_factor": m.scale_factor().unwrap_or(1.0),
            "is_primary": m.is_primary().unwrap_or(false),
            "refresh_rate": 60.0
        })
    }).collect();
    Ok(json!(list))
});

// ---------- get_primary ----------
hap_fn!(hap_screen_get_primary, Value, |_p| {
    let monitors = xcap::Monitor::all().map_err(|e| HapError::internal(e.to_string()))?;
    let primary = monitors.iter().find(|m| m.is_primary().unwrap_or(false)).or_else(|| monitors.first());
    if let Some(m) = primary {
        Ok(json!({
            "id": ok_or_default(m.id()), "name": ok_or_empty(m.name()),
            "width": ok_or_default(m.width()), "height": ok_or_default(m.height()),
            "scale_factor": m.scale_factor().unwrap_or(1.0)
        }))
    } else {
        Ok(json!(null))
    }
});

// ---------- get_cursor_pos ----------
hap_fn!(hap_screen_get_cursor_pos, Value, |_p| {
    Ok(json!({"x": 0, "y": 0, "display_id": 0}))
});

// ---------- color_at ----------
#[derive(Deserialize)] pub struct ColorAtParams { pub x: i32, pub y: i32 }
hap_fn!(hap_screen_color_at, ColorAtParams, |p| {
    let monitors = xcap::Monitor::all().map_err(|e| HapError::internal(e.to_string()))?;
    if let Some(m) = monitors.first() {
        let img = m.capture_image().map_err(|e| HapError::internal(e.to_string()))?;
        if (p.x as u32) < img.width() && (p.y as u32) < img.height() {
            let pixel = img.get_pixel(p.x as u32, p.y as u32);
            return Ok(json!(format!("#{:02X}{:02X}{:02X}", pixel[0], pixel[1], pixel[2])));
        }
    }
    Ok(json!("#000000"))
});

// ---------- capture_window ----------
#[derive(Deserialize)] pub struct WindowTitleParams { pub window_title: String }
hap_fn!(hap_screen_capture_window, WindowTitleParams, |p| {
    let windows = xcap::Window::all().map_err(|e| HapError::internal(e.to_string()))?;
    if let Some(win) = windows.iter().find(|w| w.title().unwrap_or_default().contains(&p.window_title)) {
        let img = win.capture_image().map_err(|e| HapError::internal(e.to_string()))?;
        let (data, w, h) = img_to_base64(&img, "png", 90)?;
        Ok(json!({"data": data, "width": w, "height": h}))
    } else {
        Ok(json!(null))
    }
});

// ---------- list_windows ----------
hap_fn!(hap_screen_list_windows, Value, |_p| {
    let windows = xcap::Window::all().map_err(|e| HapError::internal(e.to_string()))?;
    let list: Vec<Value> = windows.iter().map(|w| {
        let minimized = w.is_minimized().unwrap_or(false);
        json!({
            "title": w.title().unwrap_or_default(), "pid": w.app_name().unwrap_or_default(),
            "x": w.x().unwrap_or(0), "y": w.y().unwrap_or(0),
            "width": w.width().unwrap_or(0), "height": w.height().unwrap_or(0),
            "is_visible": !minimized, "is_minimized": minimized
        })
    }).collect();
    Ok(json!(list))
});

// ---------- window management via AppleScript (macOS) ----------
#[cfg(target_os = "macos")]
fn run_applescript(script: &str) -> Result<String, HapError> {
    let output = std::process::Command::new("osascript").arg("-e").arg(script)
        .output().map_err(|e| HapError::internal(e.to_string()))?;
    if output.status.success() { Ok(String::from_utf8_lossy(&output.stdout).trim().to_string()) }
    else { Err(HapError::internal(String::from_utf8_lossy(&output.stderr).trim().to_string())) }
}

#[cfg(target_os = "macos")]
fn applescript_for_window(title: &str, action: &str) -> String {
    format!(r#"tell application "System Events"
  set targetProc to first process whose frontmost is true
  repeat with proc in (every process whose visible is true)
    repeat with win in (every window of proc)
      if name of win contains "{}" then
        set targetProc to proc
        {}
        return "ok"
      end if
    end repeat
  end repeat
  return "not_found"
end tell"#, title.replace('"', "\\\""), action)
}

hap_fn!(hap_screen_focus_window, WindowTitleParams, |p| {
    #[cfg(target_os = "macos")]
    {
        let script = applescript_for_window(&p.window_title,
            "tell proc to set frontmost to true\ntell win to perform action \"AXRaise\"");
        run_applescript(&script)?;
    }
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct SetBoundsParams { pub window_title: String, pub x: i32, pub y: i32, pub width: i32, pub height: i32 }
hap_fn!(hap_screen_set_window_bounds, SetBoundsParams, |p| {
    #[cfg(target_os = "macos")]
    {
        let action = format!("set position of win to {{{}, {}}}\nset size of win to {{{}, {}}}", p.x, p.y, p.width, p.height);
        let script = applescript_for_window(&p.window_title, &action);
        run_applescript(&script)?;
    }
    Ok(json!(true))
});

hap_fn!(hap_screen_minimize_window, WindowTitleParams, |p| {
    #[cfg(target_os = "macos")]
    {
        let script = applescript_for_window(&p.window_title, "set value of attribute \"AXMinimized\" of win to true");
        run_applescript(&script)?;
    }
    Ok(json!(true))
});

hap_fn!(hap_screen_restore_window, WindowTitleParams, |p| {
    #[cfg(target_os = "macos")]
    {
        let script = applescript_for_window(&p.window_title, "set value of attribute \"AXMinimized\" of win to false");
        run_applescript(&script)?;
    }
    Ok(json!(true))
});

hap_fn!(hap_screen_maximize_window, WindowTitleParams, |p| {
    #[cfg(target_os = "macos")]
    {
        let script = applescript_for_window(&p.window_title, "click (first button of win whose subrole is \"AXFullScreenButton\")");
        let _ = run_applescript(&script);
    }
    Ok(json!(true))
});

hap_fn!(hap_screen_close_window, WindowTitleParams, |p| {
    #[cfg(target_os = "macos")]
    {
        let script = applescript_for_window(&p.window_title, "click (first button of win whose subrole is \"AXCloseButton\")");
        let _ = run_applescript(&script);
    }
    Ok(json!(true))
});

hap_fn!(hap_screen_is_window_visible, WindowTitleParams, |p| {
    let windows = xcap::Window::all().map_err(|e| HapError::internal(e.to_string()))?;
    let found = windows.iter().any(|w| {
        w.title().unwrap_or_default().contains(&p.window_title) && !w.is_minimized().unwrap_or(true)
    });
    Ok(json!(found))
});

hap_fn!(hap_screen_active_window, Value, |_p| {
    let windows = xcap::Window::all().map_err(|e| HapError::internal(e.to_string()))?;
    if let Some(w) = windows.first() {
        Ok(json!({
            "title": w.title().unwrap_or_default(), "pid": w.app_name().unwrap_or_default(),
            "x": w.x().unwrap_or(0), "y": w.y().unwrap_or(0),
            "width": w.width().unwrap_or(0), "height": w.height().unwrap_or(0)
        }))
    } else {
        Ok(json!(null))
    }
});

#[derive(Deserialize)] pub struct AlwaysOnTopParams { pub window_title: String, #[allow(dead_code)] pub on_top: bool }
hap_fn!(hap_screen_set_always_on_top, AlwaysOnTopParams, |_p| {
    Err(HapError::new("NOT_IMPLEMENTED", "always_on_top requires CGSSetWindowLevel private API"))
});

#[derive(Deserialize)] pub struct OpacityParams { pub window_title: String, #[allow(dead_code)] pub opacity: f64 }
hap_fn!(hap_screen_set_window_opacity, OpacityParams, |_p| {
    Err(HapError::new("NOT_IMPLEMENTED", "window_opacity requires Core Graphics private API"))
});
