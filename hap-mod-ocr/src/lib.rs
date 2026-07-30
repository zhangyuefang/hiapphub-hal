pub mod funcs;

use hap_common::ffi::str_to_c;
use std::ffi::c_char;

hap_common::hap_module_init!("ocr");
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
        assert_eq!(v["name"], "ocr");
        assert!(v["functions"].as_array().unwrap().len() >= 5);
        unsafe { free_c_string(ptr as *mut _); }
    }

    #[test]
    fn test_recognize_nonexistent_file() {
        let r = call(funcs::hap_ocr_recognize, json!({ "image_path": "/tmp/nonexistent_image.png" }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_recognize_base64_invalid() {
        let r = call(funcs::hap_ocr_recognize_base64, json!({ "base64_data": "not-valid-base64!!!" }));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_get_supported_languages() {
        let r = call(funcs::hap_ocr_get_supported_languages, json!({}));
        assert!(r.is_array());
        assert!(!r.as_array().unwrap().is_empty());
    }
}
