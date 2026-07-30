use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::{Command, Stdio, Child};
use std::sync::{LazyLock, Mutex};
use std::io::Write as _;

struct ChildEntry {
    child: Child,
    #[allow(dead_code)]
    command: String,
}

static CHILDREN: LazyLock<Mutex<HashMap<u32, ChildEntry>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

// ---------- exec ----------
#[derive(Deserialize)]
pub struct ExecParams {
    pub command: String,
    pub args: Option<Vec<String>>,
    pub cwd: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub timeout_ms: Option<u32>,
    pub stdin: Option<String>,
    #[allow(dead_code)] pub encoding: Option<String>,
}
hap_fn!(hap_process_exec, ExecParams, |p| {
    let use_shell = p.args.is_none() && p.command.contains(' ');
    let mut cmd = if use_shell {
        if cfg!(windows) {
            let mut c = Command::new("cmd"); c.args(["/C", &p.command]); c
        } else {
            let mut c = Command::new("sh"); c.args(["-c", &p.command]); c
        }
    } else {
        let mut c = Command::new(&p.command);
        if let Some(ref args) = p.args { c.args(args); }
        c
    };
    if let Some(ref cwd) = p.cwd { cmd.current_dir(cwd); }
    if let Some(ref env) = p.env { for (k, v) in env { cmd.env(k, v); } }
    if p.stdin.is_some() {
        cmd.stdin(Stdio::piped());
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    if let Some(timeout) = p.timeout_ms {
        let mut child = cmd.spawn().map_err(|e| HapError::internal(e.to_string()))?;
        if let (Some(ref input), Some(ref mut stdin)) = (&p.stdin, child.stdin.take()) {
            stdin.write_all(input.as_bytes()).ok();
        }
        let start = std::time::Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let out = child.stdout.map(|mut s| { let mut b = Vec::new(); std::io::Read::read_to_end(&mut s, &mut b).ok(); b }).unwrap_or_default();
                    let err = child.stderr.map(|mut s| { let mut b = Vec::new(); std::io::Read::read_to_end(&mut s, &mut b).ok(); b }).unwrap_or_default();
                    return Ok(json!({
                        "code": status.code().unwrap_or(-1),
                        "stdout": String::from_utf8_lossy(&out),
                        "stderr": String::from_utf8_lossy(&err),
                        "timed_out": false
                    }));
                },
                Ok(None) => {
                    if start.elapsed().as_millis() > timeout as u128 {
                        child.kill().ok();
                        return Ok(json!({"code": -1, "stdout": "", "stderr": "timeout", "timed_out": true}));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                },
                Err(e) => return Err(HapError::internal(e.to_string())),
            }
        }
    } else {
        let output = cmd.output().map_err(|e| HapError::internal(e.to_string()))?;
        Ok(json!({
            "code": output.status.code().unwrap_or(-1),
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
            "timed_out": false
        }))
    }
});

// ---------- spawn ----------
#[derive(Deserialize)]
pub struct SpawnParams {
    pub command: String,
    pub args: Option<Vec<String>>,
    pub cwd: Option<String>,
    pub env: Option<HashMap<String, String>>,
    #[allow(dead_code)] pub encoding: Option<String>,
    #[allow(dead_code)] pub callback_id: Option<String>,
}
hap_fn!(hap_process_spawn, SpawnParams, |p| {
    let mut cmd = Command::new(&p.command);
    if let Some(ref args) = p.args { cmd.args(args); }
    if let Some(ref cwd) = p.cwd { cmd.current_dir(cwd); }
    if let Some(ref env) = p.env { for (k, v) in env { cmd.env(k, v); } }
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let child = cmd.spawn().map_err(|e| HapError::internal(e.to_string()))?;
    let pid = child.id();
    CHILDREN.lock().unwrap().insert(pid, ChildEntry { child, command: p.command.clone() });
    Ok(json!({"pid": pid}))
});

// ---------- kill ----------
#[derive(Deserialize)] pub struct KillParams { pub pid: i32, pub signal: Option<String> }
hap_fn!(hap_process_kill, KillParams, |p| {
    let uid = p.pid as u32;
    let mut map = CHILDREN.lock().unwrap();
    if let Some(entry) = map.get_mut(&uid) {
        entry.child.kill().ok();
        map.remove(&uid);
        return Ok(json!(true));
    }
    drop(map);
    #[cfg(unix)]
    {
        let sig = match p.signal.as_deref() {
            Some("SIGKILL") => 9,
            Some("SIGINT") => 2,
            _ => 15,
        };
        let ret = unsafe { libc::kill(p.pid, sig) };
        Ok(json!(ret == 0))
    }
    #[cfg(not(unix))]
    {
        let _ = Command::new("taskkill").args(&["/PID", &p.pid.to_string(), "/F"]).output();
        Ok(json!(true))
    }
});

