pub mod funcs;

use hap_common::ffi::str_to_c;
use std::ffi::c_char;

hap_common::hap_module_init!("usb");
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
        assert_eq!(v["name"], "usb");
        assert!(v["functions"].as_array().unwrap().len() >= 8);
        unsafe { free_c_string(ptr as *mut _); }
    }

    #[test]
    fn test_list_devices() {
        let r = call(funcs::hap_usb_list_devices, json!({}));
        assert!(r.is_array());
    }

    #[test]
    fn test_list_devices_with_filter() {
        let r = call(funcs::hap_usb_list_devices, json!({ "vendor_id": 0x1234 }));
        assert!(r.is_array());
    }

    #[test]
    fn test_open_nonexistent_device() {
        let r = call(funcs::hap_usb_open, json!({ "vendor_id": 9999, "product_id": 9999 }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_close_invalid() {
        let r = call(funcs::hap_usb_close, json!({ "handle_id": "nonexistent" }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_bulk_transfer_invalid_handle() {
        let r = call(funcs::hap_usb_bulk_transfer_out, json!({
            "handle_id": "nope", "endpoint": 1, "data": "0102"
        }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_reset_invalid() {
        let r = call(funcs::hap_usb_reset_device, json!({ "handle_id": "nope" }));
        assert!(r.get("error").is_some());
    }
}
