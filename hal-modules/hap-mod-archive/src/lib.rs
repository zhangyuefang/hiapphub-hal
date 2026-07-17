mod funcs;

use hap_common::{hap_free_string, hap_module_init};

hap_module_init!("archive");
hap_free_string!();

#[no_mangle]
pub extern "C" fn hap_module_describe() -> *const std::os::raw::c_char {
    hap_common::ffi::str_to_c(include_str!("../manifest.json"))
}

#[cfg(test)]
mod tests {
    use std::ffi::{CStr, CString};
    use serde_json::{json, Value};

    fn call(func: extern "C" fn(*const std::os::raw::c_char) -> *const std::os::raw::c_char, s: &str) -> Value {
        let cs = CString::new(s).unwrap();
        let r = func(cs.as_ptr());
        assert!(!r.is_null());
        let out = unsafe { CStr::from_ptr(r) }.to_str().unwrap().to_string();
        unsafe { super::hap_free_string(r as *mut _) };
        serde_json::from_str(&out).unwrap()
    }

    fn td(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("hap_archive_{}_{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn test_zip_unzip() {
        let d = td("zu");
        let src = d.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("a.txt"), "hello").unwrap();
        std::fs::write(src.join("b.txt"), "world").unwrap();
        let sp = src.join("a.txt").to_string_lossy().to_string();
        let sp2 = src.join("b.txt").to_string_lossy().to_string();
        let zp = d.join("test.zip").to_string_lossy().to_string();
        let r = call(super::funcs::hap_archive_zip, &format!(r#"{{"source_paths":["{sp}","{sp2}"],"dest_path":"{zp}"}}"#));
        assert_eq!(r["success"], json!(true));
        assert_eq!(r["file_count"], json!(2));
        let dest = d.join("out");
        let dp = dest.to_string_lossy().to_string();
        let r2 = call(super::funcs::hap_archive_unzip, &format!(r#"{{"archive_path":"{zp}","dest_dir":"{dp}"}}"#));
        assert_eq!(r2["files"].as_array().unwrap().len(), 2);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn test_list_entries() {
        let d = td("le");
        std::fs::write(d.join("x.txt"), "data").unwrap();
        let fp = d.join("x.txt").to_string_lossy().to_string();
        let zp = d.join("list.zip").to_string_lossy().to_string();
        call(super::funcs::hap_archive_zip, &format!(r#"{{"source_paths":["{fp}"],"dest_path":"{zp}"}}"#));
        let r = call(super::funcs::hap_archive_list_entries, &format!(r#"{{"archive_path":"{zp}"}}"#));
        assert_eq!(r.as_array().unwrap().len(), 1);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn test_compress_decompress_bytes() {
        let enc = call(super::funcs::hap_archive_compress_bytes, r#"{"data":"hello world","algorithm":"gzip"}"#);
        let enc_str = enc.as_str().unwrap();
        let dec = call(super::funcs::hap_archive_decompress_bytes, &format!(r#"{{"data":"{enc_str}","algorithm":"gzip"}}"#));
        assert_eq!(dec.as_str().unwrap(), "hello world");
    }

    #[test]
    fn test_compress_decompress_zstd() {
        let enc = call(super::funcs::hap_archive_compress_bytes, r#"{"data":"zstd test data","algorithm":"zstd"}"#);
        let enc_str = enc.as_str().unwrap();
        let dec = call(super::funcs::hap_archive_decompress_bytes, &format!(r#"{{"data":"{enc_str}","algorithm":"zstd"}}"#));
        assert_eq!(dec.as_str().unwrap(), "zstd test data");
    }

    #[test]
    fn test_is_archive() {
        let d = td("ia");
        std::fs::write(d.join("f.txt"), "not archive").unwrap();
        let fp = d.join("f.txt").to_string_lossy().to_string();
        let r = call(super::funcs::hap_archive_is_archive, &format!(r#"{{"path":"{fp}"}}"#));
        assert_eq!(r["is_archive"], json!(false));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn test_tar_untar() {
        let d = td("tu");
        let src = d.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("t.txt"), "tar data").unwrap();
        let fp = src.join("t.txt").to_string_lossy().to_string();
        let tp = d.join("test.tar.gz").to_string_lossy().to_string();
        let r = call(super::funcs::hap_archive_tar, &format!(r#"{{"source_paths":["{fp}"],"dest_path":"{tp}","compress":"gzip"}}"#));
        assert_eq!(r["success"], json!(true));
        let dest = d.join("tarout");
        let dp = dest.to_string_lossy().to_string();
        let r2 = call(super::funcs::hap_archive_untar, &format!(r#"{{"archive_path":"{tp}","dest_dir":"{dp}"}}"#));
        assert!(r2["files"].as_array().unwrap().len() >= 1);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn test_read_entry_bytes() {
        let d = td("reb");
        std::fs::write(d.join("entry.txt"), "content123").unwrap();
        let fp = d.join("entry.txt").to_string_lossy().to_string();
        let zp = d.join("re.zip").to_string_lossy().to_string();
        call(super::funcs::hap_archive_zip, &format!(r#"{{"source_paths":["{fp}"],"dest_path":"{zp}"}}"#));
        let r = call(super::funcs::hap_archive_read_entry_bytes, &format!(r#"{{"archive_path":"{zp}","entry_name":"entry.txt"}}"#));
        let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, r.as_str().unwrap()).unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), "content123");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn test_describe() {
        let ptr = super::hap_module_describe();
        let s = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        let v: Value = serde_json::from_str(s).unwrap();
        assert_eq!(v["name"], "archive");
        assert_eq!(v["functions"].as_array().unwrap().len(), 14);
        unsafe { super::hap_free_string(ptr as *mut _) };
    }
}