// ---------- is_running ----------
#[derive(Deserialize)] pub struct PidParams { pub pid: i32 }
hap_fn!(hap_process_is_running, PidParams, |p| {
    let uid = p.pid as u32;
    let mut map = CHILDREN.lock().unwrap();
    if let Some(entry) = map.get_mut(&uid) {
        match entry.child.try_wait() {
            Ok(None) => return Ok(json!(true)),
            _ => {
                map.remove(&uid);
                return Ok(json!(false));
            }
        }
    }
    drop(map);
    #[cfg(unix)]
    {
        let ret = unsafe { libc::kill(p.pid, 0) };
        Ok(json!(ret == 0))
    }
    #[cfg(not(unix))]
    {
        Ok(json!(false))
    }
});

// ---------- wait ----------
#[derive(Deserialize)] pub struct WaitParams { pub pid: i32, pub timeout_ms: Option<u32> }
hap_fn!(hap_process_wait, WaitParams, |p| {
    let uid = p.pid as u32;
    let mut map = CHILDREN.lock().unwrap();
    if let Some(mut entry) = map.remove(&uid) {
        drop(map);
        if let Some(timeout) = p.timeout_ms {
            let start = std::time::Instant::now();
            loop {
                match entry.child.try_wait() {
                    Ok(Some(status)) => return Ok(json!({"code": status.code().unwrap_or(-1), "timed_out": false})),
                    Ok(None) => {
                        if start.elapsed().as_millis() > timeout as u128 {
                            return Ok(json!({"code": -1, "timed_out": true}));
                        }
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    },
                    Err(e) => return Err(HapError::internal(e.to_string())),
                }
            }
        } else {
            let status = entry.child.wait().map_err(|e| HapError::internal(e.to_string()))?;
            Ok(json!({"code": status.code().unwrap_or(-1), "timed_out": false}))
        }
    } else {
        Ok(json!({"code": -1, "timed_out": false}))
    }
});

// ---------- write_stdin ----------
#[derive(Deserialize)] pub struct WriteStdinParams { pub pid: i32, pub data: String }
hap_fn!(hap_process_write_stdin, WriteStdinParams, |p| {
    let uid = p.pid as u32;
    let mut map = CHILDREN.lock().unwrap();
    let entry = map.get_mut(&uid).ok_or_else(|| HapError::invalid_param("invalid pid or not a spawn process"))?;
    if let Some(ref mut stdin) = entry.child.stdin {
        stdin.write_all(p.data.as_bytes()).map_err(|e| HapError::internal(e.to_string()))?;
        stdin.flush().map_err(|e| HapError::internal(e.to_string()))?;
        Ok(json!(true))
    } else {
        Err(HapError::internal("stdin not available or already closed"))
    }
});

// ---------- close_stdin ----------
#[derive(Deserialize)] pub struct CloseStdinParams { pub pid: i32 }
hap_fn!(hap_process_close_stdin, CloseStdinParams, |p| {
    let uid = p.pid as u32;
    let mut map = CHILDREN.lock().unwrap();
    let entry = map.get_mut(&uid).ok_or_else(|| HapError::invalid_param("invalid pid or not a spawn process"))?;
    entry.child.stdin.take();
    Ok(json!(true))
});

// ---------- read_output ----------
#[derive(Deserialize)] pub struct ReadOutputParams { pub pid: i32, pub max_bytes: Option<usize> }
hap_fn!(hap_process_read_output, ReadOutputParams, |p| {
    let uid = p.pid as u32;
    let max = p.max_bytes.unwrap_or(8192);
    let mut map = CHILDREN.lock().unwrap();
    let entry = map.get_mut(&uid).ok_or_else(|| HapError::invalid_param("invalid pid"))?;

    let mut stdout_data = Vec::new();
    let mut stderr_data = Vec::new();

    if let Some(ref mut out) = entry.child.stdout {
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let fd = out.as_raw_fd();
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
            unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
            let mut buf = vec![0u8; max];
            match std::io::Read::read(out, &mut buf) {
                Ok(n) if n > 0 => stdout_data.extend_from_slice(&buf[..n]),
                _ => {}
            }
            unsafe { libc::fcntl(fd, libc::F_SETFL, flags) };
        }
        #[cfg(not(unix))]
        {
            let _ = out;
        }
    }

    if let Some(ref mut err) = entry.child.stderr {
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let fd = err.as_raw_fd();
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
            unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
            let mut buf = vec![0u8; max];
            match std::io::Read::read(err, &mut buf) {
                Ok(n) if n > 0 => stderr_data.extend_from_slice(&buf[..n]),
                _ => {}
            }
            unsafe { libc::fcntl(fd, libc::F_SETFL, flags) };
        }
        #[cfg(not(unix))]
        {
            let _ = err;
        }
    }

    Ok(json!({
        "stdout": String::from_utf8_lossy(&stdout_data),
        "stderr": String::from_utf8_lossy(&stderr_data)
    }))
});

