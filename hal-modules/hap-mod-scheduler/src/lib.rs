pub mod funcs;

use hap_common::ffi::str_to_c;
use std::ffi::c_char;

hap_common::hap_module_init!("scheduler");
hap_common::hap_free_string!();

#[no_mangle]
pub extern "C" fn hap_module_describe() -> *const c_char {
    str_to_c(include_str!("../manifest.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hap_common::ffi::free_c_string;
    use std::ffi::CStr;
    use serde_json::json;

    fn call(func: extern "C" fn(*const c_char) -> *const c_char, params: serde_json::Value) -> serde_json::Value {
        let input = str_to_c(&params.to_string());
        let result = func(input);
        unsafe { free_c_string(input as *mut _); }
        let s = unsafe { CStr::from_ptr(result) }.to_str().unwrap().to_string();
        unsafe { free_c_string(result as *mut _); }
        serde_json::from_str(&s).unwrap()
    }

    #[test]
    fn test_describe() {
        let ptr = hap_module_describe();
        let s = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        let v: serde_json::Value = serde_json::from_str(s).unwrap();
        assert_eq!(v["name"], "scheduler");
        assert!(v["functions"].as_array().unwrap().len() >= 8);
        unsafe { free_c_string(ptr as *mut _); }
    }

    #[test]
    fn test_create_cron_valid() {
        let r = call(funcs::hap_scheduler_create_cron, json!({
            "name": "test_cron", "cron_expression": "0 * * * * *", "callback_id": "cb1"
        }));
        assert!(r.get("task_id").is_some());
        assert_eq!(r["type"], "cron");
    }

    #[test]
    fn test_create_cron_invalid() {
        let r = call(funcs::hap_scheduler_create_cron, json!({
            "name": "bad", "cron_expression": "invalid", "callback_id": "cb1"
        }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_create_interval() {
        let r = call(funcs::hap_scheduler_create_interval, json!({
            "name": "test_int", "interval_ms": 1000, "callback_id": "cb2"
        }));
        assert!(r.get("task_id").is_some());
        assert_eq!(r["type"], "interval");
    }

    #[test]
    fn test_create_interval_invalid() {
        let r = call(funcs::hap_scheduler_create_interval, json!({
            "name": "bad", "interval_ms": -1, "callback_id": "cb"
        }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_create_timeout() {
        let r = call(funcs::hap_scheduler_create_timeout, json!({
            "name": "test_to", "delay_ms": 5000, "callback_id": "cb3"
        }));
        assert!(r.get("task_id").is_some());
        assert_eq!(r["type"], "timeout");
    }

    #[test]
    fn test_cancel_nonexistent() {
        let r = call(funcs::hap_scheduler_cancel, json!({ "task_id": "nonexistent" }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_pause_resume() {
        let r = call(funcs::hap_scheduler_create_interval, json!({
            "name": "pr_test", "interval_ms": 500, "callback_id": "cb4"
        }));
        let tid = r["task_id"].as_str().unwrap().to_string();

        let p = call(funcs::hap_scheduler_pause, json!({ "task_id": tid }));
        assert_eq!(p, json!(true));

        let res = call(funcs::hap_scheduler_resume, json!({ "task_id": tid }));
        assert_eq!(res, json!(true));
    }

    #[test]
    fn test_list() {
        let r = call(funcs::hap_scheduler_list, json!({}));
        assert!(r.is_array());
    }

    #[test]
    fn test_get_next_run() {
        let r = call(funcs::hap_scheduler_create_cron, json!({
            "name": "next_run_test", "cron_expression": "0 * * * * *", "callback_id": "cb5"
        }));
        let tid = r["task_id"].as_str().unwrap().to_string();

        let nr = call(funcs::hap_scheduler_get_next_run, json!({ "task_id": tid }));
        assert!(nr.get("next_run").is_some());
        assert!(nr["next_run"].is_string());
    }
}
