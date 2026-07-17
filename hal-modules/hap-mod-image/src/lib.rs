pub mod basic;
pub mod filters;
pub mod draw;

use hap_common::ffi::str_to_c;
use std::ffi::c_char;

hap_common::hap_module_init!("image");
hap_common::hap_free_string!();

#[no_mangle]
pub extern "C" fn hap_module_describe() -> *const c_char {
    str_to_c(include_str!("../manifest.json"))
}

#[cfg(test)]
mod tests {
    use super::basic::*;
    use super::filters::*;
    use super::draw::*;
    use hap_common::ffi::{str_to_c, free_c_string};
    use std::ffi::CStr;
    use serde_json::json;

    fn td(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("hap_image_{}_{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn call(func: extern "C" fn(*const std::ffi::c_char) -> *const std::ffi::c_char, params: serde_json::Value) -> serde_json::Value {
        let input = str_to_c(&params.to_string());
        let result = func(input);
        unsafe { free_c_string(input as *mut _); }
        let s = unsafe { CStr::from_ptr(result) }.to_str().unwrap().to_string();
        unsafe { free_c_string(result as *mut _); }
        serde_json::from_str(&s).unwrap()
    }

    fn create_test_image(dir: &std::path::Path) -> String {
        let path = dir.join("test.png");
        let img = image::RgbaImage::from_pixel(100, 100, image::Rgba([255, 0, 0, 255]));
        img.save(&path).unwrap();
        path.to_string_lossy().to_string()
    }

    #[test]
    fn test_describe() {
        let ptr = super::hap_module_describe();
        let s = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        let v: serde_json::Value = serde_json::from_str(s).unwrap();
        assert_eq!(v["name"], "image");
        assert!(v["functions"].as_array().unwrap().len() >= 41);
        unsafe { free_c_string(ptr as *mut _); }
    }

    #[test]
    fn test_info() {
        let d = td("info");
        let src = create_test_image(&d);
        let r = call(hap_image_info, json!({"path": src}));
        assert_eq!(r["width"], 100);
        assert_eq!(r["height"], 100);
    }

    #[test]
    fn test_resize() {
        let d = td("resize");
        let src = create_test_image(&d);
        let out = d.join("resized.png").to_string_lossy().to_string();
        let r = call(hap_image_resize, json!({"path": src, "width": 50, "height": 50, "output": out}));
        assert_eq!(r, json!(true));
    }

    #[test]
    fn test_crop() {
        let d = td("crop");
        let src = create_test_image(&d);
        let out = d.join("cropped.png").to_string_lossy().to_string();
        let r = call(hap_image_crop, json!({"path": src, "x": 10, "y": 10, "w": 50, "h": 50, "output": out}));
        assert_eq!(r, json!(true));
    }

    #[test]
    fn test_blur_sharpen_grayscale() {
        let d = td("filters");
        let src = create_test_image(&d);
        let out1 = d.join("blur.png").to_string_lossy().to_string();
        let out2 = d.join("sharp.png").to_string_lossy().to_string();
        let out3 = d.join("gray.png").to_string_lossy().to_string();
        assert_eq!(call(hap_image_blur, json!({"path": &src, "radius": 2.0, "output": out1})), json!(true));
        assert_eq!(call(hap_image_sharpen, json!({"path": &src, "amount": 1.0, "output": out2})), json!(true));
        assert_eq!(call(hap_image_grayscale, json!({"path": &src, "output": out3})), json!(true));
    }

    #[test]
    fn test_create_blank_and_get_pixel() {
        let d = td("blank");
        let out = d.join("blank.png").to_string_lossy().to_string();
        call(hap_image_create_blank, json!({"width": 10, "height": 10, "color": "#FF0000", "output": &out}));
        let r = call(hap_image_get_pixel, json!({"path": &out, "x": 5, "y": 5}));
        assert_eq!(r["r"], 255);
        assert_eq!(r["g"], 0);
    }

    #[test]
    fn test_to_from_base64() {
        let d = td("b64");
        let src = create_test_image(&d);
        let r = call(hap_image_to_base64, json!({"path": &src}));
        assert!(r.as_str().unwrap().len() > 10);
        let out = d.join("from_b64.png").to_string_lossy().to_string();
        let _ = call(hap_image_from_base64, json!({"data": r.as_str().unwrap(), "output": out}));
    }

    #[test]
    fn test_histogram() {
        let d = td("hist");
        let src = create_test_image(&d);
        let r = call(hap_image_histogram, json!({"path": &src}));
        assert!(r["r"].as_array().unwrap().len() == 256);
    }

    #[test]
    fn test_draw_rect() {
        let d = td("draw");
        let src = create_test_image(&d);
        let out = d.join("rect.png").to_string_lossy().to_string();
        let r = call(hap_image_draw_rect, json!({"path": &src, "output": &out, "x": 10, "y": 10, "w": 30, "h": 30, "color": "#00FF00"}));
        assert_eq!(r, json!(true));
    }

    #[test]
    fn test_invert_sepia() {
        let d = td("inv_sep");
        let src = create_test_image(&d);
        let out1 = d.join("inv.png").to_string_lossy().to_string();
        let out2 = d.join("sepia.png").to_string_lossy().to_string();
        assert_eq!(call(hap_image_invert, json!({"path": &src, "output": &out1})), json!(true));
        assert_eq!(call(hap_image_sepia, json!({"path": &src, "output": &out2})), json!(true));
    }

    #[test]
    fn test_compare() {
        let d = td("compare");
        let src = create_test_image(&d);
        let r = call(hap_image_compare, json!({"path_a": &src, "path_b": &src}));
        assert_eq!(r["identical"], true);
        assert_eq!(r["similarity"], 1.0);
    }
}
