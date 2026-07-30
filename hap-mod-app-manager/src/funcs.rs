use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::hap_format;

fn data_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".hiapphub")
}

fn app_dir() -> PathBuf {
    data_dir().join("app")
}

fn read_manifest_from_hap(hap_path: &Path) -> Result<Value, HapError> {
    let mut reader = hap_format::HapReader::open_file(hap_path)
        .map_err(|e| HapError::internal(format!("open hap: {e}")))?;
    let data = reader.read_file("manifest.json")
        .map_err(|e| HapError::internal(format!("read manifest: {e}")))?;
    let content = String::from_utf8(data)
        .map_err(|e| HapError::internal(format!("utf8: {e}")))?;
    let mut manifest: Value = serde_json::from_str(&content)
        .map_err(|e| HapError::internal(format!("parse: {e}")))?;
    if let Value::Object(ref mut map) = manifest {
        map.insert("_hapPath".to_string(), Value::String(hap_path.to_string_lossy().to_string()));
    }
    Ok(manifest)
}

fn get_plugin_version(app_id: &str) -> Option<String> {
    let hap_path = app_dir().join(format!("{app_id}.hap"));
    if !hap_path.exists() { return None; }
    read_manifest_from_hap(&hap_path).ok()
        .and_then(|m| m["version"].as_str().map(String::from))
}

fn verify_data_integrity<R: std::io::Read + std::io::Seek>(
    reader: &mut hap_format::HapReader<R>,
) -> Result<(), HapError> {
    for entry in reader.entries.clone() {
        if entry.encrypted { continue; }
        reader.read_entry(&entry)
            .map_err(|e| HapError::internal(format!("integrity check failed for '{}': {e}", entry.path)))?;
    }
    Ok(())
}

fn is_platform_app(app_id: &str) -> bool {
    matches!(app_id, "hiapphub-shell" | "hiapphub-devtools" | "hiapphub-dev-runner")
}

fn read_versions_cache() -> Value {
    let path = data_dir().join("versions.json");
    fs::read_to_string(&path).ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| json!({}))
}

fn write_versions_cache(val: &Value) -> Result<(), HapError> {
    let path = data_dir().join("versions.json");
    let content = serde_json::to_string_pretty(val)
        .map_err(|e| HapError::internal(format!("{e}")))?;
    fs::write(&path, content).map_err(|e| HapError::internal(format!("write versions.json: {e}")))
}

fn rebuild_versions_cache() -> Result<Value, HapError> {
    let mut cache = json!({});
    let known = [
        ("hiapphub-shell", "shell"),
        ("hiapphub-devtools", "devtools"),
        ("hiapphub-dev-runner", "devRunner"),
    ];
    for (app_id, key) in &known {
        if let Some(ver) = get_plugin_version(app_id) {
            cache[key] = Value::String(ver);
        }
    }
    cache["lastCheck"] = Value::Null;
    write_versions_cache(&cache)?;
    Ok(cache)
}

#[derive(Deserialize)]
pub struct EmptyParams {}

hap_fn!(hap_app_manager_list_plugins, EmptyParams, |_p| {
    let dir = app_dir();
    if !dir.exists() {
        return Ok(json!([]));
    }
    let mut plugins = Vec::new();
    let entries = fs::read_dir(&dir)
        .map_err(|e| HapError::internal(format!("read app dir: {e}")))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("hap") {
            if let Ok(manifest) = read_manifest_from_hap(&path) {
                plugins.push(manifest);
            }
        }
    }
    Ok(json!(plugins))
});

#[derive(Deserialize)]
pub struct InstallParams {
    pub hap_path: String,
}

hap_fn!(hap_app_manager_install, InstallParams, |p| {
    let hap_file = Path::new(&p.hap_path);
    if !hap_file.exists() {
        return Err(HapError::invalid_param(format!("file not found: {}", p.hap_path)));
    }
    let is_hap = hap_format::is_hap_format(hap_file)
        .map_err(|e| HapError::internal(format!("{e}")))?;
    if !is_hap {
        return Err(HapError::invalid_param("not a valid HAP file"));
    }
    let mut reader = hap_format::HapReader::open_file(hap_file)
        .map_err(|e| HapError::internal(format!("{e}")))?;
    verify_data_integrity(&mut reader)?;
    let manifest_data = reader.read_file("manifest.json")
        .map_err(|e| HapError::internal(format!("{e}")))?;
    let manifest_str = String::from_utf8(manifest_data)
        .map_err(|e| HapError::internal(format!("{e}")))?;
    let manifest: Value = serde_json::from_str(&manifest_str)
        .map_err(|e| HapError::internal(format!("parse manifest: {e}")))?;
    let plugin_id = manifest["id"].as_str()
        .ok_or_else(|| HapError::invalid_param("manifest missing id field"))?;

    let target_dir = app_dir();
    let _ = fs::create_dir_all(&target_dir);
    let target = target_dir.join(format!("{plugin_id}.hap"));
    fs::copy(hap_file, &target)
        .map_err(|e| HapError::internal(format!("copy failed: {e}")))?;
    Ok(manifest)
});

