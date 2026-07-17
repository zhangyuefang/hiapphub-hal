pub mod funcs;

use hap_common::ffi::str_to_c;
use std::ffi::c_char;

hap_common::hap_module_init!("email");
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
        assert_eq!(v["name"], "email");
        assert!(v["functions"].as_array().unwrap().len() >= 6);
        unsafe { free_c_string(ptr as *mut _); }
    }

    #[test]
    fn test_send_invalid_from() {
        let r = call(funcs::hap_email_send, json!({
            "smtp_host": "smtp.test.invalid",
            "username": "user",
            "password": "pass",
            "from": "not-an-email",
            "to": ["test@example.com"],
            "subject": "Test",
            "body": "Hello"
        }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_send_invalid_to() {
        let r = call(funcs::hap_email_send, json!({
            "smtp_host": "smtp.test.invalid",
            "username": "user",
            "password": "pass",
            "from": "valid@example.com",
            "to": ["not-valid"],
            "subject": "Test",
            "body": "Hello"
        }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_fetch_invalid_host() {
        let r = call(funcs::hap_email_fetch, json!({
            "host": "imap.test.invalid",
            "username": "user",
            "password": "pass"
        }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_list_folders_invalid_host() {
        let r = call(funcs::hap_email_list_folders, json!({
            "host": "imap.test.invalid",
            "username": "user",
            "password": "pass"
        }));
        assert!(r.get("error").is_some());
    }
}
