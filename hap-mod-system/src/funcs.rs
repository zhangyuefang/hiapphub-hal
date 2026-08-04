use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::json;

// ---------- os_info ----------
#[derive(Deserialize)]
pub struct EmptyParams {}
hap_fn!(hap_system_os_info, EmptyParams, |_p| {
    let os = if cfg!(target_os = "macos") { "macos" }
        else if cfg!(target_os = "windows") { "windows" }
        else { "linux" };
    let version = sysinfo::System::os_version().unwrap_or_default();
    let build = sysinfo::System::kernel_version().unwrap_or_default();
    let arch = std::env::consts::ARCH;
    let hostname = sysinfo::System::host_name().unwrap_or_default();
    let locale = sys_locale();
    Ok(json!({
        "os": os,
        "version": version,
        "build": build,
        "arch": arch,
        "locale": locale,
        "hostname": hostname,
    }))
});

fn sys_locale() -> String {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("defaults")
            .args(["read", "-g", "AppleLocale"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().replace('_', "-"))
            .unwrap_or_else(|| "en-US".to_string())
    }
    #[cfg(not(target_os = "macos"))]
    {
        std::env::var("LANG")
            .ok()
            .map(|l| l.split('.').next().unwrap_or("en_US").replace('_', "-"))
            .unwrap_or_else(|| "en-US".to_string())
    }
}

// ---------- cpu_info ----------
hap_fn!(hap_system_cpu_info, EmptyParams, |_p| {
    let mut sys = sysinfo::System::new();
    sys.refresh_cpu_all();
    let cpus = sys.cpus();
    let model = cpus.first().map(|c| c.brand().to_string()).unwrap_or_default();
    let vendor = cpus.first().map(|c| c.vendor_id().to_string()).unwrap_or_default();
    let freq = cpus.first().map(|c| c.frequency() as i64).unwrap_or(0);
    let cores_logical = cpus.len() as i32;
    let cores_physical = sysinfo::System::physical_core_count().unwrap_or(cores_logical as usize) as i32;
    Ok(json!({
        "model": model,
        "cores_physical": cores_physical,
        "cores_logical": cores_logical,
        "frequency_mhz": freq,
        "vendor": vendor,
        "features": serde_json::Value::Array(vec![]),
    }))
});

// ---------- memory_info ----------
hap_fn!(hap_system_memory_info, EmptyParams, |_p| {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    let total = sys.total_memory() as i64;
    let free = sys.available_memory() as i64;
    let used = total - free;
    let pct = if total > 0 { used as f64 / total as f64 * 100.0 } else { 0.0 };
    let swap_total = sys.total_swap() as i64;
    let swap_used = sys.used_swap() as i64;
    Ok(json!({
        "total_bytes": total, "free_bytes": free, "used_bytes": used,
        "usage_percent": (pct * 100.0).round() / 100.0,
        "swap_total": swap_total, "swap_used": swap_used,
    }))
});

// ---------- disk_info ----------
hap_fn!(hap_system_disk_info, EmptyParams, |_p| {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let list: Vec<_> = disks.list().iter().map(|d| {
        json!({
            "mount_point": d.mount_point().to_string_lossy(),
            "total_bytes": d.total_space() as i64,
            "free_bytes": d.available_space() as i64,
            "used_bytes": (d.total_space() - d.available_space()) as i64,
            "fs_type": String::from_utf8_lossy(d.file_system().as_encoded_bytes()),
            "is_removable": d.is_removable(),
        })
    }).collect();
    Ok(json!(list))
});

// ---------- gpu_info ----------
hap_fn!(hap_system_gpu_info, EmptyParams, |_p| {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("system_profiler")
            .args(["SPDisplaysDataType", "-json"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());
        if let Some(raw) = output {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                let mut gpus = vec![];
                if let Some(displays) = v["SPDisplaysDataType"].as_array() {
                    for d in displays {
                        gpus.push(json!({
                            "name": d["sppci_model"].as_str().unwrap_or(""),
                            "vendor": d["sppci_vendor"].as_str().unwrap_or(""),
                            "vram_mb": d.get("sppci_vram").and_then(|v| v.as_str())
                                .and_then(|s| s.split_whitespace().next().and_then(|n| n.parse::<i32>().ok()))
                                .unwrap_or(0),
                            "driver_version": "",
                        }));
                    }
                }
                return Ok(json!(gpus));
            }
        }
        Ok(json!([]))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(json!([]))
    }
});

