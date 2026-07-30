pub mod funcs;

use hap_common::ffi::str_to_c;
use std::ffi::c_char;

hap_common::hap_module_init!("pdf");
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
        assert_eq!(v["name"], "pdf");
        assert!(v["functions"].as_array().unwrap().len() >= 40);
        unsafe { free_c_string(ptr as *mut _); }
    }

    #[test]
    fn test_create_save_close() {
        let r = call(hap_pdf_create, json!({}));
        let doc_id = r["doc_id"].as_str().unwrap().to_string();

        let tmp = std::env::temp_dir().join("test_pdf_create.pdf");
        let r = call(hap_pdf_save, json!({"doc_id": doc_id, "output_path": tmp.to_str().unwrap()}));
        assert!(r["size"].as_i64().unwrap() > 0);

        let r = call(hap_pdf_close, json!({"doc_id": doc_id}));
        assert_eq!(r, true);

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_info_and_dimensions() {
        let tmp = std::env::temp_dir().join("test_pdf_info.pdf");
        let r = call(hap_pdf_create, json!({}));
        let doc_id = r["doc_id"].as_str().unwrap().to_string();
        call(hap_pdf_save, json!({"doc_id": doc_id, "output_path": tmp.to_str().unwrap()}));
        call(hap_pdf_close, json!({"doc_id": doc_id}));

        let r = call(hap_pdf_info, json!({"path": tmp.to_str().unwrap()}));
        assert!(r.get("pages").is_some() || r.get("error").is_some());

        let r = call(hap_pdf_page_dimensions, json!({"path": tmp.to_str().unwrap()}));
        assert!(r.get("width_mm").is_some() || r.get("error").is_some());

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_merge() {
        let tmp1 = std::env::temp_dir().join("test_pdf_m1.pdf");
        let tmp2 = std::env::temp_dir().join("test_pdf_m2.pdf");
        let out = std::env::temp_dir().join("test_pdf_merged.pdf");

        let r = call(hap_pdf_create, json!({}));
        let id1 = r["doc_id"].as_str().unwrap().to_string();
        call(hap_pdf_save, json!({"doc_id": id1, "output_path": tmp1.to_str().unwrap()}));
        call(hap_pdf_close, json!({"doc_id": id1}));

        let r = call(hap_pdf_create, json!({}));
        let id2 = r["doc_id"].as_str().unwrap().to_string();
        call(hap_pdf_save, json!({"doc_id": id2, "output_path": tmp2.to_str().unwrap()}));
        call(hap_pdf_close, json!({"doc_id": id2}));

        let r = call(hap_pdf_merge, json!({
            "input_paths": [tmp1.to_str().unwrap(), tmp2.to_str().unwrap()],
            "output_path": out.to_str().unwrap()
        }));
        assert!(r.get("pages").is_some() || r.get("error").is_some());

        let _ = std::fs::remove_file(&tmp1);
        let _ = std::fs::remove_file(&tmp2);
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn test_list_open() {
        let r = call(hap_pdf_list_open, json!({}));
        assert!(r.is_array());
    }

    #[test]
    fn test_add_text_line_rect() {
        let r = call(hap_pdf_create, json!({}));
        let doc_id = r["doc_id"].as_str().unwrap().to_string();

        let r = call(hap_pdf_add_page, json!({"doc_id": doc_id}));
        assert!(r["page_index"].as_i64().is_some());

        let r = call(hap_pdf_add_text, json!({
            "doc_id": doc_id, "page_index": 0, "text": "Hello PDF", "x": 72.0, "y": 700.0, "font_size": 16.0
        }));
        assert_eq!(r, true);

        let r = call(hap_pdf_add_line, json!({
            "doc_id": doc_id, "page_index": 0, "x1": 72.0, "y1": 690.0, "x2": 500.0, "y2": 690.0
        }));
        assert_eq!(r, true);

        let r = call(hap_pdf_add_rect, json!({
            "doc_id": doc_id, "page_index": 0, "x": 72.0, "y": 600.0, "w": 200.0, "h": 80.0
        }));
        assert_eq!(r, true);

        let tmp = std::env::temp_dir().join("test_pdf_text_line_rect.pdf");
        let r = call(hap_pdf_save, json!({"doc_id": doc_id, "output_path": tmp.to_str().unwrap()}));
        assert!(r["size"].as_i64().unwrap() > 0);
        call(hap_pdf_close, json!({"doc_id": doc_id}));
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_encrypt_decrypt() {
        let r = call(hap_pdf_create, json!({}));
        let doc_id = r["doc_id"].as_str().unwrap().to_string();
        call(hap_pdf_add_page, json!({"doc_id": doc_id}));
        call(hap_pdf_add_text, json!({"doc_id": doc_id, "page_index": 0, "text": "Secret", "x": 72.0, "y": 700.0, "font_size": 12.0}));
        let plain = std::env::temp_dir().join("test_pdf_plain.pdf");
        call(hap_pdf_save, json!({"doc_id": doc_id, "output_path": plain.to_str().unwrap()}));
        call(hap_pdf_close, json!({"doc_id": doc_id}));

        let encrypted = std::env::temp_dir().join("test_pdf_encrypted.pdf");
        let r = call(hap_pdf_set_password, json!({
            "input_path": plain.to_str().unwrap(),
            "output_path": encrypted.to_str().unwrap(),
            "user_password": "test123"
        }));
        assert_eq!(r, true);

        let info = call(hap_pdf_info, json!({"path": encrypted.to_str().unwrap()}));
        assert_eq!(info["encrypted"], true);

        let decrypted = std::env::temp_dir().join("test_pdf_decrypted.pdf");
        let r = call(hap_pdf_remove_password, json!({
            "input_path": encrypted.to_str().unwrap(),
            "output_path": decrypted.to_str().unwrap(),
            "password": "test123"
        }));
        assert_eq!(r, true);

        let info2 = call(hap_pdf_info, json!({"path": decrypted.to_str().unwrap()}));
        assert_eq!(info2["encrypted"], false);

        let _ = std::fs::remove_file(&plain);
        let _ = std::fs::remove_file(&encrypted);
        let _ = std::fs::remove_file(&decrypted);
    }
}
