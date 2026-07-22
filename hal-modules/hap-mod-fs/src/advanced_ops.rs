use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::json;
use std::path::Path;
use base64::Engine;

#[derive(Deserialize)]
pub struct SymlinkParams { pub target: String, pub link_path: String }
hap_fn!(hap_fs_symlink, SymlinkParams, |p| {
    #[cfg(unix)]
    std::os::unix::fs::symlink(&p.target, &p.link_path)?;
    #[cfg(windows)]
    {
        if Path::new(&p.target).is_dir() {
            std::os::windows::fs::symlink_dir(&p.target, &p.link_path)?;
        } else {
            std::os::windows::fs::symlink_file(&p.target, &p.link_path)?;
        }
    }
    Ok(json!(true))
});

#[derive(Deserialize)]
pub struct PathOnly { pub path: String }
hap_fn!(hap_fs_read_link, PathOnly, |p| {
    let target = std::fs::read_link(&p.path)?;
    Ok(json!(target.to_string_lossy()))
});

hap_fn!(hap_fs_hard_link, SymlinkParams, |p| {
    std::fs::hard_link(&p.target, &p.link_path)?;
    Ok(json!(true))
});

#[derive(Deserialize)]
pub struct SetPermParams { pub path: String, pub mode: Option<String>, pub recursive: Option<bool> }
hap_fn!(hap_fs_set_permissions, SetPermParams, |p| {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode_str = p.mode.as_deref().unwrap_or("755");
        let mode = u32::from_str_radix(mode_str, 8)
            .map_err(|_| HapError::invalid_param("invalid mode"))?;
        let perms = std::fs::Permissions::from_mode(mode);
        if p.recursive.unwrap_or(false) && Path::new(&p.path).is_dir() {
            for entry in walkdir::WalkDir::new(&p.path).into_iter().filter_map(|e| e.ok()) {
                std::fs::set_permissions(entry.path(), perms.clone())?;
            }
        } else {
            std::fs::set_permissions(&p.path, perms)?;
        }
    }
    #[cfg(windows)]
    {
        let ro = p.mode.as_deref() == Some("444") || p.mode.as_deref() == Some("r");
        let mut perms = std::fs::metadata(&p.path)?.permissions();
        perms.set_readonly(ro);
        std::fs::set_permissions(&p.path, perms)?;
    }
    Ok(json!(true))
});

#[derive(Deserialize)]
pub struct TempFileParams { pub prefix: Option<String>, pub suffix: Option<String> }
hap_fn!(hap_fs_temp_file, TempFileParams, |p| {
    let dir = std::env::temp_dir();
    let name = format!(
        "{}{}{}",
        p.prefix.as_deref().unwrap_or("hap_"),
        uuid_hex(),
        p.suffix.as_deref().unwrap_or("")
    );
    let path = dir.join(&name);
    std::fs::write(&path, "")?;
    Ok(json!(path.to_string_lossy()))
});

#[derive(Deserialize)]
pub struct TempDirParams { pub prefix: Option<String> }
hap_fn!(hap_fs_temp_dir, TempDirParams, |p| {
    let dir = std::env::temp_dir();
    let name = format!("{}{}", p.prefix.as_deref().unwrap_or("hap_"), uuid_hex());
    let path = dir.join(&name);
    std::fs::create_dir_all(&path)?;
    Ok(json!(path.to_string_lossy()))
});

fn uuid_hex() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    format!("{:x}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos())
}

#[derive(Deserialize)]
pub struct ReadBinaryRangeParams { pub path: String, pub offset: i64, pub length: i64 }
hap_fn!(hap_fs_read_binary_range, ReadBinaryRangeParams, |p| {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(&p.path)?;
    f.seek(SeekFrom::Start(p.offset as u64))?;
    let mut buf = vec![0u8; p.length as usize];
    let n = f.read(&mut buf)?;
    buf.truncate(n);
    Ok(json!(base64::engine::general_purpose::STANDARD.encode(&buf)))
});

#[derive(Deserialize)]
pub struct ChecksumParams { pub path: String, pub algorithm: Option<String> }
hap_fn!(hap_fs_checksum, ChecksumParams, |p| {
    use sha2::Digest;
    let data = std::fs::read(&p.path)?;
    let hex_str = match p.algorithm.as_deref().unwrap_or("sha256") {
        "md5" => hex::encode(md5::Md5::digest(&data)),
        _ => hex::encode(sha2::Sha256::digest(&data)),
    };
    Ok(json!(hex_str))
});

