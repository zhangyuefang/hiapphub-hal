mod funcs;

use hap_common::{hap_free_string, hap_module_init};

hap_module_init!("system");
hap_free_string!();

#[no_mangle]
pub extern "C" fn hap_module_describe() -> *const std::os::raw::c_char {
    hap_common::ffi::str_to_c(include_str!("../manifest.json"))
}

#[cfg(test)]
mod tests {
    use std::ffi::{CStr, CString};
    use serde_json::Value;

    fn call(func: extern "C" fn(*const std::os::raw::c_char) -> *const std::os::raw::c_char, json_str: &str) -> Value {
        let cs = CString::new(json_str).unwrap();
        let result = func(cs.as_ptr());
        assert!(!result.is_null());
        let s = unsafe { CStr::from_ptr(result) }.to_str().unwrap().to_string();
        unsafe { super::hap_free_string(result as *mut _) };
        serde_json::from_str(&s).unwrap()
    }

    #[test]
    fn test_os_info() {
        let r = call(super::funcs::hap_system_os_info, "{}");
        assert!(r["os"].as_str().is_some());
        assert!(r["arch"].as_str().is_some());
    }

    #[test]
    fn test_cpu_info() {
        let r = call(super::funcs::hap_system_cpu_info, "{}");
        assert!(r["cores_logical"].as_i64().unwrap() > 0);
    }

    #[test]
    fn test_memory_info() {
        let r = call(super::funcs::hap_system_memory_info, "{}");
        assert!(r["total_bytes"].as_i64().unwrap() > 0);
    }

    #[test]
    fn test_disk_info() {
        let r = call(super::funcs::hap_system_disk_info, "{}");
        assert!(r.as_array().unwrap().len() > 0);
    }

    #[test]
    fn test_hostname() {
        let r = call(super::funcs::hap_system_hostname, "{}");
        assert!(r.as_str().unwrap().len() > 0);
    }

    #[test]
    fn test_username() {
        let r = call(super::funcs::hap_system_username, "{}");
        assert!(r.as_str().unwrap().len() > 0);
    }

    #[test]
    fn test_home_dir() {
        let r = call(super::funcs::hap_system_home_dir, "{}");
        assert!(r.as_str().unwrap().len() > 0);
    }

    #[test]
    fn test_uptime() {
        let r = call(super::funcs::hap_system_uptime, "{}");
        assert!(r.as_i64().unwrap() > 0);
    }

    #[test]
    fn test_locale() {
        let r = call(super::funcs::hap_system_locale, "{}");
        assert!(r["full"].as_str().is_some());
    }

    #[test]
    fn test_theme() {
        let r = call(super::funcs::hap_system_theme, "{}");
        let t = r.as_str().unwrap();
        assert!(t == "light" || t == "dark");
    }

    #[test]
    fn test_is_elevated() {
        let r = call(super::funcs::hap_system_is_elevated, "{}");
        assert!(r.is_boolean());
    }

    #[test]
    fn test_shell_version() {
        let r = call(super::funcs::hap_system_shell_version, "{}");
        assert!(r.as_str().is_some());
    }

    #[test]
    fn test_total_memory_mb() {
        let r = call(super::funcs::hap_system_total_memory_mb, "{}");
        assert!(r.as_i64().unwrap() > 0);
    }

    #[test]
    fn test_free_memory_mb() {
        let r = call(super::funcs::hap_system_free_memory_mb, "{}");
        assert!(r.as_i64().unwrap() >= 0);
    }

    #[test]
    fn test_machine_id() {
        let r = call(super::funcs::hap_system_machine_id, "{}");
        assert!(r.as_str().unwrap().len() >= 32);
    }

    #[test]
    fn test_app_data_dir() {
        let r = call(super::funcs::hap_system_app_data_dir, r#"{"app_id":"test-app"}"#);
        assert!(r.as_str().unwrap().contains("test-app"));
    }

    #[test]
    fn test_cpu_usage() {
        let r = call(super::funcs::hap_system_cpu_usage, r#"{"interval_ms":200}"#);
        assert!(r["total_percent"].as_f64().is_some());
    }

    #[test]
    fn test_accent_color() {
        let r = call(super::funcs::hap_system_accent_color, "{}");
        assert!(r.as_str().is_some());
    }

    #[test]
    fn test_describe() {
        let ptr = super::hap_module_describe();
        let s = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        let v: Value = serde_json::from_str(s).unwrap();
        assert_eq!(v["name"], "system");
        unsafe { super::hap_free_string(ptr as *mut _) };
    }
}
