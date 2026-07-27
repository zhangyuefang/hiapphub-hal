pub mod client;
pub mod types;
pub mod connect;
pub mod interact;
pub mod window;
pub mod batch;
pub mod monitor;
pub mod storage;

use hap_common::ffi::str_to_c;
use std::ffi::c_char;

hap_common::hap_module_init!("automation");
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

    #[test]
    fn test_describe() {
        let ptr = hap_module_describe();
        let s = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        let v: serde_json::Value = serde_json::from_str(s).unwrap();
        assert_eq!(v["name"], "automation");
        assert_eq!(v["uuid"], "a1b2c3d4-1000-4000-8000-000000000040");
        assert!(v["functions"].as_array().unwrap().len() >= 15);
        unsafe { free_c_string(ptr as *mut _); }
    }

    #[test]
    fn test_init() {
        let ptr = hap_module_init(std::ptr::null());
        let s = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        assert!(s.contains("automation"));
        assert!(s.contains("ok"));
        unsafe { free_c_string(ptr as *mut _); }
    }

    fn call(func: extern "C" fn(*const c_char) -> *const c_char, params: serde_json::Value) -> serde_json::Value {
        let input = hap_common::ffi::str_to_c(&params.to_string());
        let result = func(input);
        unsafe { free_c_string(input as *mut _); }
        let s = unsafe { CStr::from_ptr(result) }.to_str().unwrap().to_string();
        unsafe { free_c_string(result as *mut _); }
        serde_json::from_str(&s).unwrap()
    }

    #[test]
    fn test_connect_no_api() {
        let r = call(connect::hap_automation_connect, serde_json::json!({
            "appId": "nonexistent-app"
        }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_disconnect_invalid() {
        let r = call(connect::hap_automation_disconnect, serde_json::json!({
            "conn_id": "invalid_conn"
        }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_list_apps_no_api() {
        let r = call(connect::hap_automation_list_apps, serde_json::json!({}));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_query_no_conn() {
        let r = call(interact::hap_automation_query, serde_json::json!({
            "conn_id": "bad", "selector": "#test"
        }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_click_no_conn() {
        let r = call(interact::hap_automation_click, serde_json::json!({
            "conn_id": "bad", "selector": "button"
        }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_eval_no_conn() {
        let r = call(interact::hap_automation_eval, serde_json::json!({
            "conn_id": "bad", "code": "1+1"
        }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_resize_no_conn() {
        let r = call(window::hap_automation_resize, serde_json::json!({
            "conn_id": "bad", "width": 800, "height": 600
        }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_screenshot_no_conn() {
        let r = call(window::hap_automation_screenshot, serde_json::json!({
            "conn_id": "bad"
        }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_batch_no_conn() {
        let r = call(batch::hap_automation_batch, serde_json::json!({
            "conn_id": "bad",
            "steps": [{"action": "click", "selector": "#btn"}]
        }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_dom_tree_no_conn() {
        let r = call(monitor::hap_automation_dom_tree, serde_json::json!({
            "conn_id": "bad"
        }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_console_start_no_conn() {
        let r = call(monitor::hap_automation_console_start, serde_json::json!({
            "conn_id": "bad"
        }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_network_start_no_conn() {
        let r = call(monitor::hap_automation_network_start, serde_json::json!({
            "conn_id": "bad"
        }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_storage_get_no_conn() {
        let r = call(storage::hap_automation_storage_get, serde_json::json!({
            "conn_id": "bad"
        }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_mock_set_no_conn() {
        let r = call(storage::hap_automation_mock_set, serde_json::json!({
            "conn_id": "bad",
            "module": "test",
            "function": "fn1",
            "response": {"ok": true}
        }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_dom_diff_local() {
        let r = call(storage::hap_automation_dom_diff, serde_json::json!({
            "before": {"tag": "div", "text": "hello"},
            "after": {"tag": "div", "text": "world"}
        }));
        assert!(r.get("error").is_none());
        assert_eq!(r["count"], 1);
    }
}
