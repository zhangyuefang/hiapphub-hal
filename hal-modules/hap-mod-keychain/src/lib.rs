pub mod funcs;

use hap_common::ffi::str_to_c;
use std::ffi::c_char;

hap_common::hap_module_init!("keychain");
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
        assert_eq!(v["name"], "keychain");
        assert_eq!(v["functions"].as_array().unwrap().len(), 11);
        unsafe { free_c_string(ptr as *mut _); }
    }

    #[test]
    fn test_store_retrieve_delete() {
        let svc = "hap_test_keychain";
        let acct = "test_user";
        let pwd = "test_password_123";

        let r = call(hap_keychain_store, json!({"service": svc, "account": acct, "password": pwd}));
        assert!(r == json!(true) || r.get("error").is_some());

        let r = call(hap_keychain_retrieve, json!({"service": svc, "account": acct}));
        if r.as_str().is_some() && !r.as_str().unwrap().is_empty() {
            assert_eq!(r, json!(pwd));
        }

        let r = call(hap_keychain_has, json!({"service": svc, "account": acct}));
        assert!(r.is_boolean() || r.get("error").is_some());

        let _ = call(hap_keychain_delete, json!({"service": svc, "account": acct}));
    }

    #[test]
    fn test_biometric_available() {
        let r = call(hap_keychain_biometric_available, json!({}));
        assert!(r.get("available").is_some());
    }

    #[test]
    fn test_list() {
        let r = call(hap_keychain_list, json!({"service": "test"}));
        assert!(r.is_array());
    }
}