hap_fn!(hap_app_manager_get_versions, EmptyParams, |_p| {
    let cache = read_versions_cache();
    if cache.as_object().map(|m| m.is_empty()).unwrap_or(true) {
        return rebuild_versions_cache();
    }
    Ok(cache)
});

#[derive(Deserialize)]
pub struct ReplaceParams {
    pub app_id: String,
    pub hap_path: String,
}

hap_fn!(hap_app_manager_replace, ReplaceParams, |p| {
    let new_path = Path::new(&p.hap_path);
    if !new_path.exists() {
        return Err(HapError::invalid_param(format!("hap not found: {}", p.hap_path)));
    }

    let mut reader = hap_format::HapReader::open_file(new_path)
        .map_err(|e| HapError::internal(format!("{e}")))?;
    let manifest_data = reader.read_file("manifest.json")
        .map_err(|e| HapError::internal(format!("{e}")))?;
    let manifest_str = String::from_utf8(manifest_data)
        .map_err(|e| HapError::internal(format!("{e}")))?;
    let new_manifest: Value = serde_json::from_str(&manifest_str)
        .map_err(|e| HapError::internal(format!("parse: {e}")))?;
    let manifest_id = new_manifest["id"].as_str().unwrap_or("");
    if !manifest_id.is_empty() && manifest_id != p.app_id {
        return Err(HapError::invalid_param(format!(
            "manifest id mismatch: expected {}, got {manifest_id}", p.app_id
        )));
    }

    let target = app_dir().join(format!("{}.hap", p.app_id));
    let backup = app_dir().join(format!("{}.hap.backup", p.app_id));

    let src_canonical = new_path.canonicalize().unwrap_or_else(|_| new_path.to_path_buf());
    let dst_canonical = target.canonicalize().unwrap_or_else(|_| target.clone());
    if src_canonical == dst_canonical {
        return Err(HapError::invalid_param("source and target are the same file"));
    }

    if target.exists() {
        fs::copy(&target, &backup).map_err(|e| HapError::internal(format!("backup: {e}")))?;
    }
    fs::copy(new_path, &target).map_err(|e| HapError::internal(format!("replace: {e}")))?;

    let new_version = new_manifest["version"].as_str().unwrap_or("unknown");
    let mut versions = read_versions_cache();
    let key = match p.app_id.as_str() {
        "hiapphub-shell" => "shell",
        "hiapphub-devtools" => "devtools",
        "hiapphub-dev-runner" => "devRunner",
        other => other,
    };
    versions[key] = Value::String(new_version.to_string());
    let _ = write_versions_cache(&versions);

    Ok(json!({
        "appId": p.app_id,
        "version": new_version,
        "backedUp": backup.exists(),
    }))
});

#[derive(Deserialize)]
pub struct AppIdParams {
    pub app_id: String,
}

hap_fn!(hap_app_manager_rollback, AppIdParams, |p| {
    let target = app_dir().join(format!("{}.hap", p.app_id));
    let backup = app_dir().join(format!("{}.hap.backup", p.app_id));
    if !backup.exists() {
        return Err(HapError::internal(format!("no backup found for {}", p.app_id)));
    }
    fs::copy(&backup, &target).map_err(|e| HapError::internal(format!("rollback: {e}")))?;

    let ver = get_plugin_version(&p.app_id);
    if let Some(ref v) = ver {
        let mut versions = read_versions_cache();
        let key = match p.app_id.as_str() {
            "hiapphub-shell" => "shell",
            "hiapphub-devtools" => "devtools",
            "hiapphub-dev-runner" => "devRunner",
            other => other,
        };
        versions[key] = Value::String(v.clone());
        let _ = write_versions_cache(&versions);
    }
    Ok(json!({
        "appId": p.app_id,
        "version": ver.unwrap_or_default(),
        "rolledBack": true,
    }))
});

