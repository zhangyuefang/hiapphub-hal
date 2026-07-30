pub mod funcs;

use hap_common::ffi::str_to_c;
use std::ffi::c_char;

hap_common::hap_module_init!("browser");
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
        assert_eq!(v["name"], "browser");
        assert_eq!(v["uuid"], "a1b2c3d4-1000-4000-8000-000000000031");
        assert!(v["functions"].as_array().unwrap().len() >= 18);
        unsafe { free_c_string(ptr as *mut _); }
    }

    #[test]
    fn test_init() {
        let ptr = hap_module_init(std::ptr::null());
        let s = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        assert!(s.contains("browser"));
        assert!(s.contains("ok"));
        unsafe { free_c_string(ptr as *mut _); }
    }

    fn call(func: extern "C" fn(*const c_char) -> *const c_char, params: serde_json::Value) -> serde_json::Value {
        let input = str_to_c(&params.to_string());
        let result = func(input);
        unsafe { free_c_string(input as *mut _); }
        let s = unsafe { CStr::from_ptr(result) }.to_str().unwrap().to_string();
        unsafe { free_c_string(result as *mut _); }
        serde_json::from_str(&s).unwrap()
    }

    #[test]
    fn test_launch_no_browser() {
        let r = call(funcs::hap_browser_launch, serde_json::json!({
            "executable_path": "/nonexistent/browser"
        }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_connect_invalid_url() {
        let r = call(funcs::hap_browser_connect, serde_json::json!({
            "ws_url": "ws://127.0.0.1:19999/invalid"
        }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_close_invalid() {
        let r = call(funcs::hap_browser_close, serde_json::json!({
            "browser_id": "nonexistent"
        }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_new_page_invalid() {
        let r = call(funcs::hap_browser_new_page, serde_json::json!({
            "browser_id": "nonexistent"
        }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_navigate_invalid() {
        let r = call(funcs::hap_browser_navigate, serde_json::json!({
            "page_id": "nonexistent", "url": "https://example.com"
        }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_evaluate_invalid() {
        let r = call(funcs::hap_browser_evaluate, serde_json::json!({
            "page_id": "nonexistent", "expression": "1+1"
        }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_screenshot_invalid() {
        let r = call(funcs::hap_browser_screenshot, serde_json::json!({
            "page_id": "nonexistent"
        }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_list_pages_invalid() {
        let r = call(funcs::hap_browser_list_pages, serde_json::json!({
            "browser_id": "nonexistent"
        }));
        assert!(r.get("error").is_some());
    }

    #[test]
    #[ignore] // Requires Edge/Chrome installed. Run with: cargo test -p hap-mod-browser -- --ignored
    fn test_e2e_browser_full() {
        use serde_json::json;

        // Launch browser headless with explicit user-data-dir
        let tmp_dir = format!("/tmp/hap_browser_e2e_{}", std::process::id());
        let launch = call(funcs::hap_browser_launch, json!({
            "executable_path": "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
            "headless": true,
            "user_data_dir": tmp_dir,
            "args": ["--disable-extensions", "--disable-sync"]
        }));
        assert!(launch.get("browser_id").is_some(), "launch failed: {launch}");
        let browser_id = launch["browser_id"].as_str().unwrap().to_string();
        let _port = launch["port"].as_u64().unwrap();

        // Create new page
        let page = call(funcs::hap_browser_new_page, json!({
            "browser_id": browser_id, "url": "about:blank"
        }));
        assert!(page.get("page_id").is_some(), "new_page failed: {page}");
        let page_id = page["page_id"].as_str().unwrap().to_string();

        // Navigate
        let nav = call(funcs::hap_browser_navigate, json!({
            "page_id": page_id, "url": "data:text/html,<h1 id='hello'>Hello</h1><input id='inp'/>"
        }));
        assert!(!nav.get("error").is_some(), "navigate failed: {nav}");

        // Evaluate JS
        let eval_r = call(funcs::hap_browser_evaluate, json!({
            "page_id": page_id, "expression": "document.title"
        }));
        assert!(!eval_r.get("error").is_some(), "evaluate failed: {eval_r}");

        // Get HTML
        let html = call(funcs::hap_browser_get_html, json!({
            "page_id": page_id, "selector": "#hello"
        }));
        assert!(!html.get("error").is_some(), "get_html failed: {html}");
        let html_str = html.as_str().unwrap_or("");
        assert!(html_str.contains("Hello"), "HTML should contain Hello: {html_str}");

        // Wait for selector
        let wait = call(funcs::hap_browser_wait_for_selector, json!({
            "page_id": page_id, "selector": "#hello", "timeout_ms": 5000
        }));
        assert_eq!(wait, json!(true), "wait failed: {wait}");

        // Type text
        let type_r = call(funcs::hap_browser_type_text, json!({
            "page_id": page_id, "selector": "#inp", "text": "hello world"
        }));
        assert_eq!(type_r, json!(true), "type failed: {type_r}");

        // Screenshot
        let ss_path = "/tmp/hap_browser_test_ss.png";
        let ss = call(funcs::hap_browser_screenshot, json!({
            "page_id": page_id, "path": ss_path
        }));
        assert!(!ss.get("error").is_some(), "screenshot failed: {ss}");
        assert!(std::path::Path::new(ss_path).exists());
        std::fs::remove_file(ss_path).ok();

        // Get cookies (empty for data: URL)
        let cookies = call(funcs::hap_browser_get_cookies, json!({ "page_id": page_id }));
        assert!(cookies.is_array(), "cookies failed: {cookies}");

        // List pages
        let pages_list = call(funcs::hap_browser_list_pages, json!({ "browser_id": browser_id }));
        assert!(pages_list.is_array(), "list_pages failed: {pages_list}");
        assert!(!pages_list.as_array().unwrap().is_empty());

        // Query selector
        let qs = call(funcs::hap_browser_query_selector, json!({
            "page_id": page_id, "selector": "#hello"
        }));
        assert!(!qs.get("error").is_some(), "querySelector failed: {qs}");

        // Close page
        let close_p = call(funcs::hap_browser_close_page, json!({ "page_id": page_id }));
        assert_eq!(close_p, json!(true), "close_page failed: {close_p}");

        // Close browser
        let close_b = call(funcs::hap_browser_close, json!({ "browser_id": browser_id }));
        assert_eq!(close_b, json!(true), "close_browser failed: {close_b}");
    }
}
