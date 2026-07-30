pub mod basic_ops;
pub mod dir_ops;
pub mod advanced_ops;

use hap_common::{hap_free_string, hap_module_init};

hap_module_init!("fs");
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

    fn tdir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("hap_fs_{}_{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn test_read_write_text() {
        let d = tdir("rwt");
        let p = d.join("test.txt");
        let ps = p.to_string_lossy();
        call(super::basic_ops::hap_fs_write_text_file, &format!(r#"{{"path":"{ps}","content":"hello"}}"#));
        let r = call(super::basic_ops::hap_fs_read_text_file, &format!(r#"{{"path":"{ps}"}}"#));
        assert_eq!(r.as_str().unwrap(), "hello");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn test_append() {
        let d = tdir("app");
        let p = d.join("append.txt");
        let ps = p.to_string_lossy();
        std::fs::write(&p, "a").unwrap();
        call(super::basic_ops::hap_fs_append_text_file, &format!(r#"{{"path":"{ps}","content":"b"}}"#));
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "ab");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn test_binary_roundtrip() {
        let d = tdir("bin");
        let p = d.join("bin.dat");
        let ps = p.to_string_lossy();
        call(super::basic_ops::hap_fs_write_binary, &format!(r#"{{"path":"{ps}","data":"AQID"}}"#));
        let r = call(super::basic_ops::hap_fs_read_binary, &format!(r#"{{"path":"{ps}"}}"#));
        assert_eq!(r.as_str().unwrap(), "AQID");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn test_exists_stat() {
        let d = tdir("es");
        let p = d.join("exist.txt");
        let ps = p.to_string_lossy();
        assert_eq!(call(super::basic_ops::hap_fs_exists, &format!(r#"{{"path":"{ps}"}}"#)), json!(false));
        std::fs::write(&p, "x").unwrap();
        assert_eq!(call(super::basic_ops::hap_fs_exists, &format!(r#"{{"path":"{ps}"}}"#)), json!(true));
        let st = call(super::basic_ops::hap_fs_stat, &format!(r#"{{"path":"{ps}"}}"#));
        assert_eq!(st["is_file"], json!(true));
        assert_eq!(st["size"], json!(1));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn test_mkdir_remove() {
        let d = tdir("mr");
        let sub = d.join("a/b/c");
        let sp = sub.to_string_lossy();
        call(super::dir_ops::hap_fs_mkdir, &format!(r#"{{"path":"{sp}"}}"#));
        assert!(sub.is_dir());
        let ap = d.join("a").to_string_lossy().to_string();
        call(super::dir_ops::hap_fs_remove, &format!(r#"{{"path":"{ap}","recursive":true}}"#));
        assert!(!d.join("a").exists());
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn test_copy_move() {
        let d = tdir("cm");
        let a = d.join("a.txt");
        let b = d.join("b.txt");
        let c = d.join("c.txt");
        std::fs::write(&a, "data").unwrap();
        let ap = a.to_string_lossy();
        let bp = b.to_string_lossy();
        let cp = c.to_string_lossy();
        call(super::dir_ops::hap_fs_copy, &format!(r#"{{"source":"{ap}","dest":"{bp}"}}"#));
        assert!(b.exists());
        call(super::dir_ops::hap_fs_move, &format!(r#"{{"source":"{bp}","dest":"{cp}"}}"#));
        assert!(!b.exists());
        assert_eq!(std::fs::read_to_string(&c).unwrap(), "data");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn test_list_dir() {
        let d = tdir("ld");
        std::fs::write(d.join("f1"), "").unwrap();
        std::fs::write(d.join("f2"), "").unwrap();
        let dp = d.to_string_lossy();
        let r = call(super::dir_ops::hap_fs_list_dir, &format!(r#"{{"path":"{dp}"}}"#));
        assert!(r.as_array().unwrap().len() >= 2);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn test_path_utils() {
        let r = call(super::basic_ops::hap_fs_join_path, r#"{"parts":["a","b","c.txt"]}"#);
        assert!(r.as_str().unwrap().contains("b"));
        let r2 = call(super::basic_ops::hap_fs_extension, r#"{"path":"file.tar.gz"}"#);
        assert_eq!(r2.as_str().unwrap(), "gz");
        let r3 = call(super::basic_ops::hap_fs_file_name, r#"{"path":"/a/b/c.txt"}"#);
        assert_eq!(r3.as_str().unwrap(), "c.txt");
        let r4 = call(super::basic_ops::hap_fs_parent_path, r#"{"path":"/a/b/c"}"#);
        assert_eq!(r4.as_str().unwrap(), "/a/b");
    }

    #[test]
    fn test_checksum() {
        let d = tdir("ck");
        let p = d.join("ck.txt");
        std::fs::write(&p, "hello").unwrap();
        let ps = p.to_string_lossy();
        let r = call(super::advanced_ops::hap_fs_checksum, &format!(r#"{{"path":"{ps}","algorithm":"sha256"}}"#));
        assert_eq!(r.as_str().unwrap(), "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn test_line_count_read_lines() {
        let d = tdir("lc");
        let p = d.join("lines.txt");
        std::fs::write(&p, "a\nb\nc\nd\ne").unwrap();
        let ps = p.to_string_lossy();
        let r = call(super::basic_ops::hap_fs_line_count, &format!(r#"{{"path":"{ps}"}}"#));
        assert_eq!(r.as_i64().unwrap(), 5);
        let r2 = call(super::advanced_ops::hap_fs_read_text_lines, &format!(r#"{{"path":"{ps}","start_line":2,"count":2}}"#));
        assert_eq!(r2["lines"].as_array().unwrap().len(), 2);
        assert_eq!(r2["lines"][0], "b");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn test_write_atomic() {
        let d = tdir("wa");
        let p = d.join("atomic.txt");
        let ps = p.to_string_lossy();
        call(super::advanced_ops::hap_fs_write_atomic, &format!(r#"{{"path":"{ps}","content":"safe"}}"#));
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "safe");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn test_search_content() {
        let d = tdir("sc");
        std::fs::write(d.join("a.txt"), "hello world\nfoo bar").unwrap();
        std::fs::write(d.join("b.txt"), "no match here").unwrap();
        let dp = d.to_string_lossy();
        let r = call(super::advanced_ops::hap_fs_search_content, &format!(r#"{{"dir":"{dp}","pattern":"hello"}}"#));
        assert!(r.as_array().unwrap().len() >= 1);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn test_compare() {
        let d = tdir("cmp");
        std::fs::write(d.join("x"), "same").unwrap();
        std::fs::write(d.join("y"), "same").unwrap();
        std::fs::write(d.join("z"), "diff").unwrap();
        let xp = d.join("x").to_string_lossy().to_string();
        let yp = d.join("y").to_string_lossy().to_string();
        let zp = d.join("z").to_string_lossy().to_string();
        let r1 = call(super::advanced_ops::hap_fs_compare, &format!(r#"{{"path_a":"{xp}","path_b":"{yp}"}}"#));
        assert_eq!(r1["identical"], json!(true));
        let r2 = call(super::advanced_ops::hap_fs_compare, &format!(r#"{{"path_a":"{xp}","path_b":"{zp}"}}"#));
        assert_eq!(r2["identical"], json!(false));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn test_truncate() {
        let d = tdir("tr");
        let p = d.join("trunc.txt");
        std::fs::write(&p, "hello world").unwrap();
        let ps = p.to_string_lossy();
        call(super::advanced_ops::hap_fs_truncate, &format!(r#"{{"path":"{ps}","size":5}}"#));
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "hello");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn test_describe() {
        let ptr = super::hap_module_describe();
        let s = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        let v: Value = serde_json::from_str(s).unwrap();
        assert_eq!(v["name"], "fs");
        assert!(v["functions"].as_array().unwrap().len() >= 40);
        unsafe { super::hap_free_string(ptr as *mut _) };
    }

    #[test]
    fn test_watch_unwatch() {
        let d = tdir("watch");
        let dp = d.to_string_lossy().replace('\\', "\\\\");
        let r = call(super::advanced_ops::hap_fs_watch, &format!(r#"{{"path":"{dp}","recursive":true,"callback_id":"cb1"}}"#));
        let wid = r["watcher_id"].as_str().unwrap().to_string();
        assert!(wid.starts_with("watch-"));
        let list = call(super::advanced_ops::hap_fs_list_watchers, r#"{}"#);
        assert!(list.as_array().unwrap().len() >= 1);
        let ur = call(super::advanced_ops::hap_fs_unwatch, &format!(r#"{{"watcher_id":"{wid}"}}"#));
        assert_eq!(ur, json!(true));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn test_lock_unlock() {
        let d = tdir("lock");
        let fp = d.join("locktest.dat");
        std::fs::write(&fp, "lock data").unwrap();
        let fps = fp.to_string_lossy().replace('\\', "\\\\");
        let r = call(super::advanced_ops::hap_fs_lock_file, &format!(r#"{{"path":"{fps}","exclusive":true}}"#));
        let lid = r["lock_id"].as_str().unwrap().to_string();
        assert!(lid.starts_with("lock-"));
        let list = call(super::advanced_ops::hap_fs_list_locks, r#"{}"#);
        assert!(list.as_array().unwrap().len() >= 1);
        let ur = call(super::advanced_ops::hap_fs_unlock_file, &format!(r#"{{"lock_id":"{lid}"}}"#));
        assert_eq!(ur, json!(true));
        std::fs::remove_dir_all(&d).ok();
    }
}
