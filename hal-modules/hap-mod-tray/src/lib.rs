pub mod funcs;
#[cfg(target_os = "macos")]
pub mod macos;
use hap_common::ffi::str_to_c;
use std::ffi::c_char;
hap_common::hap_module_init!("tray");
hap_common::hap_free_string!();

#[no_mangle]
pub extern "C" fn hap_module_describe() -> *const c_char {
        str_to_c(include_str!("../manifest.json"))
}

#[cfg(test)]
mod tests {
    use hap_common::ffi::free_c_string;
    use std::ffi::CStr;

    #[test]
    fn test_describe() {
        let ptr = super::hap_module_describe();
        let s = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        let v: serde_json::Value = serde_json::from_str(s).unwrap();
        assert_eq!(v["name"], "tray");
        assert_eq!(v["functions"].as_array().unwrap().len(), 13);
        unsafe { free_c_string(ptr as *mut _); }
    }
}