hap_fn!(hap_app_manager_uninstall, AppIdParams, |p| {
    if is_platform_app(&p.app_id) {
        return Err(HapError::invalid_param("cannot uninstall platform apps"));
    }
    let hap = app_dir().join(format!("{}.hap", p.app_id));
    let backup = app_dir().join(format!("{}.hap.backup", p.app_id));
    if hap.exists() {
        fs::remove_file(&hap).map_err(|e| HapError::internal(format!("{e}")))?;
    }
    if backup.exists() {
        let _ = fs::remove_file(&backup);
    }
    let mut versions = read_versions_cache();
    if let Some(obj) = versions.as_object_mut() {
        obj.remove(&p.app_id);
    }
    let _ = write_versions_cache(&versions);
    Ok(json!(true))
});

#[derive(Deserialize)]
pub struct ManifestParams {
    pub app_id: String,
}

hap_fn!(hap_app_manager_get_manifest, ManifestParams, |p| {
    let hap_path = app_dir().join(format!("{}.hap", p.app_id));
    if !hap_path.exists() {
        return Err(HapError::internal(format!("app '{}' not installed", p.app_id)));
    }
    read_manifest_from_hap(&hap_path)
});

static UPDATE_ENDPOINT: &str = "https://api.hiapphub.com/v1/updates/check";

hap_fn!(hap_app_manager_check_updates, EmptyParams, |_p| {
    let versions = read_versions_cache();
    let body = json!({
        "bootstrapVersion": versions["bootstrap"].as_str().unwrap_or("0.0.0"),
        "shellVersion": versions["shell"].as_str().unwrap_or("0.0.0"),
        "devtoolsVersion": versions["devtools"].as_str().unwrap_or("0.0.0"),
        "devRunnerVersion": versions["devRunner"].as_str().unwrap_or("0.0.0"),
        "platform": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
    });

    let result = match ureq_post_json(UPDATE_ENDPOINT, &body.to_string()) {
        Ok(resp) => serde_json::from_str::<Value>(&resp)
            .unwrap_or_else(|_| json!({ "updates": [], "raw": resp })),
        Err(e) => json!({ "updates": [], "error": e, "offline": true }),
    };

    let mut v = read_versions_cache();
    v["lastCheck"] = Value::String(iso_now());
    let _ = write_versions_cache(&v);

    Ok(result)
});

fn ureq_post_json(url: &str, body: &str) -> Result<String, String> {
    let req = format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        url.split("://").nth(1).and_then(|s| s.find('/').map(|i| &s[i..])).unwrap_or("/"),
        url.split("://").nth(1).and_then(|s| s.split('/').next()).unwrap_or(""),
        body.len(),
        body,
    );
    let _ = req;
    Err("HTTP not available in HAL; use hap.hal('http', 'request', ...) from frontend".into())
}

fn iso_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let days = secs / 86400;
    let rem = secs % 86400;
    let (y, m, d) = days_to_ymd(days);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, (rem % 3600) / 60, rem % 60)
}

fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    let mut y = 1970u64;
    let mut r = days;
    loop {
        let dy = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366 } else { 365 };
        if r < dy { break; }
        r -= dy;
        y += 1;
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let dm: [u64; 12] = if leap { [31,29,31,30,31,30,31,31,30,31,30,31] } else { [31,28,31,30,31,30,31,31,30,31,30,31] };
    let mut m = 0;
    for &md in &dm { if r < md { break; } r -= md; m += 1; }
    (y, m + 1, r + 1)
}

#[derive(Deserialize)]
pub struct DownloadUpdateParams {
    pub url: String,
    pub app_id: String,
}

hap_fn!(hap_app_manager_download_update, DownloadUpdateParams, |_p| {
    Err(HapError::internal(
        "download_update requires HTTP; call from frontend: hap.hal('http', 'request', {url, output_file}) then hap.system.replaceHap(appId, path)"
    ))
});

hap_fn!(hap_app_manager_ensure_dirs, EmptyParams, |_p| {
    let base = data_dir();
    let dirs = ["data", "data/plugins", "config", "app", "lib", "cache/downloads", "backup", "logs"];
    for d in &dirs {
        fs::create_dir_all(base.join(d))
            .map_err(|e| HapError::internal(format!("mkdir {d}: {e}")))?;
    }
    Ok(json!(base.to_string_lossy()))
});
