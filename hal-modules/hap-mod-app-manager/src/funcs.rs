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

hap_fn!(hap_app_manager_ensure_dirs, EmptyParams, |_p| {
    let base = data_dir();
    let dirs = ["data", "data/plugins", "config", "app", "lib", "cache/downloads", "backup", "logs"];
    for d in &dirs {
        fs::create_dir_all(base.join(d))
            .map_err(|e| HapError::internal(format!("mkdir {d}: {e}")))?;
    }
    Ok(json!(base.to_string_lossy()))
});
