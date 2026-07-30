use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::{json, Value};
use std::process::Command;

fn run_native_ocr(image_path: &str, languages: &[String]) -> Result<Value, HapError> {
    if cfg!(target_os = "macos") {
        run_macos_ocr(image_path, languages)
    } else if cfg!(target_os = "windows") {
        run_windows_ocr(image_path, languages)
    } else {
        run_tesseract_ocr(image_path, languages)
    }
}

fn run_macos_ocr(image_path: &str, languages: &[String]) -> Result<Value, HapError> {
    let lang_arg = if languages.is_empty() {
        "en-US".to_string()
    } else {
        languages.join(",")
    };

    let script = format!(
        r#"
        import Vision
        import Foundation
        import CoreGraphics
        let url = URL(fileURLWithPath: "{}")
        guard let source = CGImageSourceCreateWithURL(url as CFURL, nil),
              let img = CGImageSourceCreateImageAtIndex(source, 0, nil) else {{ exit(1) }}
        let req = VNRecognizeTextRequest()
        req.recognitionLanguages = ["{}"]
        let handler = VNImageRequestHandler(cgImage: img)
        try handler.perform([req])
        let results = req.results ?? []
        for obs in results {{
            print(obs.topCandidates(1).first?.string ?? "")
        }}
        "#,
        image_path.replace('"', r#"\""#),
        lang_arg.replace('"', r#"\""#)
    );

    let output = Command::new("swift")
        .args(["-e", &script])
        .output()
        .map_err(|e| HapError::internal(format!("swift exec failed: {e}")))?;

    let text = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(json!({ "text": text.trim(), "confidence": 0.9, "engine": "macos_vision" }))
}

fn run_windows_ocr(_image_path: &str, _languages: &[String]) -> Result<Value, HapError> {
    Ok(json!({ "text": "", "confidence": 0.0, "engine": "windows_ocr", "error": "not implemented on this platform" }))
}

fn run_tesseract_ocr(image_path: &str, languages: &[String]) -> Result<Value, HapError> {
    let lang = if languages.is_empty() { "eng".to_string() } else {
        languages.iter().map(|l| match l.as_str() {
            "zh-CN" | "zh" => "chi_sim",
            "zh-TW" => "chi_tra",
            "ja" => "jpn",
            "ko" => "kor",
            _ => "eng",
        }).collect::<Vec<_>>().join("+")
    };

    let output = Command::new("tesseract")
        .args([image_path, "stdout", "-l", &lang])
        .output()
        .map_err(|e| HapError::internal(format!("tesseract not found: {e}")))?;

    if !output.status.success() {
        return Err(HapError::internal(format!("tesseract failed: {}", String::from_utf8_lossy(&output.stderr))));
    }

    let text = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(json!({ "text": text.trim(), "confidence": 0.85, "engine": "tesseract" }))
}

#[derive(Deserialize)]
struct RecognizeParams {
    image_path: String,
    languages: Option<Vec<String>>,
}

hap_fn!(hap_ocr_recognize, RecognizeParams, |params| {
    let langs = params.languages.unwrap_or_default();
    if !std::path::Path::new(&params.image_path).exists() {
        return Err(HapError::invalid_param("image_path does not exist"));
    }
    run_native_ocr(&params.image_path, &langs)
});

#[derive(Deserialize)]
struct RecognizeRegionParams {
    image_path: String,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    languages: Option<Vec<String>>,
}

hap_fn!(hap_ocr_recognize_region, RecognizeRegionParams, |params| {
    if !std::path::Path::new(&params.image_path).exists() {
        return Err(HapError::invalid_param("image_path does not exist"));
    }
    let langs = params.languages.unwrap_or_default();

    let img = image::open(&params.image_path)
        .map_err(|e| HapError::internal(format!("open image: {e}")))?;
    let cropped = img.crop_imm(params.x as u32, params.y as u32, params.width as u32, params.height as u32);

    let tmp = std::env::temp_dir().join(format!("hap_ocr_region_{}.png", std::process::id()));
    cropped.save(&tmp).map_err(|e| HapError::internal(format!("save crop: {e}")))?;

    let result = run_native_ocr(tmp.to_str().unwrap_or(""), &langs);
    let _ = std::fs::remove_file(&tmp);
    result
});

#[derive(Deserialize)]
struct RecognizeBase64Params {
    base64_data: String,
    languages: Option<Vec<String>>,
}

hap_fn!(hap_ocr_recognize_base64, RecognizeBase64Params, |params| {
    use base64::Engine;
    let data = base64::engine::general_purpose::STANDARD.decode(&params.base64_data)
        .map_err(|e| HapError::invalid_param(format!("invalid base64: {e}")))?;

    let tmp = std::env::temp_dir().join(format!("hap_ocr_b64_{}.png", std::process::id()));
    std::fs::write(&tmp, &data).map_err(|e| HapError::internal(format!("write temp: {e}")))?;

    let langs = params.languages.unwrap_or_default();
    let result = run_native_ocr(tmp.to_str().unwrap_or(""), &langs);
    let _ = std::fs::remove_file(&tmp);
    result
});

#[derive(Deserialize)]
struct EmptyParams {}

hap_fn!(hap_ocr_get_supported_languages, EmptyParams, |_params| {
    if cfg!(target_os = "macos") {
        Ok(json!(["en-US", "zh-CN", "zh-TW", "ja", "ko", "fr", "de", "es", "it", "pt"]))
    } else {
        let output = Command::new("tesseract").args(["--list-langs"]).output();
        match output {
            Ok(o) => {
                let text = String::from_utf8_lossy(&o.stdout);
                let langs: Vec<&str> = text.lines().skip(1).collect();
                Ok(json!(langs))
            }
            Err(_) => Ok(json!(["eng"]))
        }
    }
});

#[derive(Deserialize)]
struct ScreenRegionParams {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    languages: Option<Vec<String>>,
}

hap_fn!(hap_ocr_recognize_screen_region, ScreenRegionParams, |params| {
    let tmp = std::env::temp_dir().join(format!("hap_ocr_screen_{}.png", std::process::id()));
    let tmp_path = tmp.to_str().unwrap_or("/tmp/hap_ocr_screen.png");

    if cfg!(target_os = "macos") {
        let region = format!("-R{},{},{},{}", params.x, params.y, params.width, params.height);
        let status = Command::new("screencapture")
            .args(["-x", &region, tmp_path])
            .status()
            .map_err(|e| HapError::internal(format!("screencapture failed: {e}")))?;
        if !status.success() {
            return Err(HapError::internal("screencapture failed"));
        }
    } else {
        return Err(HapError::internal("screen capture not supported on this platform"));
    }

    let langs = params.languages.unwrap_or_default();
    let result = run_native_ocr(tmp_path, &langs);
    let _ = std::fs::remove_file(&tmp);
    result
});
