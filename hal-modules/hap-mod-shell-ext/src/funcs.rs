use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::{json, Value};

// ---------- open_url ----------
#[derive(Deserialize)]
pub struct OpenUrlParams { pub url: String }
hap_fn!(hap_shell_ext_open_url, OpenUrlParams, |p| {
    open::that(&p.url).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

// ---------- open_path ----------
#[derive(Deserialize)]
pub struct OpenPathParams { pub path: String }
hap_fn!(hap_shell_ext_open_path, OpenPathParams, |p| {
    open::that(&p.path).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

// ---------- open_with ----------
#[derive(Deserialize)]
pub struct OpenWithParams { pub path: String, pub app_path: String }
hap_fn!(hap_shell_ext_open_with, OpenWithParams, |p| {
    open::with(&p.path, &p.app_path).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

// ---------- show_in_explorer ----------
hap_fn!(hap_shell_ext_show_in_explorer, OpenPathParams, |p| {
    #[cfg(target_os = "macos")]
    { std::process::Command::new("open").arg("-R").arg(&p.path).spawn().ok(); }
    #[cfg(target_os = "windows")]
    { std::process::Command::new("explorer").arg("/select,").arg(&p.path).spawn().ok(); }
    #[cfg(target_os = "linux")]
    { std::process::Command::new("xdg-open").arg(std::path::Path::new(&p.path).parent().unwrap_or(std::path::Path::new("/"))).spawn().ok(); }
    Ok(json!(true))
});

// ---------- trash ----------
#[derive(Deserialize)]
pub struct TrashParams { pub paths: Vec<String> }
hap_fn!(hap_shell_ext_trash, TrashParams, |p| {
    let mut trashed = 0;
    let mut failed = 0;
    for path in &p.paths {
        match trash::delete(path) {
            Ok(_) => trashed += 1,
            Err(_) => failed += 1,
        }
    }
    Ok(json!({"trashed": trashed, "failed": failed}))
});

// ---------- run_detached ----------
#[derive(Deserialize)]
pub struct RunDetachedParams { pub command: String, pub args: Option<Vec<String>>, pub cwd: Option<String> }
hap_fn!(hap_shell_ext_run_detached, RunDetachedParams, |p| {
    let mut cmd = std::process::Command::new(&p.command);
    if let Some(ref args) = p.args { cmd.args(args); }
    if let Some(ref cwd) = p.cwd { cmd.current_dir(cwd); }
    let child = cmd.spawn().map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!({"pid": child.id() as i32}))
});

// ---------- get_special_dir ----------
#[derive(Deserialize)]
pub struct GetSpecialDirParams { pub name: String }
hap_fn!(hap_shell_ext_get_special_dir, GetSpecialDirParams, |p| {
    let dir = match p.name.as_str() {
        "desktop" => dirs::desktop_dir(),
        "documents" => dirs::document_dir(),
        "downloads" => dirs::download_dir(),
        "pictures" => dirs::picture_dir(),
        "music" => dirs::audio_dir(),
        "videos" => dirs::video_dir(),
        "temp" => Some(std::env::temp_dir()),
        "home" => dirs::home_dir(),
        "config" => dirs::config_dir(),
        "data" => dirs::data_dir(),
        "cache" => dirs::cache_dir(),
        "app_data" => dirs::data_local_dir(),
        _ => None,
    };
    Ok(json!(dir.map(|d| d.to_string_lossy().to_string()).unwrap_or_default()))
});

// ---------- get_env ----------
#[derive(Deserialize)]
pub struct GetEnvParams { pub name: String }
hap_fn!(hap_shell_ext_get_env, GetEnvParams, |p| {
    Ok(json!(std::env::var(&p.name).unwrap_or_default()))
});

// ---------- set_env ----------
#[derive(Deserialize)]
pub struct SetEnvParams { pub name: String, pub value: String }
hap_fn!(hap_shell_ext_set_env, SetEnvParams, |p| {
    std::env::set_var(&p.name, &p.value);
    Ok(json!(true))
});

// ---------- get_file_icon (stub) ----------
#[derive(Deserialize)]
pub struct GetFileIconParams { #[allow(dead_code)] pub path: String, #[allow(dead_code)] pub size: Option<String> }
hap_fn!(hap_shell_ext_get_file_icon, GetFileIconParams, |p| {
    #[cfg(target_os = "macos")]
    {
        let size = match p.size.as_deref() {
            Some("large") => 128,
            Some("small") => 16,
            _ => 32,
        };
        let output = std::process::Command::new("osascript")
            .arg("-l").arg("JavaScript")
            .arg("-e").arg(&format!(
                r#"ObjC.import('AppKit');
                var ws = $.NSWorkspace.sharedWorkspace;
                var icon = ws.iconForFile($('{}'));
                icon.setSize($.NSMakeSize({},{}));
                var tiff = icon.TIFFRepresentation;
                var bitmap = $.NSBitmapImageRep.imageRepWithData(tiff);
                var png = bitmap.representationUsingTypeProperties($.NSBitmapImageFileTypePNG, $.NSDictionary.dictionary);
                var b64 = png.base64EncodedStringWithOptions(0);
                b64.js;"#,
                p.path.replace('\'', "\\'").replace('"', "\\\""), size, size
            ))
            .output();
        match output {
            Ok(o) if o.status.success() => {
                let b64 = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if b64.is_empty() { return Ok(json!("")); }
                Ok(json!(format!("data:image/png;base64,{b64}")))
            }
            _ => Ok(json!("")),
        }
    }
    #[cfg(not(target_os = "macos"))]
    { let _ = &p; Ok(json!("")) }
});

// ---------- get_mime_type ----------
#[derive(Deserialize)]
pub struct GetMimeTypeParams { pub path: String }
hap_fn!(hap_shell_ext_get_mime_type, GetMimeTypeParams, |p| {
    let ext = std::path::Path::new(&p.path).extension().and_then(|e| e.to_str()).unwrap_or("");
    let mime = match ext.to_lowercase().as_str() {
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "xml" => "application/xml",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "mp3" => "audio/mpeg",
        "mp4" => "video/mp4",
        "txt" => "text/plain",
        "csv" => "text/csv",
        _ => "application/octet-stream",
    };
    Ok(json!(mime))
});

// ---------- get_default_app ----------
#[derive(Deserialize)]
pub struct GetDefaultAppParams { pub extension: String }
hap_fn!(hap_shell_ext_get_default_app, GetDefaultAppParams, |p| {
    #[cfg(target_os = "macos")]
    {
        let uti_map: &[(&str, &str)] = &[
            ("html", "public.html"), ("htm", "public.html"), ("txt", "public.plain-text"),
            ("pdf", "com.adobe.pdf"), ("png", "public.png"), ("jpg", "public.jpeg"),
            ("jpeg", "public.jpeg"), ("gif", "com.compuserve.gif"), ("mp3", "public.mp3"),
            ("mp4", "public.mpeg-4"), ("zip", "com.pkware.zip-archive"), ("doc", "com.microsoft.word.doc"),
            ("xls", "com.microsoft.excel.xls"), ("ppt", "com.microsoft.powerpoint.ppt"),
            ("csv", "public.comma-separated-values-text"), ("json", "public.json"),
        ];
        let ext = p.extension.trim_start_matches('.').to_lowercase();
        let uti = uti_map.iter().find(|(e, _)| *e == ext).map(|(_, u)| *u)
            .unwrap_or_else(|| Box::leak(format!("public.{}", ext).into_boxed_str()));
        let output = std::process::Command::new("/usr/bin/python3")
            .args(["-c", &format!(
                "from LaunchServices import LSCopyDefaultApplicationURLForContentType as f; from CoreServices import UTTypeCreatePreferredIdentifierForTag, kUTTagClassFilenameExtension; print(f('{}', 0x2) or '')", uti
            )])
            .output();
        match output {
            Ok(o) => {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if s.is_empty() || s == "None" { Ok(json!(null)) }
                else { Ok(json!({"path": s})) }
            }
            Err(_) => Ok(json!(null)),
        }
    }
    #[cfg(not(target_os = "macos"))]
    { Ok(json!(null)) }
});

// ---------- create_shortcut (stub) ----------
#[derive(Deserialize)]
pub struct CreateShortcutParams {
    #[allow(dead_code)] pub target_path: String, #[allow(dead_code)] pub shortcut_path: String,
    #[allow(dead_code)] pub icon_path: Option<String>, #[allow(dead_code)] pub description: Option<String>,
    #[allow(dead_code)] pub args: Option<String>,
}
hap_fn!(hap_shell_ext_create_shortcut, CreateShortcutParams, |p| {
    #[cfg(target_os = "macos")]
    {
        if p.shortcut_path.ends_with(".app") || p.shortcut_path.ends_with(".lnk") {
            std::os::unix::fs::symlink(&p.target_path, &p.shortcut_path)?;
        } else {
            std::os::unix::fs::symlink(&p.target_path, &p.shortcut_path)?;
        }
    }
    #[cfg(target_os = "linux")]
    { std::os::unix::fs::symlink(&p.target_path, &p.shortcut_path)?; }
    #[cfg(target_os = "windows")]
    { let _ = (&p.target_path, &p.shortcut_path); return Err(HapError::new("NOT_IMPLEMENTED", "Windows shortcut needs COM API")); }
    Ok(json!(true))
});

// ---------- remove_shortcut ----------
#[derive(Deserialize)]
pub struct RemoveShortcutParams { pub shortcut_path: String }
hap_fn!(hap_shell_ext_remove_shortcut, RemoveShortcutParams, |p| {
    let _ = std::fs::remove_file(&p.shortcut_path);
    Ok(json!(true))
});

// ---------- shortcut_exists ----------
hap_fn!(hap_shell_ext_shortcut_exists, RemoveShortcutParams, |p| {
    Ok(json!(std::path::Path::new(&p.shortcut_path).exists()))
});

// ---------- is_autostart ----------
#[derive(Deserialize)]
pub struct AutostartParams { pub app_id: String, pub enabled: Option<bool>, pub args: Option<Vec<String>> }

fn autostart_plist_path(app_id: &str) -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|h| h.join("Library/LaunchAgents").join(format!("{}.plist", app_id)))
}

hap_fn!(hap_shell_ext_is_autostart, AutostartParams, |p| {
    #[cfg(target_os = "macos")]
    {
        if let Some(plist) = autostart_plist_path(&p.app_id) {
            Ok(json!(plist.exists()))
        } else {
            Ok(json!(false))
        }
    }
    #[cfg(not(target_os = "macos"))]
    { let _ = &p; Ok(json!(false)) }
});

// ---------- set_autostart ----------
hap_fn!(hap_shell_ext_set_autostart, AutostartParams, |p| {
    #[cfg(target_os = "macos")]
    {
        let plist = autostart_plist_path(&p.app_id).ok_or_else(|| HapError::internal("cannot determine home dir"))?;
        let enable = p.enabled.unwrap_or(true);
        if enable {
            let exe = std::env::current_exe().map_err(|e| HapError::internal(e.to_string()))?;
            let exe_str = exe.to_string_lossy();
            let args_xml = p.args.as_ref().map(|a| a.iter().map(|s| format!("    <string>{}</string>", s)).collect::<Vec<_>>().join("\n")).unwrap_or_default();
            let content = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{}</string>
    <key>ProgramArguments</key>
    <array>
    <string>{}</string>
{}
    </array>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>"#, p.app_id, exe_str, args_xml);
            if let Some(parent) = plist.parent() { std::fs::create_dir_all(parent)?; }
            std::fs::write(&plist, content)?;
        } else if plist.exists() {
            std::fs::remove_file(&plist)?;
        }
        Ok(json!(true))
    }
    #[cfg(not(target_os = "macos"))]
    { let _ = &p; Err(HapError::new("NOT_IMPLEMENTED", "autostart not supported on this platform")) }
});

// ---------- list_printers ----------
hap_fn!(hap_shell_ext_list_printers, Value, |_p| {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("lpstat").arg("-a").output();
        match output {
            Ok(o) => {
                let s = String::from_utf8_lossy(&o.stdout);
                let printers: Vec<Value> = s.lines().filter_map(|line| {
                    let name = line.split_whitespace().next()?;
                    Some(json!({"name": name, "status": "idle"}))
                }).collect();
                Ok(json!(printers))
            }
            Err(_) => Ok(json!([])),
        }
    }
    #[cfg(not(target_os = "macos"))]
    { Ok(json!([])) }
});

// ---------- get_default_printer ----------
hap_fn!(hap_shell_ext_get_default_printer, Value, |_p| {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("lpstat").arg("-d").output();
        match output {
            Ok(o) => {
                let s = String::from_utf8_lossy(&o.stdout);
                let name = s.split(':').nth(1).map(|s| s.trim().to_string()).unwrap_or_default();
                Ok(json!(name))
            }
            Err(_) => Ok(json!("")),
        }
    }
    #[cfg(not(target_os = "macos"))]
    { Ok(json!("")) }
});

// ---------- print_file ----------
#[derive(Deserialize)]
pub struct PrintFileParams {
    pub path: String, pub printer: Option<String>,
    pub copies: Option<i32>, pub range: Option<String>,
    pub duplex: Option<bool>, pub orientation: Option<String>,
    #[allow(dead_code)] pub paper_size: Option<String>, #[allow(dead_code)] pub silent: Option<bool>,
}
hap_fn!(hap_shell_ext_print_file, PrintFileParams, |p| {
    #[cfg(target_os = "macos")]
    {
        let mut cmd = std::process::Command::new("lp");
        if let Some(ref printer) = p.printer { cmd.arg("-d").arg(printer); }
        if let Some(copies) = p.copies { cmd.arg("-n").arg(copies.to_string()); }
        if let Some(ref range) = p.range { cmd.arg("-P").arg(range); }
        if let Some(true) = p.duplex { cmd.arg("-o").arg("sides=two-sided-long-edge"); }
        if let Some(ref orient) = p.orientation {
            if orient == "landscape" { cmd.arg("-o").arg("landscape"); }
        }
        cmd.arg(&p.path);
        let output = cmd.output().map_err(|e| HapError::internal(e.to_string()))?;
        if output.status.success() { Ok(json!(true)) }
        else { Err(HapError::internal(String::from_utf8_lossy(&output.stderr).to_string())) }
    }
    #[cfg(not(target_os = "macos"))]
    { let _ = &p; Err(HapError::new("NOT_IMPLEMENTED", "print not supported on this platform")) }
});