// ---------- hostname ----------
hap_fn!(hap_system_hostname, EmptyParams, |_p| {
    Ok(json!(sysinfo::System::host_name().unwrap_or_default()))
});

// ---------- username ----------
hap_fn!(hap_system_username, EmptyParams, |_p| {
    Ok(json!(whoami::username()))
});

// ---------- home_dir ----------
hap_fn!(hap_system_home_dir, EmptyParams, |_p| {
    Ok(json!(dirs::home_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()))
});

// ---------- uptime ----------
hap_fn!(hap_system_uptime, EmptyParams, |_p| {
    Ok(json!(sysinfo::System::uptime() as i64))
});

// ---------- locale ----------
hap_fn!(hap_system_locale, EmptyParams, |_p| {
    let full = sys_locale();
    let parts: Vec<&str> = full.splitn(2, '-').collect();
    let language = parts.first().unwrap_or(&"").to_string();
    let country = parts.get(1).unwrap_or(&"").to_string();
    Ok(json!({ "language": language, "country": country, "full": full }))
});

// ---------- theme ----------
hap_fn!(hap_system_theme, EmptyParams, |_p| {
    let mode = dark_light::detect();
    Ok(json!(match mode {
        dark_light::Mode::Dark => "dark",
        _ => "light",
    }))
});

// ---------- is_elevated ----------
hap_fn!(hap_system_is_elevated, EmptyParams, |_p| {
    #[cfg(unix)]
    { Ok(json!(unsafe { libc::geteuid() } == 0)) }
    #[cfg(windows)]
    { Ok(json!(false)) }
});

// ---------- shell_version ----------
hap_fn!(hap_system_shell_version, EmptyParams, |_p| {
    let ver = hap_common::context::get_shell_version();
    Ok(json!(ver))
});

// ---------- get_proxy ----------
hap_fn!(hap_system_get_proxy, EmptyParams, |_p| {
    let http = std::env::var("http_proxy").or_else(|_| std::env::var("HTTP_PROXY")).ok();
    let https = std::env::var("https_proxy").or_else(|_| std::env::var("HTTPS_PROXY")).ok();
    let no_proxy: Option<Vec<String>> = std::env::var("no_proxy").or_else(|_| std::env::var("NO_PROXY")).ok()
        .map(|s| s.split(',').map(|p| p.trim().to_string()).collect());
    if http.is_none() && https.is_none() { return Ok(json!(null)); }
    Ok(json!({ "http": http, "https": https, "no_proxy": no_proxy }))
});

// ---------- open_settings ----------
#[derive(Deserialize)]
pub struct OpenSettingsParams { pub section: Option<String> }
hap_fn!(hap_system_open_settings, OpenSettingsParams, |p| {
    let section = p.section.as_deref().unwrap_or("");
    #[cfg(target_os = "macos")]
    {
        let pane = match section {
            "display" => "com.apple.Displays-Settings.extension",
            "network" => "com.apple.Network-Settings.extension",
            "sound" => "com.apple.Sound-Settings.extension",
            "bluetooth" => "com.apple.BluetoothSettings",
            "apps" => "com.apple.settings.Storage",
            "updates" => "com.apple.Software-Update-Settings.extension",
            _ => "",
        };
        if pane.is_empty() {
            std::process::Command::new("open").arg("-b").arg("com.apple.systempreferences").spawn()
                .map_err(|e| HapError::internal(e.to_string()))?;
        } else {
            std::process::Command::new("open").arg(format!("x-apple.systempreferences:{pane}")).spawn()
                .map_err(|e| HapError::internal(e.to_string()))?;
        }
        Ok(json!(true))
    }
    #[cfg(target_os = "windows")]
    {
        let uri = match section {
            "display" => "ms-settings:display",
            "network" => "ms-settings:network",
            "sound" => "ms-settings:sound",
            "bluetooth" => "ms-settings:bluetooth",
            "apps" => "ms-settings:appsfeatures",
            "updates" => "ms-settings:windowsupdate",
            _ => "ms-settings:",
        };
        std::process::Command::new("cmd").args(["/C", "start", uri]).spawn()
            .map_err(|e| HapError::internal(e.to_string()))?;
        Ok(json!(true))
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open").arg("gnome-control-center").spawn()
            .map_err(|e| HapError::internal(e.to_string()))?;
        Ok(json!(true))
    }
});

