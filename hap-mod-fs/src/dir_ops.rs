use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::json;
use std::path::Path;

#[derive(Deserialize)]
pub struct MkdirParams { pub path: String, pub recursive: Option<bool> }
hap_fn!(hap_fs_mkdir, MkdirParams, |p| {
    if p.recursive.unwrap_or(true) {
        std::fs::create_dir_all(&p.path)?;
    } else {
        std::fs::create_dir(&p.path)?;
    }
    Ok(json!(true))
});

#[derive(Deserialize)]
pub struct CopyParams { pub source: String, pub dest: String, pub overwrite: Option<bool> }
hap_fn!(hap_fs_copy, CopyParams, |p| {
    if !p.overwrite.unwrap_or(false) && Path::new(&p.dest).exists() {
        return Err(HapError::invalid_param("target already exists"));
    }
    std::fs::copy(&p.source, &p.dest)?;
    Ok(json!(true))
});

#[derive(Deserialize)]
pub struct CopyDirParams { pub source: String, pub dest: String, pub overwrite: Option<bool>, #[allow(dead_code)] pub callback_id: Option<String> }
hap_fn!(hap_fs_copy_dir, CopyDirParams, |p| {
    let overwrite = p.overwrite.unwrap_or(false);
    let mut count = 0i32;
    copy_dir_recursive(Path::new(&p.source), Path::new(&p.dest), overwrite, &mut count)?;
    Ok(json!({"files_copied": count}))
});

fn copy_dir_recursive(src: &Path, dst: &Path, overwrite: bool, count: &mut i32) -> Result<(), HapError> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let dest_path = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path, overwrite, count)?;
        } else {
            if !overwrite && dest_path.exists() { continue; }
            std::fs::copy(entry.path(), &dest_path)?;
            *count += 1;
        }
    }
    Ok(())
}

#[derive(Deserialize)]
pub struct MoveParams { pub source: String, pub dest: String, pub overwrite: Option<bool> }
hap_fn!(hap_fs_move, MoveParams, |p| {
    if !p.overwrite.unwrap_or(false) && Path::new(&p.dest).exists() {
        return Err(HapError::invalid_param("target already exists"));
    }
    std::fs::rename(&p.source, &p.dest).or_else(|_| {
        std::fs::copy(&p.source, &p.dest)?;
        std::fs::remove_file(&p.source)
    })?;
    Ok(json!(true))
});

#[derive(Deserialize)]
pub struct RemoveParams { pub path: String, pub recursive: Option<bool> }
hap_fn!(hap_fs_remove, RemoveParams, |p| {
    let path = Path::new(&p.path);
    if path.is_dir() {
        if p.recursive.unwrap_or(false) { std::fs::remove_dir_all(path)?; } else { std::fs::remove_dir(path)?; }
    } else {
        std::fs::remove_file(path)?;
    }
    Ok(json!(true))
});

#[derive(Deserialize)]
pub struct ListDirParams { pub path: String, pub recursive: Option<bool>, pub include_hidden: Option<bool>, #[allow(dead_code)] pub callback_id: Option<String> }
hap_fn!(hap_fs_list_dir, ListDirParams, |p| {
    let include_hidden = p.include_hidden.unwrap_or(false);
    let recursive = p.recursive.unwrap_or(false);
    let mut items = vec![];
    list_dir_inner(Path::new(&p.path), recursive, include_hidden, &mut items)?;
    Ok(json!(items))
});

fn list_dir_inner(dir: &Path, recursive: bool, include_hidden: bool, items: &mut Vec<serde_json::Value>) -> Result<(), HapError> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if !include_hidden && name.starts_with('.') { continue; }
        let meta = entry.metadata()?;
        let modified = meta.modified().ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs().to_string()).unwrap_or_default();
        items.push(json!({
            "name": name,
            "path": entry.path().to_string_lossy(),
            "is_dir": meta.is_dir(),
            "is_file": meta.is_file(),
            "size": meta.len() as i64,
            "modified": modified,
        }));
        if recursive && meta.is_dir() {
            list_dir_inner(&entry.path(), true, include_hidden, items)?;
        }
    }
    Ok(())
}

#[derive(Deserialize)]
pub struct GlobParams { pub pattern: String, pub cwd: Option<String>, pub ignore: Option<Vec<String>> }
hap_fn!(hap_fs_glob, GlobParams, |p| {
    let cwd = p.cwd.as_deref().unwrap_or(".");
    let glob = globset::GlobBuilder::new(&p.pattern).literal_separator(false).build()
        .map_err(|e| HapError::invalid_param(format!("glob: {e}")))?
        .compile_matcher();
    let ignores: Vec<globset::GlobMatcher> = p.ignore.as_deref().unwrap_or(&[]).iter()
        .filter_map(|ig| globset::GlobBuilder::new(ig).literal_separator(false).build().ok())
        .map(|g| g.compile_matcher()).collect();
    let mut results = vec![];
    for entry in walkdir::WalkDir::new(cwd).into_iter().filter_map(|e| e.ok()) {
        let rel = entry.path().strip_prefix(cwd).unwrap_or(entry.path());
        let rel_str = rel.to_string_lossy().to_string();
        if glob.is_match(&rel_str) && !ignores.iter().any(|ig| ig.is_match(&rel_str)) {
            results.push(entry.path().to_string_lossy().to_string());
        }
    }
    Ok(json!(results))
});

#[derive(Deserialize)]
pub struct DirSizeParams { pub path: String }
hap_fn!(hap_fs_dir_size, DirSizeParams, |p| {
    let mut total = 0i64;
    let mut files = 0i32;
    let mut dirs = 0i32;
    for entry in walkdir::WalkDir::new(&p.path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            total += entry.metadata().map(|m| m.len() as i64).unwrap_or(0);
            files += 1;
        } else if entry.file_type().is_dir() && entry.path() != Path::new(&p.path) {
            dirs += 1;
        }
    }
    Ok(json!({"total_bytes": total, "file_count": files, "dir_count": dirs}))
});

#[derive(Deserialize)]
pub struct DiskUsageParams { pub path: Option<String> }
hap_fn!(hap_fs_disk_usage, DiskUsageParams, |p| {
    let target = p.path.as_deref().unwrap_or("/");
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mut best: Option<&sysinfo::Disk> = None;
    let mut best_len = 0;
    for d in disks.list() {
        let mp = d.mount_point().to_string_lossy().to_string();
        if target.starts_with(&mp) && mp.len() > best_len {
            best = Some(d);
            best_len = mp.len();
        }
    }
    match best {
        Some(d) => Ok(json!({
            "total": d.total_space() as i64,
            "free": d.available_space() as i64,
            "used": (d.total_space() - d.available_space()) as i64,
            "mount_point": d.mount_point().to_string_lossy(),
        })),
        None => Err(HapError::internal("disk not found")),
    }
});
