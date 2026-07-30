pub mod funcs;
use hap_common::ffi::str_to_c;
use std::ffi::c_char;
hap_common::hap_module_init!("shell-ext");
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
        assert_eq!(v["name"], "shell-ext");
        assert_eq!(v["functions"].as_array().unwrap().len(), 20);
        unsafe { free_c_string(ptr as *mut _); }
    }

    #[test]
    fn test_get_special_dir() {
        let r = call(hap_shell_ext_get_special_dir, json!({"name": "home"}));
        assert!(!r.as_str().unwrap().is_empty());
    }

    #[test]
    fn test_get_env() {
        let r = call(hap_shell_ext_get_env, json!({"name": "PATH"}));
        assert!(!r.as_str().unwrap().is_empty());
    }

    #[test]
    fn test_get_mime_type() {
        let r = call(hap_shell_ext_get_mime_type, json!({"path": "test.png"}));
        assert_eq!(r.as_str().unwrap(), "image/png");
    }

    #[test]
    fn test_shortcut_exists() {
        let r = call(hap_shell_ext_shortcut_exists, json!({"shortcut_path": "/nonexistent"}));
        assert_eq!(r, json!(false));
    }

    #[test]
    fn test_list_printers() {
        let r = call(hap_shell_ext_list_printers, json!({}));
        assert!(r.is_array());
    }
}
