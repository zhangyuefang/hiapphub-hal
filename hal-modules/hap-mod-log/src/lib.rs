use hap_common::{hap_free_string, hap_module_init, hap_fn, HapError};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::Write;
use std::sync::Mutex;
use std::sync::LazyLock;

hap_module_init!("log");
hap_free_string!();

#[derive(Clone)]
struct LogConfig {
    level: Level,
    file_path: Option<String>,
    max_size_bytes: u64,
    max_files: u32,
    format: LogFormat,
}

#[derive(Clone, Copy, PartialEq, PartialOrd)]
enum Level { Trace = 0, Debug = 1, Info = 2, Warn = 3, Error = 4 }

#[derive(Clone, Copy)]
enum LogFormat { Text, Json }

impl Level {
    fn from_str(s: &str) -> Result<Self, HapError> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(Self::Trace),
            "debug" => Ok(Self::Debug),
            "info" => Ok(Self::Info),
            "warn" => Ok(Self::Warn),
            "error" => Ok(Self::Error),
            _ => Err(HapError::invalid_param(format!("unknown log level: {s}"))),
        }
    }
    fn as_str(&self) -> &'static str {
        match self {
            Self::Trace => "TRACE", Self::Debug => "DEBUG", Self::Info => "INFO",
            Self::Warn => "WARN", Self::Error => "ERROR",
        }
    }
}

static CONFIG: LazyLock<Mutex<LogConfig>> = LazyLock::new(|| Mutex::new(LogConfig {
    level: Level::Debug,
    file_path: None,
    max_size_bytes: 10 * 1024 * 1024,
    max_files: 5,
    format: LogFormat::Text,
}));

fn should_log(level: Level) -> bool {
    CONFIG.lock().map(|c| level >= c.level).unwrap_or(true)
}

fn write_log(level: Level, tag: &str, message: &str, data: Option<&Value>) {
    if !should_log(level) { return; }
    let config = CONFIG.lock().unwrap().clone();
    let now = Utc::now();
    let line = match config.format {
        LogFormat::Text => {
            if let Some(d) = data {
                format!("[{}] [{}] [{}] {} {:?}\n", now.format("%Y-%m-%d %H:%M:%S%.3f"), level.as_str(), tag, message, d)
            } else {
                format!("[{}] [{}] [{}] {}\n", now.format("%Y-%m-%d %H:%M:%S%.3f"), level.as_str(), tag, message)
            }
        }
        LogFormat::Json => {
            let mut obj = json!({
                "timestamp": now.to_rfc3339(),
                "level": level.as_str(),
                "tag": tag,
                "message": message,
            });
            if let Some(d) = data { obj["data"] = d.clone(); }
            format!("{}\n", obj)
        }
    };

    if let Some(ref path) = config.file_path {
        if let Ok(meta) = std::fs::metadata(path) {
            if meta.len() >= config.max_size_bytes {
                rotate_file(path, config.max_files);
            }
        }
        if let Some(parent) = std::path::Path::new(path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = f.write_all(line.as_bytes());
        }
    }
    eprint!("{line}");
}

fn rotate_file(path: &str, max_files: u32) {
    for i in (1..max_files).rev() {
        let from = format!("{path}.{i}");
        let to = format!("{path}.{}", i + 1);
        let _ = std::fs::rename(&from, &to);
    }
    let _ = std::fs::rename(path, format!("{path}.1"));
}

// ---------- 1-5. debug/info/warn/error/trace ----------
#[derive(Deserialize)]
struct LogParams { tag: String, message: String, data: Option<Value> }

hap_fn!(hap_log_debug, LogParams, |p| { write_log(Level::Debug, &p.tag, &p.message, p.data.as_ref()); Ok(json!(null)) });
hap_fn!(hap_log_info, LogParams, |p| { write_log(Level::Info, &p.tag, &p.message, p.data.as_ref()); Ok(json!(null)) });
hap_fn!(hap_log_warn, LogParams, |p| { write_log(Level::Warn, &p.tag, &p.message, p.data.as_ref()); Ok(json!(null)) });
hap_fn!(hap_log_error, LogParams, |p| { write_log(Level::Error, &p.tag, &p.message, p.data.as_ref()); Ok(json!(null)) });

#[derive(Deserialize)]
struct TraceParams { tag: String, message: String }
hap_fn!(hap_log_trace, TraceParams, |p| { write_log(Level::Trace, &p.tag, &p.message, None); Ok(json!(null)) });

