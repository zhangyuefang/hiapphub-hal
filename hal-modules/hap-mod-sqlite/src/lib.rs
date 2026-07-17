mod funcs;

use hap_common::{hap_free_string, hap_module_init};

hap_module_init!("sqlite");
hap_free_string!();

#[no_mangle]
pub extern "C" fn hap_module_describe() -> *const std::os::raw::c_char {
    hap_common::ffi::str_to_c(include_str!("../manifest.json"))
}

#[cfg(test)]
mod tests {
    use std::ffi::{CStr, CString};
    use serde_json::{json, Value};

    fn call(f: extern "C" fn(*const std::os::raw::c_char) -> *const std::os::raw::c_char, s: &str) -> Value {
        let cs = CString::new(s).unwrap();
        let r = f(cs.as_ptr());
        assert!(!r.is_null());
        let out = unsafe { CStr::from_ptr(r) }.to_str().unwrap().to_string();
        unsafe { super::hap_free_string(r as *mut _) };
        serde_json::from_str(&out).unwrap()
    }

    fn td(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("hap_sqlite_{}_{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn open_db(name: &str) -> (std::path::PathBuf, String) {
        let d = td(name);
        let p = d.join("test.db").to_string_lossy().to_string();
        let r = call(super::funcs::hap_sqlite_open, &format!(r#"{{"path":"{p}"}}"#));
        let db_id = r["db_id"].as_str().unwrap().to_string();
        (d, db_id)
    }

    #[test]
    fn test_open_close() {
        let (d, db_id) = open_db("oc");
        assert_eq!(call(super::funcs::hap_sqlite_is_open, &format!(r#"{{"db_id":"{db_id}"}}"#)), json!(true));
        call(super::funcs::hap_sqlite_close, &format!(r#"{{"db_id":"{db_id}"}}"#));
        assert_eq!(call(super::funcs::hap_sqlite_is_open, &format!(r#"{{"db_id":"{db_id}"}}"#)), json!(false));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn test_execute_query() {
        let (d, db_id) = open_db("eq");
        call(super::funcs::hap_sqlite_execute, &format!(r#"{{"db_id":"{db_id}","sql":"CREATE TABLE t(id INTEGER PRIMARY KEY, name TEXT)"}}"#));
        let r = call(super::funcs::hap_sqlite_execute, &format!(r#"{{"db_id":"{db_id}","sql":"INSERT INTO t(name) VALUES(?1)","params":["Alice"]}}"#));
        assert_eq!(r["changes"], json!(1));
        assert_eq!(r["last_insert_rowid"], json!(1));
        let q = call(super::funcs::hap_sqlite_query, &format!(r#"{{"db_id":"{db_id}","sql":"SELECT * FROM t"}}"#));
        assert_eq!(q["columns"].as_array().unwrap().len(), 2);
        assert_eq!(q["rows"].as_array().unwrap().len(), 1);
        call(super::funcs::hap_sqlite_close, &format!(r#"{{"db_id":"{db_id}"}}"#));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn test_query_one_objects() {
        let (d, db_id) = open_db("qo");
        call(super::funcs::hap_sqlite_execute, &format!(r#"{{"db_id":"{db_id}","sql":"CREATE TABLE t(id INT, val TEXT)"}}"#));
        call(super::funcs::hap_sqlite_execute, &format!(r#"{{"db_id":"{db_id}","sql":"INSERT INTO t VALUES(1,'a'),(2,'b')"}}"#));
        let one = call(super::funcs::hap_sqlite_query_one, &format!(r#"{{"db_id":"{db_id}","sql":"SELECT * FROM t WHERE id=1"}}"#));
        assert_eq!(one["val"], "a");
        let none = call(super::funcs::hap_sqlite_query_one, &format!(r#"{{"db_id":"{db_id}","sql":"SELECT * FROM t WHERE id=99"}}"#));
        assert!(none.is_null());
        let objs = call(super::funcs::hap_sqlite_query_objects, &format!(r#"{{"db_id":"{db_id}","sql":"SELECT * FROM t ORDER BY id"}}"#));
        assert_eq!(objs.as_array().unwrap().len(), 2);
        assert_eq!(objs[1]["val"], "b");
        call(super::funcs::hap_sqlite_close, &format!(r#"{{"db_id":"{db_id}"}}"#));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn test_batch_execute() {
        let (d, db_id) = open_db("be");
        call(super::funcs::hap_sqlite_execute, &format!(r#"{{"db_id":"{db_id}","sql":"CREATE TABLE t(v TEXT)"}}"#));
        let r = call(super::funcs::hap_sqlite_batch_execute, &format!(
            r#"{{"db_id":"{db_id}","statements":[{{"sql":"INSERT INTO t VALUES('a')"}},{{"sql":"INSERT INTO t VALUES('b')"}}]}}"#
        ));
        assert_eq!(r["total_changes"], json!(2));
        call(super::funcs::hap_sqlite_close, &format!(r#"{{"db_id":"{db_id}"}}"#));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn test_transaction() {
        let (d, db_id) = open_db("tx");
        call(super::funcs::hap_sqlite_execute, &format!(r#"{{"db_id":"{db_id}","sql":"CREATE TABLE t(v INT)"}}"#));
        call(super::funcs::hap_sqlite_begin, &format!(r#"{{"db_id":"{db_id}"}}"#));
        call(super::funcs::hap_sqlite_execute, &format!(r#"{{"db_id":"{db_id}","sql":"INSERT INTO t VALUES(1)"}}"#));
        call(super::funcs::hap_sqlite_rollback, &format!(r#"{{"db_id":"{db_id}"}}"#));
        let cnt = call(super::funcs::hap_sqlite_count, &format!(r#"{{"db_id":"{db_id}","table_name":"t"}}"#));
        assert_eq!(cnt.as_i64().unwrap(), 0);
        call(super::funcs::hap_sqlite_close, &format!(r#"{{"db_id":"{db_id}"}}"#));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn test_table_list_info() {
        let (d, db_id) = open_db("tli");
        call(super::funcs::hap_sqlite_execute, &format!(r#"{{"db_id":"{db_id}","sql":"CREATE TABLE users(id INT PRIMARY KEY, name TEXT NOT NULL)"}}"#));
        let tables = call(super::funcs::hap_sqlite_table_list, &format!(r#"{{"db_id":"{db_id}"}}"#));
        assert!(tables.as_array().unwrap().contains(&json!("users")));
        let info = call(super::funcs::hap_sqlite_table_info, &format!(r#"{{"db_id":"{db_id}","table_name":"users"}}"#));
        assert_eq!(info.as_array().unwrap().len(), 2);
        call(super::funcs::hap_sqlite_close, &format!(r#"{{"db_id":"{db_id}"}}"#));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn test_backup() {
        let (d, db_id) = open_db("bk");
        call(super::funcs::hap_sqlite_execute, &format!(r#"{{"db_id":"{db_id}","sql":"CREATE TABLE t(v TEXT)"}}"#));
        call(super::funcs::hap_sqlite_execute, &format!(r#"{{"db_id":"{db_id}","sql":"INSERT INTO t VALUES('data')"}}"#));
        let bp = d.join("backup.db").to_string_lossy().to_string();
        let r = call(super::funcs::hap_sqlite_backup, &format!(r#"{{"db_id":"{db_id}","dest_path":"{bp}"}}"#));
        assert!(r["size"].as_i64().unwrap() > 0);
        call(super::funcs::hap_sqlite_close, &format!(r#"{{"db_id":"{db_id}"}}"#));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn test_export_import_csv() {
        let (d, db_id) = open_db("csv");
        call(super::funcs::hap_sqlite_execute, &format!(r#"{{"db_id":"{db_id}","sql":"CREATE TABLE t(a TEXT, b TEXT)"}}"#));
        call(super::funcs::hap_sqlite_execute, &format!(r#"{{"db_id":"{db_id}","sql":"INSERT INTO t VALUES('1','x'),('2','y')"}}"#));
        let csv_path = d.join("out.csv").to_string_lossy().to_string();
        let r = call(super::funcs::hap_sqlite_export_csv, &format!(r#"{{"db_id":"{db_id}","table_or_query":"t","output_path":"{csv_path}"}}"#));
        assert_eq!(r["rows"], json!(2));
        call(super::funcs::hap_sqlite_execute, &format!(r#"{{"db_id":"{db_id}","sql":"CREATE TABLE t2(a TEXT, b TEXT)"}}"#));
        let r2 = call(super::funcs::hap_sqlite_import_csv, &format!(r#"{{"db_id":"{db_id}","table_name":"t2","csv_path":"{csv_path}"}}"#));
        assert_eq!(r2["imported"], json!(2));
        call(super::funcs::hap_sqlite_close, &format!(r#"{{"db_id":"{db_id}"}}"#));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn test_describe() {
        let ptr = super::hap_module_describe();
        let s = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        let v: Value = serde_json::from_str(s).unwrap();
        assert_eq!(v["name"], "sqlite");
        assert_eq!(v["functions"].as_array().unwrap().len(), 30);
        unsafe { super::hap_free_string(ptr as *mut _) };
    }

    #[test]
    fn test_invalid_db_id() {
        let r = call(super::funcs::hap_sqlite_execute, r#"{"db_id":"nonexistent","sql":"SELECT 1"}"#);
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_pragma_and_vacuum() {
        let (d, db_id) = open_db("pv");
        let r = call(super::funcs::hap_sqlite_pragma, &format!(r#"{{"db_id":"{db_id}","name":"journal_mode"}}"#));
        assert!(r.as_str().is_some());
        let r = call(super::funcs::hap_sqlite_vacuum, &format!(r#"{{"db_id":"{db_id}"}}"#));
        assert_eq!(r, true);
        call(super::funcs::hap_sqlite_close, &format!(r#"{{"db_id":"{db_id}"}}"#));
        std::fs::remove_dir_all(&d).ok();
    }
}