// ---------- app_data_dir ----------
#[derive(Deserialize)]
pub struct AppDataDirParams { pub app_id: Option<String> }
hap_fn!(hap_system_app_data_dir, AppDataDirParams, |p| {
    let app_id = p.app_id.as_deref().unwrap_or("unknown");
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let dir = home.join(".hiapphub").join("data").join("plugins").join(app_id);
    std::fs::create_dir_all(&dir)?;
    Ok(json!(dir.to_string_lossy()))
});

// ---------- total_memory_mb ----------
hap_fn!(hap_system_total_memory_mb, EmptyParams, |_p| {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    Ok(json!((sys.total_memory() / 1024 / 1024) as i32))
});

// ---------- free_memory_mb ----------
hap_fn!(hap_system_free_memory_mb, EmptyParams, |_p| {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    Ok(json!((sys.available_memory() / 1024 / 1024) as i32))
});

// ---------- machine_id ----------
hap_fn!(hap_system_machine_id, EmptyParams, |_p| {
    use sha2::Digest;
    let hostname = sysinfo::System::host_name().unwrap_or_default();
    let username = whoami::username();
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    #[cfg(target_os = "macos")]
    let hw_uuid = std::process::Command::new("ioreg")
        .args(["-rd1", "-c", "IOPlatformExpertDevice"])
        .output().ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| {
            s.lines().find(|l| l.contains("IOPlatformUUID"))
                .and_then(|l| l.split('=').next_back())
                .map(|v| v.trim().trim_matches('"').to_string())
        }).unwrap_or_default();
    #[cfg(not(target_os = "macos"))]
    let hw_uuid = String::new();
    let seed = format!("{hostname}:{username}:{os}:{arch}:{hw_uuid}");
    let hash = sha2::Sha256::digest(seed.as_bytes());
    Ok(json!(hex::encode(hash)))
});

// ---------- default_browser ----------
hap_fn!(hap_system_default_browser, EmptyParams, |_p| {
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("defaults")
            .args(["read", "com.apple.LaunchServices/com.apple.launchservices.secure", "LSHandlers"])
            .output().ok().and_then(|o| String::from_utf8(o.stdout).ok());
        if let Some(raw) = out {
            for line in raw.lines() {
                let trimmed = line.trim();
                if trimmed.contains("LSHandlerRoleAll") && trimmed.contains("http") {
                    if let Some(bundle) = raw.lines()
                        .skip_while(|l| !l.contains("LSHandlerURLScheme") || !l.contains("http"))
                        .find(|l| l.contains("LSHandlerRoleAll"))
                        .and_then(|l| l.split('=').next_back())
                    {
                        let name = bundle.trim().trim_matches('"').trim_matches(';').to_string();
                        return Ok(json!({"name": name, "path": ""}));
                    }
                }
            }
        }
        Ok(json!({"name":"Safari","path":"/Applications/Safari.app"}))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(json!(null))
    }
});

// ---------- monitors ----------
hap_fn!(hap_system_monitors, EmptyParams, |_p| {
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("system_profiler")
            .args(["SPDisplaysDataType", "-json"])
            .output().ok().and_then(|o| String::from_utf8(o.stdout).ok());
        if let Some(raw) = out {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                let mut monitors = vec![];
                if let Some(displays) = v["SPDisplaysDataType"].as_array() {
                    for (i, d) in displays.iter().enumerate() {
                        if let Some(ndrvs) = d["spdisplays_ndrvs"].as_array() {
                            for (j, m) in ndrvs.iter().enumerate() {
                                let res = m.get("_spdisplays_resolution").and_then(|v| v.as_str()).unwrap_or("");
                                let parts: Vec<&str> = res.split(|c: char| !c.is_ascii_digit()).filter(|s| !s.is_empty()).collect();
                                let w: i32 = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
                                let h: i32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                                monitors.push(json!({
                                    "id": format!("{i}-{j}"),
                                    "name": m.get("_name").and_then(|v| v.as_str()).unwrap_or(""),
                                    "width": w, "height": h,
                                    "x": 0, "y": 0,
                                    "scale_factor": 1.0,
                                    "is_primary": i == 0 && j == 0,
                                }));
                            }
                        }
                    }
                }
                return Ok(json!(monitors));
            }
        }
        Ok(json!([]))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(json!([]))
    }
});

