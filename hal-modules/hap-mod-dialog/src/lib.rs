pub mod funcs;
use hap_common::ffi::str_to_c;
use std::ffi::c_char;

hap_common::hap_module_init!("dialog");
hap_common::hap_free_string!();

#[no_mangle]
pub extern "C" fn hap_module_describe() -> *const c_char {
        str_to_c(include_str!("../manifest.json"))
}

#[cfg(test)]
mod tests {
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
        assert_eq!(v["name"], "dialog");
        assert_eq!(v["functions"].as_array().unwrap().len(), 15);
        unsafe { free_c_string(ptr as *mut _); }
    }

    #[test]
    fn test_progress_lifecycle() {
        use super::funcs::*;
        let r = call(hap_dialog_progress, json!({"title": "Test", "cancellable": true}));
        let did = r["dialog_id"].as_str().unwrap().to_string();
        assert!(did.starts_with("prog_"));

        let r2 = call(hap_dialog_progress_update, json!({"dialog_id": did, "value": 0.5, "message": "half"}));
        assert_eq!(r2["cancelled"], false);

        let r3 = call(hap_dialog_progress_close, json!({"dialog_id": did}));
        assert_eq!(r3, json!(true));

        let r4 = call(hap_dialog_progress_update, json!({"dialog_id": "nonexistent", "value": 0.1}));
        assert!(r4["error"].is_object());
    }
}