#[derive(Deserialize)]
pub struct ReadTextLinesParams { pub path: String, pub start_line: Option<i32>, pub count: i32, #[allow(dead_code)] pub encoding: Option<String> }
hap_fn!(hap_fs_read_text_lines, ReadTextLinesParams, |p| {
    use std::io::{BufRead, BufReader};
    let f = std::fs::File::open(&p.path)?;
    let reader = BufReader::new(f);
    let all_lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();
    let total = all_lines.len() as i32;
    let start = (p.start_line.unwrap_or(1).max(1) - 1) as usize;
    let end = (start + p.count as usize).min(all_lines.len());
    let lines: Vec<&String> = all_lines[start..end].iter().collect();
    Ok(json!({"lines": lines, "total_lines": total}))
});

#[derive(Deserialize)]
pub struct WriteAtomicParams { pub path: String, pub content: String, #[allow(dead_code)] pub encoding: Option<String> }
hap_fn!(hap_fs_write_atomic, WriteAtomicParams, |p| {
    let tmp = format!("{}.tmp.{}", p.path, uuid_hex());
    std::fs::write(&tmp, &p.content)?;
    std::fs::rename(&tmp, &p.path)?;
    Ok(json!(true))
});

#[derive(Deserialize)]
pub struct FileTypeParams { pub path: String }
hap_fn!(hap_fs_file_type, FileTypeParams, |p| {
    let buf = {
        use std::io::Read;
        let mut f = std::fs::File::open(&p.path)?;
        let mut buf = vec![0u8; 8192];
        let n = f.read(&mut buf)?;
        buf.truncate(n);
        buf
    };
    let kind = infer::get(&buf);
    match kind {
        Some(t) => Ok(json!({
            "mime": t.mime_type(),
            "extension": t.extension(),
            "description": t.mime_type(),
        })),
        None => Ok(json!({"mime":"application/octet-stream","extension":"","description":"unknown"})),
    }
});

#[derive(Deserialize)]
pub struct SearchContentParams {
    pub dir: String, pub pattern: String, pub glob: Option<String>,
    pub recursive: Option<bool>, pub max_results: Option<i32>,
    pub case_sensitive: Option<bool>, #[allow(dead_code)] pub callback_id: Option<String>,
}
hap_fn!(hap_fs_search_content, SearchContentParams, |p| {
    let case = p.case_sensitive.unwrap_or(false);
    let re = if case {
        regex::Regex::new(&p.pattern)
    } else {
        regex::RegexBuilder::new(&p.pattern).case_insensitive(true).build()
    }.map_err(|e| HapError::invalid_param(format!("regex: {e}")))?;
    let max = p.max_results.unwrap_or(1000) as usize;
    let recursive = p.recursive.unwrap_or(true);
    let file_glob = p.glob.as_deref().unwrap_or("*");
    let matcher = globset::GlobBuilder::new(file_glob).literal_separator(false).build()
        .map_err(|e| HapError::invalid_param(format!("glob: {e}")))?.compile_matcher();
    let mut results = vec![];
    let walker = if recursive {
        walkdir::WalkDir::new(&p.dir)
    } else {
        walkdir::WalkDir::new(&p.dir).max_depth(1)
    };
    'outer: for entry in walker.into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() { continue; }
        let fname = entry.file_name().to_string_lossy();
        if !matcher.is_match(fname.as_ref()) { continue; }
        if let Ok(content) = std::fs::read_to_string(entry.path()) {
            for (i, line) in content.lines().enumerate() {
                if let Some(m) = re.find(line) {
                    results.push(json!({
                        "path": entry.path().to_string_lossy(),
                        "line_number": i + 1,
                        "line_content": line,
                        "match_start": m.start(),
                        "match_end": m.end(),
                    }));
                    if results.len() >= max { break 'outer; }
                }
            }
        }
    }
    Ok(json!(results))
});

#[derive(Deserialize)]
pub struct TruncateParams { pub path: String, pub size: i64 }
hap_fn!(hap_fs_truncate, TruncateParams, |p| {
    let f = std::fs::OpenOptions::new().write(true).open(&p.path)?;
    f.set_len(p.size as u64)?;
    Ok(json!(true))
});

#[derive(Deserialize)]
pub struct CompareParams { pub path_a: String, pub path_b: String, pub method: Option<String> }
hap_fn!(hap_fs_compare, CompareParams, |p| {
    let meta_a = std::fs::metadata(&p.path_a)?;
    let meta_b = std::fs::metadata(&p.path_b)?;
    let sa = meta_a.len() as i64;
    let sb = meta_b.len() as i64;
    if sa != sb { return Ok(json!({"identical": false, "size_a": sa, "size_b": sb})); }
    let identical = match p.method.as_deref().unwrap_or("byte") {
        "hash" => {
            use sha2::Digest;
            let ha = hex::encode(sha2::Sha256::digest(&std::fs::read(&p.path_a)?));
            let hb = hex::encode(sha2::Sha256::digest(&std::fs::read(&p.path_b)?));
            ha == hb
        }
        _ => std::fs::read(&p.path_a)? == std::fs::read(&p.path_b)?,
    };
    Ok(json!({"identical": identical, "size_a": sa, "size_b": sb}))
});

// ---------- Watch / Unwatch / ListWatchers ----------
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use notify::{Watcher, RecursiveMode};

struct WatcherEntry {
    _watcher: notify::RecommendedWatcher,
    path: String,
    recursive: bool,
}

static WATCHERS: LazyLock<Mutex<HashMap<String, WatcherEntry>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Deserialize)]
pub struct WatchParams { pub path: String, pub recursive: Option<bool>, pub debounce_ms: Option<u32>, pub callback_id: String }
hap_fn!(hap_fs_watch, WatchParams, |p| {
    let watcher_id = format!("watch-{}", uuid_hex());
    let cb_id = p.callback_id.clone();
    let watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
        if let Ok(event) = res {
            let kind = match event.kind {
                notify::EventKind::Create(_) => "create",
                notify::EventKind::Modify(_) => "modify",
                notify::EventKind::Remove(_) => "delete",
                _ => "other",
            };
            let paths: Vec<String> = event.paths.iter().map(|p| p.to_string_lossy().to_string()).collect();
            let event_json = serde_json::json!({"kind": kind, "paths": paths});
            hap_common::context::emit_callback(&cb_id, &event_json.to_string());
        }
    }).map_err(|e| HapError::internal(e.to_string()))?;
    let mode = if p.recursive.unwrap_or(false) { RecursiveMode::Recursive } else { RecursiveMode::NonRecursive };
    let mut w = watcher;
    w.watch(std::path::Path::new(&p.path), mode).map_err(|e| HapError::internal(e.to_string()))?;
    let entry = WatcherEntry { _watcher: w, path: p.path.clone(), recursive: p.recursive.unwrap_or(false) };
    WATCHERS.lock().unwrap().insert(watcher_id.clone(), entry);
    Ok(json!({"watcher_id": watcher_id}))
});

