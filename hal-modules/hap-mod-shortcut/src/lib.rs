pub mod funcs;
use hap_common::ffi::str_to_c;
use std::ffi::c_char;
hap_common::hap_module_init!("shortcut");
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
    fn test_register_unregister() {
        let key = format!("Ctrl+Shift+F{}", std::process::id() % 12 + 1);
        let _ = call(hap_shortcut_register, json!({"accelerator": key, "callback_id": "cb1"}));
        let r = call(hap_shortcut_is_registered, json!({"accelerator": key}));
        assert_eq!(r, json!(true));
        let _ = call(hap_shortcut_unregister, json!({"accelerator": key}));
        let r2 = call(hap_shortcut_is_registered, json!({"accelerator": key}));
        assert_eq!(r2, json!(false));
    }

    #[test]
    fn test_list_and_register() {
        let key = format!("Alt+Shift+F{}", std::process::id() % 12 + 1);
        let _ = call(hap_shortcut_register, json!({"accelerator": key, "callback_id": "cb2"}));
        let r = call(hap_shortcut_is_registered, json!({"accelerator": key}));
        assert_eq!(r, json!(true));
        let list = call(hap_shortcut_list, json!({}));
        assert!(list.as_array().unwrap().len() >= 1);
        let _ = call(hap_shortcut_unregister, json!({"accelerator": key}));
    }
}
