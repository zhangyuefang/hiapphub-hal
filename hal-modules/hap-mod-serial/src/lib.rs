pub mod funcs;

use hap_common::ffi::str_to_c;
use std::ffi::c_char;

hap_common::hap_module_init!("serial");
hap_common::hap_free_string!();

#[no_mangle]
pub extern "C" fn hap_module_describe() -> *const c_char {
    str_to_c(include_str!("../manifest.json"))
}

#[cfg(test)]
mod tests {
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
        assert_eq!(v["name"], "serial");
        assert_eq!(v["functions"].as_array().unwrap().len(), 17);
        unsafe { free_c_string(ptr as *mut _); }
    }

    #[test]
    fn test_list_ports() {
        let r = call(super::funcs::hap_serial_list_ports, serde_json::json!({}));
        assert!(r.is_array());
    }

    #[test]
    fn test_list_open() {
        let r = call(super::funcs::hap_serial_list_open, serde_json::json!({}));
        assert!(r.is_array());
    }
}
