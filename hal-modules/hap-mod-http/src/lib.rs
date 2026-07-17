pub mod request;

use hap_common::ffi::str_to_c;
use std::ffi::c_char;

hap_common::hap_module_init!("http");
hap_common::hap_free_string!();

#[no_mangle]
pub extern "C" fn hap_module_describe() -> *const c_char {
    str_to_c(include_str!("../manifest.json"))
}

#[cfg(test)]
mod tests {
    use super::request::*;
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
        assert_eq!(v["name"], "http");
        assert!(v["functions"].as_array().unwrap().len() >= 25);
        unsafe { free_c_string(ptr as *mut _); }
    }

    #[test]
    fn test_get_request() {
        let r = call(hap_http_get, json!({"url": "https://httpbin.org/get", "timeout_ms": 15000}));
        if let Some(s) = r.get("status") {
            assert!(s.as_i64().unwrap() > 0);
        } else {
            assert!(r.get("error").is_some(), "expected status or error: {r}");
        }
    }

    #[test]
    fn test_post_json() {
        let r = call(hap_http_post_json, json!({"url": "https://httpbin.org/post", "data": {"hello": "world"}, "timeout_ms": 15000}));
        if let Some(s) = r.get("status") {
            assert!(s.as_i64().unwrap() > 0);
        } else {
            assert!(r.get("error").is_some(), "expected status or error: {r}");
        }
    }

    #[test]
    fn test_set_default_headers() {
        let r = call(hap_http_set_default_headers, json!({"headers": {"X-Test": "value"}}));
        assert_eq!(r, json!(true));
    }

    #[test]
    fn test_set_default_timeout() {
        let r = call(hap_http_set_default_timeout, json!({"timeout_ms": 5000}));
        assert_eq!(r, json!(true));
    }

    #[test]
    fn test_set_proxy_null() {
        let r = call(hap_http_set_proxy, json!({"proxy_url": null}));
        assert_eq!(r, json!(true));
    }

    #[test]
    fn test_cookie_operations() {
        let _ = call(hap_http_set_cookie, json!({
            "url": "https://example.com", "name": "test", "value": "v1"
        }));
        let cookies = call(hap_http_get_cookies, json!({"url": "https://example.com"}));
        assert!(!cookies.as_array().unwrap().is_empty());
        let _ = call(hap_http_clear_cookies, json!({"url": "https://example.com"}));
        let cookies2 = call(hap_http_get_cookies, json!({"url": "https://example.com"}));
        assert!(cookies2.as_array().unwrap().is_empty());
    }

    #[test]
    fn test_set_basic_auth() {
        let r = call(hap_http_set_basic_auth, json!({"username": "user", "password": "pass"}));
        assert_eq!(r, json!(true));
    }

    #[test]
    fn test_sse_list_empty() {
        let r = call(hap_http_list_sse, json!({}));
        assert!(r.is_array());
    }

    #[test]
    fn test_cancel_invalid() {
        let r = call(hap_http_cancel, json!({"request_id": "nonexistent"}));
        assert!(r["error"].is_object());
    }

    #[test]
    fn test_sse_close_invalid() {
        let r = call(hap_http_sse_close, json!({"conn_id": "nonexistent"}));
        assert!(r["error"].is_object());
    }

    #[test]
    fn test_sse_poll_invalid() {
        let r = call(hap_http_sse_poll, json!({"conn_id": "nonexistent"}));
        assert!(r["error"].is_object());
    }

    #[test]
    fn test_head_request() {
        let r = call(hap_http_head, json!({"url": "https://httpbin.org/get", "timeout_ms": 15000}));
        if let Some(s) = r.get("status") {
            assert!(s.as_i64().unwrap() > 0);
            assert!(r.get("body").is_none());
        } else {
            assert!(r.get("error").is_some(), "expected status or error: {r}");
        }
    }
}
