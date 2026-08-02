use hap_common::{hap_fn, HapError};
use serde_json::{json, Value};

hap_fn!(hap_ohos_file_pick_file, Value, |params| {
    let suffixes = params.get("suffixes").cloned().unwrap_or(json!([]));
    let max_count = params.get("max_count").and_then(|v| v.as_u64()).unwrap_or(1);
    Ok(json!({
        "action": "filePicker.DocumentSelectOptions",
        "suffixFilters": suffixes,
        "maxSelectNumber": max_count,
        "delegate": "arkts"
    }))
});

hap_fn!(hap_ohos_file_save_file, Value, |params| {
    let name = params.get("default_name").and_then(|v| v.as_str()).unwrap_or("untitled");
    let suffixes = params.get("suffixes").cloned().unwrap_or(json!([".txt"]));
    Ok(json!({
        "action": "filePicker.DocumentSaveOptions",
        "newFileNames": [name],
        "fileSuffixChoices": suffixes,
        "delegate": "arkts"
    }))
});

hap_fn!(hap_ohos_file_get_sandbox_path, Value, |params| {
    let path_type = params.get("type").and_then(|v| v.as_str())
        .ok_or_else(|| HapError::invalid_param("type required: files|cache|temp"))?;
    let base = "/data/storage/el2/base";
    let path = match path_type {
        "cache" => format!("{base}/cache"),
        "temp" => format!("{base}/temp"),
        _ => format!("{base}/files"),
    };
    Ok(json!({ "path": path }))
});

hap_fn!(hap_ohos_file_copy_to_sandbox, Value, |params| {
    let uri = params.get("uri").and_then(|v| v.as_str())
        .ok_or_else(|| HapError::invalid_param("uri required"))?;
    let dest = params.get("dest_name").and_then(|v| v.as_str())
        .ok_or_else(|| HapError::invalid_param("dest_name required"))?;
    Ok(json!({
        "action": "fs.copyFile",
        "source_uri": uri,
        "dest_name": dest,
        "delegate": "arkts"
    }))
});

hap_fn!(hap_ohos_file_share_file, Value, |params| {
    let path = params.get("path").and_then(|v| v.as_str())
        .ok_or_else(|| HapError::invalid_param("path required"))?;
    let mime = params.get("mime_type").and_then(|v| v.as_str()).unwrap_or("application/octet-stream");
    Ok(json!({
        "action": "systemShare.show",
        "path": path,
        "mime_type": mime,
        "delegate": "arkts"
    }))
});

hap_fn!(hap_ohos_file_get_file_info, Value, |params| {
    let path = params.get("path").and_then(|v| v.as_str())
        .ok_or_else(|| HapError::invalid_param("path required"))?;
    let p = std::path::Path::new(path);
    match std::fs::metadata(p) {
        Ok(meta) => Ok(json!({
            "size": meta.len(),
            "is_dir": meta.is_dir(),
            "modified": meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs())
        })),
        Err(e) => Err(HapError::io(e))
    }
});

hap_fn!(hap_ohos_file_list_dir, Value, |params| {
    let path = params.get("path").and_then(|v| v.as_str())
        .ok_or_else(|| HapError::invalid_param("path required"))?;
    let entries: Vec<Value> = std::fs::read_dir(path)
        .map_err(HapError::io)?
        .filter_map(|e| e.ok())
        .map(|e| json!({
            "name": e.file_name().to_string_lossy().to_string(),
            "is_dir": e.file_type().map(|t| t.is_dir()).unwrap_or(false)
        }))
        .collect();
    Ok(json!({ "entries": entries }))
});

hap_fn!(hap_ohos_file_read_text, Value, |params| {
    let path = params.get("path").and_then(|v| v.as_str())
        .ok_or_else(|| HapError::invalid_param("path required"))?;
    let content = std::fs::read_to_string(path).map_err(HapError::io)?;
    Ok(json!({ "content": content }))
});

hap_fn!(hap_ohos_file_write_text, Value, |params| {
    let path = params.get("path").and_then(|v| v.as_str())
        .ok_or_else(|| HapError::invalid_param("path required"))?;
    let content = params.get("content").and_then(|v| v.as_str())
        .ok_or_else(|| HapError::invalid_param("content required"))?;
    if let Some(parent) = std::path::Path::new(path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(path, content).map_err(HapError::io)?;
    Ok(json!({ "success": true }))
});

hap_fn!(hap_ohos_file_delete_file, Value, |params| {
    let path = params.get("path").and_then(|v| v.as_str())
        .ok_or_else(|| HapError::invalid_param("path required"))?;
    std::fs::remove_file(path).map_err(HapError::io)?;
    Ok(json!({ "success": true }))
});
