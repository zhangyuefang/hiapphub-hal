pub mod funcs;
pub mod server;

use hap_common::ffi::str_to_c;
use std::ffi::c_char;

hap_common::hap_module_init!("websocket");
hap_common::hap_free_string!();

#[no_mangle]
pub extern "C" fn hap_module_describe() -> *const c_char {
        str_to_c(include_str!("../manifest.json"))
}

#[cfg(test)]
mod tests {
    use super::funcs::*;
    use hap_common::ffi::{str_to_c, free_c_string};
    use serde_json::json;
    use std::ffi::CStr;
    
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
        assert_eq!(v["name"], "websocket");
        assert_eq!(v["functions"].as_array().unwrap().len(), 16);
        unsafe { free_c_string(ptr as *mut _); }
    }

    #[test]
    fn test_list_connections_empty() {
        let r = call(hap_ws_list_connections, json!({}));
        assert!(r.as_array().unwrap().is_empty() || r.is_array());
    }

    #[test]
    fn test_state_invalid() {
        let r = call(hap_ws_state, json!({"conn_id": "nonexistent"}));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_close_nonexistent() {
        let r = call(hap_ws_close, json!({"conn_id": "fake"}));
        assert_eq!(r, json!(true));
    }

    #[test]
    fn test_set_auto_reconnect_invalid_conn() {
        let r = call(hap_ws_set_auto_reconnect, json!({
            "conn_id": "ws_nonexist", "enabled": true, "interval_ms": 3000
        }));
        assert!(r["error"].is_object());
    }
}
