use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::json;
use std::path::Path;
use base64::Engine;

#[derive(Deserialize)]
pub struct ReadTextParams { pub path: String, #[allow(dead_code)] pub encoding: Option<String> }
hap_fn!(hap_fs_read_text_file, ReadTextParams, |p| {
    let content = std::fs::read_to_string(&p.path)?;
    Ok(json!(content))
});

#[derive(Deserialize)]
pub struct WriteTextParams { pub path: String, pub content: String, #[allow(dead_code)] pub encoding: Option<String>, pub create_dirs: Option<bool> }
hap_fn!(hap_fs_write_text_file, WriteTextParams, |p| {
    if p.create_dirs.unwrap_or(false) {
        if let Some(parent) = Path::new(&p.path).parent() { std::fs::create_dir_all(parent)?; }
    }
    std::fs::write(&p.path, &p.content)?;
    Ok(json!(true))
});

#[derive(Deserialize)]
pub struct AppendTextParams { pub path: String, pub content: String, #[allow(dead_code)] pub encoding: Option<String> }
hap_fn!(hap_fs_append_text_file, AppendTextParams, |p| {
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new().append(true).create(true).open(&p.path)?;
    f.write_all(p.content.as_bytes())?;
    Ok(json!(true))
});

#[derive(Deserialize)]
pub struct ReadBinaryParams { pub path: String }
hap_fn!(hap_fs_read_binary, ReadBinaryParams, |p| {
    let data = std::fs::read(&p.path)?;
    Ok(json!(base64::engine::general_purpose::STANDARD.encode(&data)))
});

#[derive(Deserialize)]
pub struct WriteBinaryParams { pub path: String, pub data: String, pub create_dirs: Option<bool> }
hap_fn!(hap_fs_write_binary, WriteBinaryParams, |p| {
    if p.create_dirs.unwrap_or(false) {
        if let Some(parent) = Path::new(&p.path).parent() { std::fs::create_dir_all(parent)?; }
    }
    let bytes = base64::engine::general_purpose::STANDARD.decode(&p.data)
        .map_err(|e| HapError::invalid_param(format!("base64: {e}")))?;
    std::fs::write(&p.path, &bytes)?;
    Ok(json!(true))
});

#[derive(Deserialize)]
pub struct PathParams { pub path: String }
hap_fn!(hap_fs_exists, PathParams, |p| { Ok(json!(Path::new(&p.path).exists())) });

hap_fn!(hap_fs_stat, PathParams, |p| {
    let meta = std::fs::symlink_metadata(&p.path)?;
    let fmt = |st: std::io::Result<std::time::SystemTime>| -> String {
        st.ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs().to_string()).unwrap_or_default()
    };
    Ok(json!({
        "size": meta.len() as i64,
        "is_dir": meta.is_dir(),
        "is_file": meta.is_file(),
        "is_symlink": meta.is_symlink(),
        "created": fmt(meta.created()),
        "modified": fmt(meta.modified()),
        "accessed": fmt(meta.accessed()),
        "readonly": meta.permissions().readonly(),
    }))
});

hap_fn!(hap_fs_is_dir, PathParams, |p| { Ok(json!(Path::new(&p.path).is_dir())) });
hap_fn!(hap_fs_is_file, PathParams, |p| { Ok(json!(Path::new(&p.path).is_file())) });
hap_fn!(hap_fs_file_size, PathParams, |p| {
    Ok(json!(std::fs::metadata(&p.path)?.len() as i64))
});
hap_fn!(hap_fs_real_path, PathParams, |p| {
    Ok(json!(std::fs::canonicalize(&p.path)?.to_string_lossy()))
});

hap_fn!(hap_fs_touch, PathParams, |p| {
    if !Path::new(&p.path).exists() {
        std::fs::write(&p.path, "")?;
    } else {
        std::fs::OpenOptions::new().write(true).open(&p.path)?;
    }
    Ok(json!(true))
});

hap_fn!(hap_fs_line_count, PathParams, |p| {
    use std::io::{BufRead, BufReader};
    let f = std::fs::File::open(&p.path)?;
    let count = BufReader::new(f).lines().count() as i32;
    Ok(json!(count))
});

hap_fn!(hap_fs_extension, PathParams, |p| {
    Ok(json!(Path::new(&p.path).extension().and_then(|e| e.to_str()).unwrap_or("")))
});

#[derive(Deserialize)]
pub struct FileNameParams { pub path: String, pub with_extension: Option<bool> }
hap_fn!(hap_fs_file_name, FileNameParams, |p| {
    let path = Path::new(&p.path);
    if p.with_extension.unwrap_or(true) {
        Ok(json!(path.file_name().and_then(|n| n.to_str()).unwrap_or("")))
    } else {
        Ok(json!(path.file_stem().and_then(|n| n.to_str()).unwrap_or("")))
    }
});

hap_fn!(hap_fs_parent_path, PathParams, |p| {
    Ok(json!(Path::new(&p.path).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()))
});

#[derive(Deserialize)]
pub struct JoinPathParams { pub parts: Vec<String> }
hap_fn!(hap_fs_join_path, JoinPathParams, |p| {
    let mut path = std::path::PathBuf::new();
    for part in &p.parts { path.push(part); }
    Ok(json!(path.to_string_lossy()))
});

hap_fn!(hap_fs_normalize_path, PathParams, |p| {
    let mut comps = vec![];
    for c in Path::new(&p.path).components() {
        match c {
            std::path::Component::ParentDir => { comps.pop(); }
            std::path::Component::CurDir => {}
            _ => comps.push(c),
        }
    }
    let result: std::path::PathBuf = comps.into_iter().collect();
    Ok(json!(result.to_string_lossy()))
});