// ---------- list ----------
#[derive(Deserialize)] pub struct ListParams { #[allow(dead_code)] pub sort_by: Option<String> }
hap_fn!(hap_process_list, ListParams, |_p| {
    use sysinfo::System;
    let mut sys = System::new_all();
    sys.refresh_all();
    let procs: Vec<Value> = sys.processes().iter().map(|(pid, proc_info)| {
        json!({
            "pid": pid.as_u32(),
            "name": proc_info.name().to_string_lossy(),
            "cpu_percent": proc_info.cpu_usage(),
            "memory_bytes": proc_info.memory(),
            "status": format!("{:?}", proc_info.status()),
        })
    }).collect();
    Ok(json!(procs))
});

// ---------- find_by_name ----------
#[derive(Deserialize)] pub struct FindParams { pub name: String }
hap_fn!(hap_process_find_by_name, FindParams, |p| {
    use sysinfo::System;
    let mut sys = System::new_all();
    sys.refresh_all();
    let found: Vec<Value> = sys.processes().iter()
        .filter(|(_, proc_info)| proc_info.name().to_string_lossy().contains(&p.name))
        .map(|(pid, proc_info)| json!({"pid": pid.as_u32(), "name": proc_info.name().to_string_lossy()}))
        .collect();
    Ok(json!(found))
});

// ---------- current_pid ----------
hap_fn!(hap_process_current_pid, Value, |_p| {
    Ok(json!(std::process::id()))
});

// ---------- env_var ----------
#[derive(Deserialize)] pub struct EnvVarParams { pub name: String }
hap_fn!(hap_process_env_var, EnvVarParams, |p| {
    Ok(json!(std::env::var(&p.name).unwrap_or_default()))
});

// ---------- env_vars ----------
hap_fn!(hap_process_env_vars, Value, |_p| {
    let vars: HashMap<String, String> = std::env::vars().collect();
    Ok(json!(vars))
});

// ---------- which ----------
#[derive(Deserialize)] pub struct WhichParams { pub command: String }
hap_fn!(hap_process_which, WhichParams, |p| {
    match which::which(&p.command) {
        Ok(path) => Ok(json!(path.to_string_lossy())),
        Err(_) => Ok(json!(null)),
    }
});

// ---------- self_usage ----------
hap_fn!(hap_process_self_usage, Value, |_p| {
    use sysinfo::{System, Pid};
    let pid = Pid::from_u32(std::process::id());
    let mut sys = System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
    if let Some(p) = sys.process(pid) {
        Ok(json!({
            "cpu_percent": p.cpu_usage(),
            "memory_bytes": p.memory(),
            "threads": 0,
            "uptime_ms": p.run_time() * 1000
        }))
    } else {
        Ok(json!({"cpu_percent": 0.0, "memory_bytes": 0, "threads": 0, "uptime_ms": 0}))
    }
});

// ---------- set_priority ----------
#[derive(Deserialize)] pub struct SetPriorityParams { pub pid: i32, pub priority: String }
hap_fn!(hap_process_set_priority, SetPriorityParams, |p| {
    #[cfg(unix)]
    {
        let nice_val = match p.priority.as_str() {
            "low" => 19,
            "below_normal" => 10,
            "normal" => 0,
            "above_normal" => -5,
            "high" => -10,
            _ => return Err(HapError::invalid_param("invalid priority")),
        };
        let ret = unsafe { libc::setpriority(libc::PRIO_PROCESS, p.pid as u32, nice_val) };
        if ret != 0 { return Err(HapError::internal("setpriority failed")); }
        Ok(json!(true))
    }
    #[cfg(not(unix))]
    {
        Err(HapError::new("NOT_IMPLEMENTED", "set_priority only supported on Unix"))
    }
});

