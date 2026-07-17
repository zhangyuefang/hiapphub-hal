pub mod funcs;

use hap_common::ffi::str_to_c;
use std::ffi::c_char;

hap_common::hap_module_init!("bluetooth");
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
        assert_eq!(v["name"], "bluetooth");
        assert!(v["functions"].as_array().unwrap().len() >= 10);
        unsafe { free_c_string(ptr as *mut _); }
    }

    #[test]
    fn test_scan_stop() {
        let r = call(funcs::hap_bluetooth_scan_stop, json!({}));
        assert_eq!(r, json!(true));
    }

    #[test]
    fn test_connect_disconnect() {
        let r = call(funcs::hap_bluetooth_connect, json!({ "device_id": "AA:BB:CC:DD:EE:FF" }));
        assert_eq!(r["connected"], true);

        let is = call(funcs::hap_bluetooth_is_connected, json!({ "device_id": "AA:BB:CC:DD:EE:FF" }));
        assert_eq!(is, json!(true));

        let d = call(funcs::hap_bluetooth_disconnect, json!({ "device_id": "AA:BB:CC:DD:EE:FF" }));
        assert_eq!(d, json!(true));

        let is2 = call(funcs::hap_bluetooth_is_connected, json!({ "device_id": "AA:BB:CC:DD:EE:FF" }));
        assert_eq!(is2, json!(false));
    }

    #[test]
    fn test_disconnect_invalid() {
        let r = call(funcs::hap_bluetooth_disconnect, json!({ "device_id": "nonexistent" }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_read_char_not_connected() {
        let r = call(funcs::hap_bluetooth_read_characteristic, json!({
            "device_id": "no_device", "service_uuid": "1800", "characteristic_uuid": "2a00"
        }));
        assert!(r.get("error").is_some());
    }
}
