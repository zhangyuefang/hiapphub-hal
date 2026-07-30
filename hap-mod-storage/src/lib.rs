mod funcs;

use hap_common::{hap_free_string, hap_module_init};

hap_module_init!("storage");
hap_free_string!();

#[no_mangle]
pub extern "C" fn hap_module_describe() -> *const std::os::raw::c_char {
    hap_common::ffi::str_to_c(include_str!("../manifest.json"))
}

#[cfg(test)]
mod tests {
    use std::ffi::{CStr, CString};
    use serde_json::{json, Value};

    fn call(func: extern "C" fn(*const std::os::raw::c_char) -> *const std::os::raw::c_char, json_str: &str) -> Value {
        let cs = CString::new(json_str).unwrap();
        let result = func(cs.as_ptr());
        assert!(!result.is_null());
        let s = unsafe { CStr::from_ptr(result) }.to_str().unwrap().to_string();
        unsafe { super::hap_free_string(result as *mut _) };
        serde_json::from_str(&s).unwrap()
    }

    fn unique_dir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("hap_storage_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn test_set_get() {
        let dir = unique_dir("sg");
        let dp = dir.to_string_lossy();
        call(super::funcs::hap_storage_set, &format!(r#"{{"namespace":"ns","key":"k1","value":"v1","_storage_dir":"{dp}"}}"#));
        let r = call(super::funcs::hap_storage_get, &format!(r#"{{"namespace":"ns","key":"k1","_storage_dir":"{dp}"}}"#));
        assert_eq!(r.as_str().unwrap(), "v1");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_has_delete() {
        let dir = unique_dir("hd");
        let dp = dir.to_string_lossy();
        call(super::funcs::hap_storage_set, &format!(r#"{{"namespace":"ns","key":"k","value":"v","_storage_dir":"{dp}"}}"#));
        assert_eq!(call(super::funcs::hap_storage_has, &format!(r#"{{"namespace":"ns","key":"k","_storage_dir":"{dp}"}}"#)), json!(true));
        call(super::funcs::hap_storage_delete, &format!(r#"{{"namespace":"ns","key":"k","_storage_dir":"{dp}"}}"#));
        assert_eq!(call(super::funcs::hap_storage_has, &format!(r#"{{"namespace":"ns","key":"k","_storage_dir":"{dp}"}}"#)), json!(false));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_keys_count() {
        let dir = unique_dir("kc");
        let dp = dir.to_string_lossy();
        call(super::funcs::hap_storage_set, &format!(r#"{{"namespace":"ns","key":"a1","value":"v","_storage_dir":"{dp}"}}"#));
        call(super::funcs::hap_storage_set, &format!(r#"{{"namespace":"ns","key":"a2","value":"v","_storage_dir":"{dp}"}}"#));
        call(super::funcs::hap_storage_set, &format!(r#"{{"namespace":"ns","key":"b1","value":"v","_storage_dir":"{dp}"}}"#));
        let keys = call(super::funcs::hap_storage_keys, &format!(r#"{{"namespace":"ns","prefix":"a","_storage_dir":"{dp}"}}"#));
        assert_eq!(keys.as_array().unwrap().len(), 2);
        let count = call(super::funcs::hap_storage_count, &format!(r#"{{"namespace":"ns","_storage_dir":"{dp}"}}"#));
        assert_eq!(count.as_i64().unwrap(), 3);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_get_many_set_many() {
        let dir = unique_dir("gmsm");
        let dp = dir.to_string_lossy();
        call(super::funcs::hap_storage_set_many, &format!(r#"{{"namespace":"ns","entries":{{"x":"1","y":"2"}},"_storage_dir":"{dp}"}}"#));
        let r = call(super::funcs::hap_storage_get_many, &format!(r#"{{"namespace":"ns","keys":["x","y","z"],"_storage_dir":"{dp}"}}"#));
        assert_eq!(r["x"], "1");
        assert_eq!(r["y"], "2");
        assert_eq!(r["z"], json!(null));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_clear() {
        let dir = unique_dir("cl");
        let dp = dir.to_string_lossy();
        call(super::funcs::hap_storage_set_many, &format!(r#"{{"namespace":"ns","entries":{{"a":"1","b":"2","c":"3"}},"_storage_dir":"{dp}"}}"#));
        let r = call(super::funcs::hap_storage_clear, &format!(r#"{{"namespace":"ns","_storage_dir":"{dp}"}}"#));
        assert_eq!(r.as_i64().unwrap(), 3);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_increment() {
        let dir = unique_dir("inc");
        let dp = dir.to_string_lossy();
        let r1 = call(super::funcs::hap_storage_increment, &format!(r#"{{"namespace":"ns","key":"counter","_storage_dir":"{dp}"}}"#));
        assert_eq!(r1.as_i64().unwrap(), 1);
        let r2 = call(super::funcs::hap_storage_increment, &format!(r#"{{"namespace":"ns","key":"counter","delta":5,"_storage_dir":"{dp}"}}"#));
        assert_eq!(r2.as_i64().unwrap(), 6);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_set_json_get_json() {
        let dir = unique_dir("jj");
        let dp = dir.to_string_lossy();
        call(super::funcs::hap_storage_set_json, &format!(r#"{{"namespace":"ns","key":"obj","value":{{"a":1,"b":"two"}},"_storage_dir":"{dp}"}}"#));
        let r = call(super::funcs::hap_storage_get_json, &format!(r#"{{"namespace":"ns","key":"obj","_storage_dir":"{dp}"}}"#));
        assert_eq!(r["a"], 1);
        assert_eq!(r["b"], "two");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_export_import() {
        let dir = unique_dir("ei");
        let dp = dir.to_string_lossy();
        call(super::funcs::hap_storage_set_many, &format!(r#"{{"namespace":"ns","entries":{{"x":"1","y":"2"}},"_storage_dir":"{dp}"}}"#));
        let exp_path = dir.join("export.json");
        let ep = exp_path.to_string_lossy();
        let r = call(super::funcs::hap_storage_export, &format!(r#"{{"namespace":"ns","output_path":"{ep}","_storage_dir":"{dp}"}}"#));
        assert_eq!(r["keys"].as_i64().unwrap(), 2);
        let r2 = call(super::funcs::hap_storage_import, &format!(r#"{{"namespace":"ns2","input_path":"{ep}","_storage_dir":"{dp}"}}"#));
        assert_eq!(r2["imported"].as_i64().unwrap(), 2);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_entries_values() {
        let dir = unique_dir("ev");
        let dp = dir.to_string_lossy();
        call(super::funcs::hap_storage_set_many, &format!(r#"{{"namespace":"ns","entries":{{"k1":"v1","k2":"v2"}},"_storage_dir":"{dp}"}}"#));
        let vals = call(super::funcs::hap_storage_values, &format!(r#"{{"namespace":"ns","_storage_dir":"{dp}"}}"#));
        assert_eq!(vals.as_array().unwrap().len(), 2);
        let entries = call(super::funcs::hap_storage_entries, &format!(r#"{{"namespace":"ns","_storage_dir":"{dp}"}}"#));
        assert_eq!(entries.as_array().unwrap().len(), 2);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_delete_many() {
        let dir = unique_dir("dm");
        let dp = dir.to_string_lossy();
        call(super::funcs::hap_storage_set_many, &format!(r#"{{"namespace":"ns","entries":{{"a":"1","b":"2","c":"3"}},"_storage_dir":"{dp}"}}"#));
        let r = call(super::funcs::hap_storage_delete_many, &format!(r#"{{"namespace":"ns","keys":["a","c"],"_storage_dir":"{dp}"}}"#));
        assert_eq!(r.as_i64().unwrap(), 2);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_ttl() {
        let dir = unique_dir("ttl");
        let dp = dir.to_string_lossy();
        call(super::funcs::hap_storage_set_with_ttl, &format!(r#"{{"namespace":"ns","key":"temp","value":"val","ttl_ms":100,"_storage_dir":"{dp}"}}"#));
        let r1 = call(super::funcs::hap_storage_get, &format!(r#"{{"namespace":"ns","key":"temp","_storage_dir":"{dp}"}}"#));
        assert_eq!(r1.as_str().unwrap(), "val");
        std::thread::sleep(std::time::Duration::from_millis(200));
        let r2 = call(super::funcs::hap_storage_get, &format!(r#"{{"namespace":"ns","key":"temp","_storage_dir":"{dp}"}}"#));
        assert_eq!(r2.as_str().unwrap(), "");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_describe() {
        let ptr = super::hap_module_describe();
        let s = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        let v: Value = serde_json::from_str(s).unwrap();
        assert_eq!(v["name"], "storage");
        unsafe { super::hap_free_string(ptr as *mut _) };
    }
}