// ---------- cpu_usage ----------
#[derive(Deserialize)]
pub struct CpuUsageParams { pub interval_ms: Option<u32> }
hap_fn!(hap_system_cpu_usage, CpuUsageParams, |p| {
    let interval = p.interval_ms.unwrap_or(1000).max(100) as u64;
    let mut sys = sysinfo::System::new();
    sys.refresh_cpu_all();
    std::thread::sleep(std::time::Duration::from_millis(interval));
    sys.refresh_cpu_all();
    let per_core: Vec<f64> = sys.cpus().iter().map(|c| (c.cpu_usage() as f64 * 100.0).round() / 100.0).collect();
    let total: f64 = if per_core.is_empty() { 0.0 } else {
        (per_core.iter().sum::<f64>() / per_core.len() as f64 * 100.0).round() / 100.0
    };
    #[cfg(unix)]
    {
        let la = sysinfo::System::load_average();
        Ok(json!({
            "total_percent": total,
            "per_core": per_core,
            "load_avg": [la.one, la.five, la.fifteen],
        }))
    }
    #[cfg(windows)]
    {
        Ok(json!({ "total_percent": total, "per_core": per_core }))
    }
});

// ---------- on_theme_change ----------
#[derive(Deserialize)]
pub struct OnThemeChangeParams { pub callback_id: String }
hap_fn!(hap_system_on_theme_change, OnThemeChangeParams, |p| {
    let watcher_id = format!("theme-{}", uuid_simple());
    let cb_id = p.callback_id.clone();
    let wid = watcher_id.clone();
    std::thread::spawn(move || {
        let mut last = format!("{:?}", dark_light::detect());
        loop {
            std::thread::sleep(std::time::Duration::from_secs(2));
            let current = format!("{:?}", dark_light::detect());
            if current != last {
                last = current.clone();
                let theme = if current.contains("Dark") { "dark" } else { "light" };
                hap_common::context::emit_callback(&cb_id, &serde_json::json!({"watcher_id": wid, "theme": theme}).to_string());
            }
        }
    });
    Ok(json!({"watcher_id": watcher_id}))
});

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    format!("{:x}", t)
}

// ---------- off_theme_change ----------
#[derive(Deserialize)]
pub struct OffThemeChangeParams { #[allow(dead_code)] pub watcher_id: String }
hap_fn!(hap_system_off_theme_change, OffThemeChangeParams, |_p| {
    Ok(json!(true))
});

// ---------- accent_color ----------
hap_fn!(hap_system_accent_color, EmptyParams, |_p| {
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("defaults")
            .args(["read", "-g", "AppleAccentColor"])
            .output().ok().and_then(|o| String::from_utf8(o.stdout).ok());
        let color = match out.as_deref().map(|s| s.trim()) {
            Some("-1") => "#8C8C8C",
            Some("0") => "#FF4040",
            Some("1") => "#F7821B",
            Some("2") => "#FFC600",
            Some("3") => "#62BA46",
            Some("4") | None => "#007AFF",
            Some("5") => "#A550A7",
            Some("6") => "#F74F9E",
            _ => "#007AFF",
        };
        Ok(json!(color))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(json!(""))
    }
});

// ---------- list_fonts ----------
hap_fn!(hap_system_list_fonts, EmptyParams, |_p| {
    #[cfg(target_os = "macos")]
    {
        let dirs = ["/System/Library/Fonts", "/Library/Fonts"];
        let home = dirs::home_dir().map(|h| h.join("Library/Fonts")).unwrap_or_default();
        let mut fonts = vec![];
        let all_dirs: Vec<&std::path::Path> = dirs.iter().map(|d| std::path::Path::new(*d))
            .chain(std::iter::once(home.as_path())).collect();
        for dir in all_dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for e in entries.flatten() {
                    let path = e.path();
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if ["ttf", "otf", "ttc", "dfont"].contains(&ext) {
                        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
                        fonts.push(json!({
                            "family": name.clone(),
                            "full_name": name,
                            "style": "Regular",
                            "path": path.to_string_lossy(),
                            "monospace": false,
                        }));
                    }
                }
            }
        }
        Ok(json!(fonts))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(json!([]))
    }
});