// ---------- 6. set_level ----------
#[derive(Deserialize)]
struct SetLevelParams { level: String }
hap_fn!(hap_log_set_level, SetLevelParams, |p| {
    let level = Level::from_str(&p.level)?;
    CONFIG.lock().unwrap().level = level;
    Ok(json!(null))
});

// ---------- 7. set_file ----------
#[derive(Deserialize)]
struct SetFileParams { path: String, max_size_mb: Option<i32>, max_files: Option<i32>, format: Option<String> }
hap_fn!(hap_log_set_file, SetFileParams, |p| {
    let mut config = CONFIG.lock().unwrap();
    if let Some(parent) = std::path::Path::new(&p.path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    config.file_path = Some(p.path);
    if let Some(mb) = p.max_size_mb { config.max_size_bytes = mb as u64 * 1024 * 1024; }
    if let Some(mf) = p.max_files { config.max_files = mf as u32; }
    if let Some(ref fmt) = p.format {
        config.format = if fmt == "json" { LogFormat::Json } else { LogFormat::Text };
    }
    Ok(json!(true))
});

// ---------- 8. flush ----------
#[derive(Deserialize)]
struct EmptyParams {}
hap_fn!(hap_log_flush, EmptyParams, |_p| { Ok(json!(null)) });

// ---------- 9. read_lines ----------
#[derive(Deserialize)]
struct ReadLinesParams { path: String, tail_lines: Option<i32>, level_filter: Option<String> }
hap_fn!(hap_log_read_lines, ReadLinesParams, |p| {
    let content = std::fs::read_to_string(&p.path)?;
    let lines: Vec<&str> = content.lines().collect();
    let tail = p.tail_lines.unwrap_or(100) as usize;
    let start = if lines.len() > tail { lines.len() - tail } else { 0 };
    let mut result: Vec<&str> = lines[start..].to_vec();
    if let Some(ref filter) = p.level_filter {
        let min_level = Level::from_str(filter)?;
        result.retain(|line| {
            for level in [Level::Error, Level::Warn, Level::Info, Level::Debug, Level::Trace] {
                if line.contains(&format!("[{}]", level.as_str())) {
                    return level >= min_level;
                }
            }
            true
        });
    }
    Ok(json!(result))
});

// ---------- 10. clear ----------
#[derive(Deserialize)]
struct ClearParams { path: Option<String> }
hap_fn!(hap_log_clear, ClearParams, |p| {
    let path = p.path.or_else(|| CONFIG.lock().ok()?.file_path.clone())
        .ok_or_else(|| HapError::invalid_param("log file path not specified"))?;
    std::fs::write(&path, "")?;
    Ok(json!(true))
});

// ---------- 11. search ----------
#[derive(Deserialize)]
struct SearchParams { path: String, pattern: String, max_results: Option<i32> }
hap_fn!(hap_log_search, SearchParams, |p| {
    let re = regex::Regex::new(&p.pattern)
        .map_err(|e| HapError::invalid_param(format!("invalid regex: {e}")))?;
    let content = std::fs::read_to_string(&p.path)?;
    let max = p.max_results.unwrap_or(100) as usize;
    let mut results = Vec::new();
    for (i, line) in content.lines().enumerate() {
        if re.is_match(line) {
            let ts = line.get(1..24).unwrap_or("");
            results.push(json!({ "line_number": i + 1, "content": line, "timestamp": ts }));
            if results.len() >= max { break; }
        }
    }
    Ok(json!(results))
});

// ---------- 12. rotate ----------
#[derive(Deserialize)]
struct RotateParams { path: Option<String> }
hap_fn!(hap_log_rotate, RotateParams, |p| {
    let config = CONFIG.lock().unwrap().clone();
    let path = p.path.or(config.file_path)
        .ok_or_else(|| HapError::invalid_param("log file path not specified"))?;
    if std::path::Path::new(&path).exists() {
        rotate_file(&path, config.max_files);
    }
    Ok(json!(true))
});

#[no_mangle]
pub extern "C" fn hap_module_describe() -> *const std::os::raw::c_char {
    hap_common::ffi::str_to_c(include_str!("../manifest.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{CStr, CString};

    fn call(func: extern "C" fn(*const std::os::raw::c_char) -> *const std::os::raw::c_char, json: &str) -> Value {
        let cs = CString::new(json).unwrap();
        let result = func(cs.as_ptr());
        assert!(!result.is_null());
        let s = unsafe { CStr::from_ptr(result) }.to_str().unwrap().to_string();
        unsafe { hap_free_string(result as *mut _) };
        serde_json::from_str(&s).unwrap()
    }

    #[test]
    fn test_log_levels() {
        call(hap_log_debug, r#"{"tag":"test","message":"debug msg"}"#);
        call(hap_log_info, r#"{"tag":"test","message":"info msg"}"#);
        call(hap_log_warn, r#"{"tag":"test","message":"warn msg"}"#);
        call(hap_log_error, r#"{"tag":"test","message":"error msg"}"#);
        call(hap_log_trace, r#"{"tag":"test","message":"trace msg"}"#);
    }

    #[test]
    fn test_log_with_data() {
        call(hap_log_info, r#"{"tag":"app","message":"user login","data":{"user_id":123}}"#);
    }

    #[test]
    fn test_set_level() {
        call(hap_log_set_level, r#"{"level":"warn"}"#);
        call(hap_log_set_level, r#"{"level":"debug"}"#);
    }

    #[test]
    fn test_set_file_and_write() {
        call(hap_log_set_level, r#"{"level":"trace"}"#);
        let tmp = std::env::temp_dir().join("hap_log_test.log");
        std::fs::remove_file(&tmp).ok();
        let path = tmp.to_string_lossy().replace('\\', "\\\\");
        call(hap_log_set_file, &format!(r#"{{"path":"{path}","format":"text"}}"#));
        call(hap_log_info, r#"{"tag":"test","message":"file log test"}"#);
        call(hap_log_flush, r#"{}"#);
        std::thread::sleep(std::time::Duration::from_millis(500));
        let content = std::fs::read_to_string(&tmp).unwrap_or_default();
        assert!(content.contains("file log test"), "log content: {content}");
        std::fs::remove_file(&tmp).ok();
        CONFIG.lock().unwrap().file_path = None;
    }

    #[test]
    fn test_read_lines_and_search() {
        let tmp = std::env::temp_dir().join("hap_log_read_test.log");
        let path = tmp.to_string_lossy().replace('\\', "\\\\");
        std::fs::write(&tmp, "[2024-01-01 00:00:00.000] [INFO] [app] hello\n[2024-01-01 00:00:01.000] [ERROR] [app] oops\n[2024-01-01 00:00:02.000] [DEBUG] [app] detail\n").unwrap();

        let lines = call(hap_log_read_lines, &format!(r#"{{"path":"{path}","tail_lines":10}}"#));
        assert_eq!(lines.as_array().unwrap().len(), 3);

        let filtered = call(hap_log_read_lines, &format!(r#"{{"path":"{path}","level_filter":"error"}}"#));
        assert_eq!(filtered.as_array().unwrap().len(), 1);

        let search = call(hap_log_search, &format!(r#"{{"path":"{path}","pattern":"oops"}}"#));
        assert_eq!(search.as_array().unwrap().len(), 1);

        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_clear() {
        let tmp = std::env::temp_dir().join("hap_log_clear_test.log");
        std::fs::write(&tmp, "some log data\n").unwrap();
        let path = tmp.to_string_lossy().replace('\\', "\\\\");
        call(hap_log_clear, &format!(r#"{{"path":"{path}"}}"#));
        let content = std::fs::read_to_string(&tmp).unwrap();
        assert!(content.is_empty());
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_rotate() {
        let tmp = std::env::temp_dir().join("hap_log_rotate_test.log");
        std::fs::write(&tmp, "original content\n").unwrap();
        let path = tmp.to_string_lossy().replace('\\', "\\\\");
        call(hap_log_rotate, &format!(r#"{{"path":"{path}"}}"#));
        let rotated = std::env::temp_dir().join("hap_log_rotate_test.log.1");
        assert!(rotated.exists());
        let content = std::fs::read_to_string(&rotated).unwrap();
        assert!(content.contains("original content"));
        std::fs::remove_file(&rotated).ok();
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_describe() {
        let ptr = hap_module_describe();
        let s = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        let v: Value = serde_json::from_str(s).unwrap();
        assert_eq!(v["name"], "log");
        assert_eq!(v["functions"].as_array().unwrap().len(), 12);
        unsafe { hap_free_string(ptr as *mut _) };
    }
}
