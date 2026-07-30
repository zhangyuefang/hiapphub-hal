pub mod funcs;

use hap_common::ffi::str_to_c;
use std::ffi::c_char;

hap_common::hap_module_init!("process");
hap_common::hap_free_string!();

#[no_mangle]
pub extern "C" fn hap_module_describe() -> *const c_char {
    str_to_c(include_str!("../manifest.json"))
}

#[cfg(test)]
mod tests {
    use super::funcs::*;
    use hap_common::ffi::{str_to_c, free_c_string};
    use std::ffi::CStr;
    use serde_json::json;

    fn call(func: extern "C" fn(*const std::ffi::c_char) -> *const std::ffi::c_char, params: serde_json::Value) -> serde_json::Value {
        let input = str_to_c(&params.to_string());
        let result = func(input);
        unsafe { free_c_string(input as *mut _); }
        let s = unsafe { CStr::from_ptr(result) }.to_str().unwrap().to_string();
        unsafe { free_c_string(result as *mut _); }
        serde_json::from_str(&s).unwrap()
    }

    #[test]
    fn test_describe() {
        let ptr = super::hap_module_describe();
        let s = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        let v: serde_json::Value = serde_json::from_str(s).unwrap();
        assert_eq!(v["name"], "process");
        assert_eq!(v["functions"].as_array().unwrap().len(), 16);
        unsafe { free_c_string(ptr as *mut _); }
    }

    #[test]
    fn test_exec() {
        let r = call(hap_process_exec, json!({"command": "echo", "args": ["hello"]}));
        assert!(r["stdout"].as_str().unwrap().contains("hello"));
        assert_eq!(r["code"], 0);
    }

    #[test]
    fn test_current_pid() {
        let r = call(hap_process_current_pid, json!({}));
        assert!(r.as_u64().unwrap() > 0);
    }

    #[test]
    fn test_env_var() {
        let r = call(hap_process_env_var, json!({"name": "PATH"}));
        assert!(!r.as_str().unwrap().is_empty());
    }

    #[test]
    fn test_env_vars() {
        let r = call(hap_process_env_vars, json!({}));
        assert!(r.as_object().unwrap().len() > 0);
    }

    #[test]
    fn test_which() {
        let r = call(hap_process_which, json!({"command": "echo"}));
        assert!(r.as_str().is_some());
    }

    #[test]
    fn test_list() {
        let r = call(hap_process_list, json!({}));
        assert!(r.as_array().unwrap().len() > 0);
    }

    #[test]
    fn test_is_running() {
        let pid = std::process::id() as i32;
        let r = call(hap_process_is_running, json!({"pid": pid}));
        assert_eq!(r, true);
    }

    #[test]
    fn test_find_by_name() {
        let r = call(hap_process_find_by_name, json!({"name": "cargo"}));
        assert!(r.is_array());
    }

    #[test]
    fn test_spawn_write_stdin_wait() {
        let r = call(hap_process_spawn, json!({"command": "cat"}));
        let pid = r["pid"].as_u64().unwrap() as i32;
        assert!(pid > 0);

        let r2 = call(hap_process_write_stdin, json!({"pid": pid, "data": "hello\n"}));
        assert_eq!(r2, true);

        call(hap_process_close_stdin, json!({"pid": pid}));

        let r3 = call(hap_process_wait, json!({"pid": pid, "timeout_ms": 3000}));
        assert_eq!(r3["timed_out"], false);
    }

    #[test]
    fn test_read_output() {
        let r = call(hap_process_spawn, json!({"command": "echo", "args": ["hello_output"]}));
        let pid = r["pid"].as_u64().unwrap() as i32;
        std::thread::sleep(std::time::Duration::from_millis(200));
        let out = call(hap_process_read_output, json!({"pid": pid}));
        let stdout = out["stdout"].as_str().unwrap_or("");
        assert!(stdout.contains("hello_output"), "stdout should contain 'hello_output', got: {stdout}");
        call(hap_process_wait, json!({"pid": pid, "timeout_ms": 1000}));
    }

    #[test]
    fn test_exec_timeout() {
        let r = call(hap_process_exec, json!({"command": "sleep", "args": ["10"], "timeout_ms": 100}));
        assert_eq!(r["timed_out"], true);
    }
}
