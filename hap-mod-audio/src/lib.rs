pub mod funcs;
use hap_common::ffi::str_to_c;
use std::ffi::c_char;
hap_common::hap_module_init!("audio");
hap_common::hap_free_string!();

#[no_mangle]
pub extern "C" fn hap_module_describe() -> *const c_char {
    str_to_c(include_str!("../manifest.json"))
}


#[cfg(test)]
mod tests {
    use hap_common::ffi::free_c_string;
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

    #[test]
    fn test_describe() {
        let ptr = super::hap_module_describe();
        let s = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        let v: serde_json::Value = serde_json::from_str(s).unwrap();
        assert_eq!(v["name"], "audio");
        assert_eq!(v["functions"].as_array().unwrap().len(), 40);
        unsafe { free_c_string(ptr as *mut _); }
    }

    fn create_test_wav(path: &str) {
        let spec = hound::WavSpec { channels: 1, sample_rate: 44100, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        for i in 0..44100 {
            let t = i as f32 / 44100.0;
            let sample = (t * 440.0 * 2.0 * std::f32::consts::PI).sin();
            writer.write_sample((sample * 32767.0) as i16).unwrap();
        }
        writer.finalize().unwrap();
    }

    #[test]
    fn test_wav_trim() {
        let tmp = std::env::temp_dir().join("hap_audio_test_trim.wav");
        let out = std::env::temp_dir().join("hap_audio_test_trimmed.wav");
        create_test_wav(tmp.to_str().unwrap());
        let r = call(super::funcs::hap_audio_trim, &format!(
            r#"{{"input_path":"{}","output_path":"{}","start_ms":0,"end_ms":500}}"#,
            tmp.to_string_lossy(), out.to_string_lossy()
        ));
        assert!(r["duration_ms"].as_f64().unwrap() > 0.0);
        assert!(r["size"].as_i64().unwrap() > 0);
        std::fs::remove_file(&tmp).ok();
        std::fs::remove_file(&out).ok();
    }

    #[test]
    fn test_wav_normalize() {
        let tmp = std::env::temp_dir().join("hap_audio_test_norm.wav");
        let out = std::env::temp_dir().join("hap_audio_test_normed.wav");
        create_test_wav(tmp.to_str().unwrap());
        let r = call(super::funcs::hap_audio_normalize, &format!(
            r#"{{"input_path":"{}","output_path":"{}"}}"#,
            tmp.to_string_lossy(), out.to_string_lossy()
        ));
        assert_eq!(r, json!(true));
        assert!(out.exists());
        std::fs::remove_file(&tmp).ok();
        std::fs::remove_file(&out).ok();
    }

    #[test]
    fn test_wav_waveform() {
        let tmp = std::env::temp_dir().join("hap_audio_test_wf.wav");
        create_test_wav(tmp.to_str().unwrap());
        let r = call(super::funcs::hap_audio_get_waveform, &format!(
            r#"{{"path":"{}","samples":50}}"#, tmp.to_string_lossy()
        ));
        let peaks = r["peaks"].as_array().unwrap();
        assert!(peaks.len() > 0);
        assert!(r["duration_ms"].as_f64().unwrap() > 900.0);
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_wav_concat() {
        let tmp1 = std::env::temp_dir().join("hap_audio_concat1.wav");
        let tmp2 = std::env::temp_dir().join("hap_audio_concat2.wav");
        let out = std::env::temp_dir().join("hap_audio_concated.wav");
        create_test_wav(tmp1.to_str().unwrap());
        create_test_wav(tmp2.to_str().unwrap());
        let r = call(super::funcs::hap_audio_concat, &format!(
            r#"{{"input_paths":["{}","{}"],"output_path":"{}"}}"#,
            tmp1.to_string_lossy(), tmp2.to_string_lossy(), out.to_string_lossy()
        ));
        assert!(r["duration_ms"].as_f64().unwrap() > 1800.0);
        std::fs::remove_file(&tmp1).ok();
        std::fs::remove_file(&tmp2).ok();
        std::fs::remove_file(&out).ok();
    }

    #[test]
    fn test_wav_split() {
        let tmp = std::env::temp_dir().join("hap_audio_split.wav");
        let outdir = std::env::temp_dir().join("hap_audio_split_out");
        create_test_wav(tmp.to_str().unwrap());
        let r = call(super::funcs::hap_audio_split, &format!(
            r#"{{"input_path":"{}","output_dir":"{}","positions_ms":[500]}}"#,
            tmp.to_string_lossy(), outdir.to_string_lossy()
        ));
        let files = r["files"].as_array().unwrap();
        assert_eq!(files.len(), 2);
        std::fs::remove_file(&tmp).ok();
        std::fs::remove_dir_all(&outdir).ok();
    }

    #[test]
    fn test_system_volume() {
        let r = call(super::funcs::hap_audio_get_system_volume, r#"{}"#);
        let vol = r.as_f64().unwrap();
        assert!(vol >= 0.0 && vol <= 1.0);
    }

    #[test]
    fn test_is_muted() {
        let r = call(super::funcs::hap_audio_is_muted, r#"{}"#);
        assert!(r.is_boolean());
    }

    #[test]
    fn test_on_off_device_change() {
        let r = call(super::funcs::hap_audio_on_device_change, r#"{"callback_id":"cb1"}"#);
        let wid = r["watcher_id"].as_str().unwrap().to_string();
        assert!(wid.starts_with("adw_"));
        std::thread::sleep(std::time::Duration::from_millis(100));
        let r2 = call(super::funcs::hap_audio_off_device_change, &format!(r#"{{"watcher_id":"{}"}}"#, wid));
        assert_eq!(r2, serde_json::json!(true));
    }
}
