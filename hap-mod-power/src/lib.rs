pub mod funcs;

use hap_common::ffi::str_to_c;
use std::ffi::c_char;

hap_common::hap_module_init!("power");
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
        assert_eq!(v["name"], "power");
        assert_eq!(v["functions"].as_array().unwrap().len(), 9);
        unsafe { free_c_string(ptr as *mut _); }
    }

    #[test]
    fn test_battery_status() {
        let r = call(hap_power_battery_status, json!({}));
        assert!(r.get("has_battery").is_some());
    }

    #[test]
    fn test_is_on_battery() {
        let r = call(hap_power_is_on_battery, json!({}));
        assert!(r.is_boolean());
    }

    #[test]
    fn test_prevent_allow_sleep() {
        let r = call(hap_power_prevent_sleep, json!({"reason": "test"}));
        let lock_id = r["lock_id"].as_str().unwrap().to_string();

        let r = call(hap_power_allow_sleep, json!({"lock_id": lock_id}));
        assert_eq!(r, true);
    }

    #[test]
    fn test_list_locks() {
        let r = call(hap_power_list_locks, json!({}));
        assert!(r.is_array());
    }

    #[test]
    fn test_idle_time() {
        let r = call(hap_power_idle_time, json!({}));
        assert!(r.is_number());
    }
}
