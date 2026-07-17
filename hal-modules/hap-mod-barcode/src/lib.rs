pub mod funcs;
use hap_common::ffi::str_to_c;
use std::ffi::c_char;
hap_common::hap_module_init!("barcode");
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
        assert_eq!(v["name"], "barcode");
        assert_eq!(v["functions"].as_array().unwrap().len(), 7);
        unsafe { free_c_string(ptr as *mut _); }
    }

    #[test]
    fn test_generate_qr_png() {
        let r = call(hap_barcode_generate_qr, json!({"data": "https://example.com"}));
        assert!(r["data"].as_str().unwrap().len() > 100);
        assert!(r["width"].as_i64().unwrap() > 0);
    }

    #[test]
    fn test_generate_qr_svg() {
        let r = call(hap_barcode_generate_qr, json!({"data": "hello", "format": "svg"}));
        assert!(r["data"].as_str().unwrap().contains("<svg"));
    }

    #[test]
    fn test_save_qr() {
        let d = std::env::temp_dir().join(format!("hap_barcode_{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let out = d.join("qr.png").to_string_lossy().to_string();
        let r = call(hap_barcode_save_qr, json!({"data": "test", "output_path": out}));
        assert_eq!(r, json!(true));
    }
}