#[derive(Deserialize)]
pub struct UnwatchParams { pub watcher_id: String }
hap_fn!(hap_fs_unwatch, UnwatchParams, |p| {
    let removed = WATCHERS.lock().unwrap().remove(&p.watcher_id).is_some();
    Ok(json!(removed))
});

#[derive(Deserialize)]
pub struct EmptyP {}
hap_fn!(hap_fs_list_watchers, EmptyP, |_p| {
    let map = WATCHERS.lock().unwrap();
    let list: Vec<serde_json::Value> = map.iter().map(|(id, entry)| {
        json!({"watcher_id": id, "path": entry.path, "recursive": entry.recursive})
    }).collect();
    Ok(json!(list))
});

// ---------- LockFile / UnlockFile / ListLocks ----------
use fs2::FileExt;

struct LockEntry {
    file: std::fs::File,
    path: String,
    exclusive: bool,
}

static LOCKS: LazyLock<Mutex<HashMap<String, LockEntry>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Deserialize)]
pub struct LockFileParams { pub path: String, pub exclusive: Option<bool>, #[allow(dead_code)] pub timeout_ms: Option<u32> }
hap_fn!(hap_fs_lock_file, LockFileParams, |p| {
    let file = std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(&p.path)?;
    let exclusive = p.exclusive.unwrap_or(true);
    if exclusive {
        file.try_lock_exclusive().map_err(|e| HapError::internal(format!("lock failed: {e}")))?;
    } else {
        file.try_lock_shared().map_err(|e| HapError::internal(format!("lock failed: {e}")))?;
    }
    let lock_id = format!("lock-{}", uuid_hex());
    LOCKS.lock().unwrap().insert(lock_id.clone(), LockEntry { file, path: p.path.clone(), exclusive });
    Ok(json!({"lock_id": lock_id}))
});

#[derive(Deserialize)]
pub struct UnlockParams { pub lock_id: String }
hap_fn!(hap_fs_unlock_file, UnlockParams, |p| {
    if let Some(entry) = LOCKS.lock().unwrap().remove(&p.lock_id) {
        entry.file.unlock().map_err(|e| HapError::internal(e.to_string()))?;
        Ok(json!(true))
    } else {
        Ok(json!(false))
    }
});

hap_fn!(hap_fs_list_locks, EmptyP, |_p| {
    let map = LOCKS.lock().unwrap();
    let list: Vec<serde_json::Value> = map.iter().map(|(id, entry)| {
        json!({"lock_id": id, "path": entry.path, "exclusive": entry.exclusive})
    }).collect();
    Ok(json!(list))
});
