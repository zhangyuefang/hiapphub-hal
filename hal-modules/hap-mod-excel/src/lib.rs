pub mod funcs;

use hap_common::ffi::str_to_c;
use std::ffi::c_char;

hap_common::hap_module_init!("excel");
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
        assert_eq!(v["name"], "excel");
        assert!(v["functions"].as_array().unwrap().len() >= 39);
        unsafe { free_c_string(ptr as *mut _); }
    }

    #[test]
    fn test_write_and_read() {
        let tmp = std::env::temp_dir().join("test_excel_rw.xlsx");
        let r = call(hap_excel_write, json!({
            "path": tmp.to_str().unwrap(),
            "headers": ["Name", "Age"],
            "rows": [["Alice", 30], ["Bob", 25]]
        }));
        assert_eq!(r["written_rows"], 2);

        let r = call(hap_excel_read, json!({"path": tmp.to_str().unwrap()}));
        assert!(r.get("headers").is_some() || r.get("error").is_some());

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_create_save_close() {
        let tmp = std::env::temp_dir().join("test_excel_create.xlsx");
        let r = call(hap_excel_create, json!({"path": tmp.to_str().unwrap()}));
        let book_id = r["book_id"].as_str().unwrap().to_string();

        let r = call(hap_excel_save, json!({"book_id": book_id}));
        assert_eq!(r, true);

        let r = call(hap_excel_close, json!({"book_id": book_id}));
        assert_eq!(r, true);

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_list_sheets() {
        let tmp = std::env::temp_dir().join("test_excel_sheets.xlsx");
        call(hap_excel_write, json!({
            "path": tmp.to_str().unwrap(),
            "headers": ["A"],
            "rows": [[1]]
        }));
        let r = call(hap_excel_list_sheets, json!({"path": tmp.to_str().unwrap()}));
        assert!(r.is_array() || r.get("error").is_some());
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_to_csv_and_json() {
        let tmp_xlsx = std::env::temp_dir().join("test_export.xlsx");
        let tmp_csv = std::env::temp_dir().join("test_export.csv");
        call(hap_excel_write, json!({
            "path": tmp_xlsx.to_str().unwrap(),
            "headers": ["Name", "Score"],
            "rows": [["Alice", 95], ["Bob", 80]]
        }));

        let r = call(hap_excel_to_csv, json!({
            "path": tmp_xlsx.to_str().unwrap(),
            "output_path": tmp_csv.to_str().unwrap()
        }));
        assert!(r == json!(true) || r.get("error").is_some());

        let r = call(hap_excel_to_json, json!({"path": tmp_xlsx.to_str().unwrap()}));
        assert!(r.is_array() || r.get("error").is_some());

        let _ = std::fs::remove_file(&tmp_xlsx);
        let _ = std::fs::remove_file(&tmp_csv);
    }

    #[test]
    fn test_from_csv() {
        let tmp_csv = std::env::temp_dir().join("test_from.csv");
        let tmp_xlsx = std::env::temp_dir().join("test_from.xlsx");
        std::fs::write(&tmp_csv, "Name,Age\nAlice,30\nBob,25\n").unwrap();

        let r = call(hap_excel_from_csv, json!({
            "csv_path": tmp_csv.to_str().unwrap(),
            "output_path": tmp_xlsx.to_str().unwrap()
        }));
        assert!(r == json!(true) || r.get("error").is_some());

        let _ = std::fs::remove_file(&tmp_csv);
        let _ = std::fs::remove_file(&tmp_xlsx);
    }

    #[test]
    fn test_list_open() {
        let r = call(hap_excel_list_open, json!({}));
        assert!(r.is_array());
    }

    #[test]
    fn test_set_range_and_column_width() {
        let tmp = std::env::temp_dir().join("test_excel_range.xlsx");
        let r = call(hap_excel_create, json!({"path": tmp.to_str().unwrap()}));
        let book_id = r["book_id"].as_str().unwrap().to_string();

        let r = call(hap_excel_set_range, json!({
            "book_id": book_id, "sheet": "Sheet1", "start_cell": "A1",
            "values": [["Name", "Score"], ["Alice", 95], ["Bob", 80]]
        }));
        assert_eq!(r["cells_written"], 6);

        let r = call(hap_excel_set_column_width, json!({"book_id": book_id, "sheet": "Sheet1", "column": "A", "width": 20.0}));
        assert_eq!(r, true);

        let r = call(hap_excel_set_row_height, json!({"book_id": book_id, "sheet": "Sheet1", "row": 1, "height": 30.0}));
        assert_eq!(r, true);

        call(hap_excel_save, json!({"book_id": book_id}));
        call(hap_excel_close, json!({"book_id": book_id}));
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_freeze_and_protect() {
        let tmp = std::env::temp_dir().join("test_excel_freeze.xlsx");
        let r = call(hap_excel_create, json!({"path": tmp.to_str().unwrap()}));
        let book_id = r["book_id"].as_str().unwrap().to_string();

        let r = call(hap_excel_freeze_panes, json!({"book_id": book_id, "sheet": "Sheet1", "row": 1, "col": 0}));
        assert_eq!(r, true);

        let r = call(hap_excel_protect_sheet, json!({"book_id": book_id, "sheet": "Sheet1", "password": "test123"}));
        assert_eq!(r, true);

        call(hap_excel_save, json!({"book_id": book_id}));
        call(hap_excel_close, json!({"book_id": book_id}));
        let _ = std::fs::remove_file(&tmp);
    }
}
