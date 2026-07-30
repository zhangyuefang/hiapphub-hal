use hap_common::{hap_free_string, hap_module_init, hap_fn, HapError};
use serde::Deserialize;
use serde_json::json;

hap_module_init!("encoding");
hap_free_string!();

// ---------- 1. base64_encode ----------
#[derive(Deserialize)]
struct Base64EncodeParams { data: String }
hap_fn!(hap_encoding_base64_encode, Base64EncodeParams, |p| {
    use base64::Engine;
    Ok(json!(base64::engine::general_purpose::STANDARD.encode(p.data.as_bytes())))
});

// ---------- 2. base64_decode ----------
#[derive(Deserialize)]
struct Base64DecodeParams { data: String }
hap_fn!(hap_encoding_base64_decode, Base64DecodeParams, |p| {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD.decode(&p.data)
        .map_err(|e| HapError::invalid_param(format!("base64 decode failed: {e}")))?;
    let s = String::from_utf8(bytes)
        .map_err(|e| HapError::invalid_param(format!("UTF-8 decode failed: {e}")))?;
    Ok(json!(s))
});

// ---------- 3. base64url_encode ----------
#[derive(Deserialize)]
struct Base64urlEncodeParams { data: String }
hap_fn!(hap_encoding_base64url_encode, Base64urlEncodeParams, |p| {
    use base64::Engine;
    Ok(json!(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(p.data.as_bytes())))
});

// ---------- 4. base64url_decode ----------
#[derive(Deserialize)]
struct Base64urlDecodeParams { data: String }
hap_fn!(hap_encoding_base64url_decode, Base64urlDecodeParams, |p| {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(&p.data)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(&p.data))
        .map_err(|e| HapError::invalid_param(format!("base64url decode failed: {e}")))?;
    let s = String::from_utf8(bytes)
        .map_err(|e| HapError::invalid_param(format!("UTF-8 decode failed: {e}")))?;
    Ok(json!(s))
});

// ---------- 5. base32_encode ----------
#[derive(Deserialize)]
struct Base32EncodeParams { data: String, padding: Option<bool> }
hap_fn!(hap_encoding_base32_encode, Base32EncodeParams, |p| {
    let padding = p.padding.unwrap_or(true);
    let encoded = if padding {
        data_encoding::BASE32.encode(p.data.as_bytes())
    } else {
        data_encoding::BASE32_NOPAD.encode(p.data.as_bytes())
    };
    Ok(json!(encoded))
});

// ---------- 6. base32_decode ----------
#[derive(Deserialize)]
struct Base32DecodeParams { data: String }
hap_fn!(hap_encoding_base32_decode, Base32DecodeParams, |p| {
    let bytes = data_encoding::BASE32_NOPAD.decode(p.data.trim_end_matches('=').as_bytes())
        .or_else(|_| data_encoding::BASE32.decode(p.data.as_bytes()))
        .map_err(|e| HapError::invalid_param(format!("base32 decode failed: {e}")))?;
    let s = String::from_utf8(bytes)
        .map_err(|e| HapError::invalid_param(format!("UTF-8 decode failed: {e}")))?;
    Ok(json!(s))
});

// ---------- 7. hex_encode ----------
#[derive(Deserialize)]
struct HexEncodeParams { data: String }
hap_fn!(hap_encoding_hex_encode, HexEncodeParams, |p| {
    let hex: String = p.data.as_bytes().iter().map(|b| format!("{b:02x}")).collect();
    Ok(json!(hex))
});

// ---------- 8. hex_decode ----------
#[derive(Deserialize)]
struct HexDecodeParams { data: String }
hap_fn!(hap_encoding_hex_decode, HexDecodeParams, |p| {
    let hex = p.data.trim();
    if hex.len() % 2 != 0 {
        return Err(HapError::invalid_param("hex string length must be even"));
    }
    let bytes: Result<Vec<u8>, _> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i+2], 16))
        .collect();
    let bytes = bytes.map_err(|e| HapError::invalid_param(format!("hex decode failed: {e}")))?;
    let s = String::from_utf8(bytes)
        .map_err(|e| HapError::invalid_param(format!("UTF-8 decode failed: {e}")))?;
    Ok(json!(s))
});

// ---------- 9. url_encode ----------
#[derive(Deserialize)]
struct UrlEncodeParams { data: String, component: Option<bool> }
hap_fn!(hap_encoding_url_encode, UrlEncodeParams, |p| {
    let component = p.component.unwrap_or(true);
    let encoded = if component {
        percent_encoding::utf8_percent_encode(&p.data, percent_encoding::NON_ALPHANUMERIC).to_string()
    } else {
        percent_encoding::utf8_percent_encode(&p.data, percent_encoding::CONTROLS).to_string()
    };
    Ok(json!(encoded))
});

// ---------- 10. url_decode ----------
#[derive(Deserialize)]
struct UrlDecodeParams { data: String }
hap_fn!(hap_encoding_url_decode, UrlDecodeParams, |p| {
    let decoded = percent_encoding::percent_decode_str(&p.data)
        .decode_utf8()
        .map_err(|e| HapError::invalid_param(format!("URL decode failed: {e}")))?;
    Ok(json!(decoded.into_owned()))
});

// ---------- 11. html_encode ----------
#[derive(Deserialize)]
struct HtmlEncodeParams { data: String }
hap_fn!(hap_encoding_html_encode, HtmlEncodeParams, |p| {
    Ok(json!(htmlize::escape_text(&p.data).to_string()))
});

// ---------- 12. html_decode ----------
#[derive(Deserialize)]
struct HtmlDecodeParams { data: String }
hap_fn!(hap_encoding_html_decode, HtmlDecodeParams, |p| {
    Ok(json!(htmlize::unescape(&p.data).to_string()))
});

// ---------- 13. text_convert ----------
#[derive(Deserialize)]
struct TextConvertParams { data: String, from: String, to: String }
hap_fn!(hap_encoding_text_convert, TextConvertParams, |p| {
    let src_enc = encoding_rs::Encoding::for_label(p.from.as_bytes())
        .ok_or_else(|| HapError::invalid_param(format!("unknown encoding: {}", p.from)))?;
    let dst_enc = encoding_rs::Encoding::for_label(p.to.as_bytes())
        .ok_or_else(|| HapError::invalid_param(format!("unknown encoding: {}", p.to)))?;

    let (decoded, _, had_errors) = src_enc.decode(p.data.as_bytes());
    if had_errors {
        return Err(HapError::invalid_param(format!("decode from {} failed", p.from)));
    }
    let (encoded, _, had_errors) = dst_enc.encode(&decoded);
    if had_errors {
        return Err(HapError::invalid_param(format!("encode to {} failed", p.to)));
    }
    let result = String::from_utf8_lossy(&encoded).into_owned();
    Ok(json!(result))
});

// ---------- 14. detect_encoding ----------
#[derive(Deserialize)]
struct DetectEncodingParams { file_path: String, sample_size: Option<i32> }
hap_fn!(hap_encoding_detect_encoding, DetectEncodingParams, |p| {
    let sample_size = p.sample_size.unwrap_or(65536) as usize;
    let data = std::fs::read(&p.file_path)?;
    let sample = if data.len() > sample_size { &data[..sample_size] } else { &data };

    let mut detector = chardetng::EncodingDetector::new();
    detector.feed(sample, true);
    let (encoding, confident) = (detector.guess(None, true), detector.guess_assess(None, true).1);
    let confidence = if confident { 0.95 } else { 0.5 };

    Ok(json!({
        "encoding": encoding.name(),
        "confidence": confidence,
    }))
});

// ---------- 15. convert_file ----------
#[derive(Deserialize)]
struct ConvertFileParams { input_path: String, output_path: String, from: String, to: Option<String>, bom: Option<bool> }
hap_fn!(hap_encoding_convert_file, ConvertFileParams, |p| {
    let to_name = p.to.as_deref().unwrap_or("UTF-8");
    let src_enc = encoding_rs::Encoding::for_label(p.from.as_bytes())
        .ok_or_else(|| HapError::invalid_param(format!("unknown encoding: {}", p.from)))?;
    let dst_enc = encoding_rs::Encoding::for_label(to_name.as_bytes())
        .ok_or_else(|| HapError::invalid_param(format!("unknown encoding: {}", to_name)))?;

    let raw = std::fs::read(&p.input_path)?;
    let (decoded, _, _) = src_enc.decode(&raw);
    let (encoded, _, _) = dst_enc.encode(&decoded);

    let mut output = Vec::new();
    if p.bom.unwrap_or(false) && (dst_enc == encoding_rs::UTF_8) {
        output.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    }
    output.extend_from_slice(&encoded);

    if let Some(parent) = std::path::Path::new(&p.output_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&p.output_path, &output)?;
    Ok(json!({ "size": output.len() as i64 }))
});

// ---------- 16. base58_encode ----------
#[derive(Deserialize)]
struct Base58EncodeParams { data: String, alphabet: Option<String> }
hap_fn!(hap_encoding_base58_encode, Base58EncodeParams, |p| {
    let alpha = match p.alphabet.as_deref().unwrap_or("bitcoin") {
        "ripple" => bs58::Alphabet::RIPPLE,
        _ => bs58::Alphabet::BITCOIN,
    };
    let encoded = bs58::encode(p.data.as_bytes()).with_alphabet(alpha).into_string();
    Ok(json!(encoded))
});

// ---------- 17. base58_decode ----------
#[derive(Deserialize)]
struct Base58DecodeParams { data: String, alphabet: Option<String> }
hap_fn!(hap_encoding_base58_decode, Base58DecodeParams, |p| {
    let alpha = match p.alphabet.as_deref().unwrap_or("bitcoin") {
        "ripple" => bs58::Alphabet::RIPPLE,
        _ => bs58::Alphabet::BITCOIN,
    };
    let bytes = bs58::decode(&p.data).with_alphabet(alpha).into_vec()
        .map_err(|e| HapError::invalid_param(format!("base58 decode failed: {e}")))?;
    let s = String::from_utf8(bytes)
        .map_err(|e| HapError::invalid_param(format!("UTF-8 decode failed: {e}")))?;
    Ok(json!(s))
});

// ---------- hap_module_describe ----------
#[no_mangle]
pub extern "C" fn hap_module_describe() -> *const std::os::raw::c_char {
    hap_common::ffi::str_to_c(include_str!("../manifest.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::ffi::{CStr, CString};

    fn call(func: extern "C" fn(*const std::os::raw::c_char) -> *const std::os::raw::c_char, json: &str) -> Value {
        let cs = CString::new(json).unwrap();
        let result = func(cs.as_ptr());
        assert!(!result.is_null());
        let s = unsafe { CStr::from_ptr(result) }.to_str().unwrap().to_string();
        unsafe { hap_free_string(result as *mut _) };
        serde_json::from_str(&s).unwrap()
    }

    #[test]
    fn test_base64_roundtrip() {
        let enc = call(hap_encoding_base64_encode, r#"{"data":"Hello, 世界!"}"#);
        assert_eq!(enc.as_str().unwrap(), "SGVsbG8sIOS4lueVjCE=");
        let dec = call(hap_encoding_base64_decode, &format!(r#"{{"data":"{}"}}"#, enc.as_str().unwrap()));
        assert_eq!(dec.as_str().unwrap(), "Hello, 世界!");
    }

    #[test]
    fn test_base64url_roundtrip() {
        let enc = call(hap_encoding_base64url_encode, r#"{"data":"test+data/here"}"#);
        let dec = call(hap_encoding_base64url_decode, &format!(r#"{{"data":"{}"}}"#, enc.as_str().unwrap()));
        assert_eq!(dec.as_str().unwrap(), "test+data/here");
    }

    #[test]
    fn test_base32_roundtrip() {
        let enc = call(hap_encoding_base32_encode, r#"{"data":"Hello"}"#);
        assert_eq!(enc.as_str().unwrap(), "JBSWY3DP");
        let dec = call(hap_encoding_base32_decode, &format!(r#"{{"data":"{}"}}"#, enc.as_str().unwrap()));
        assert_eq!(dec.as_str().unwrap(), "Hello");
    }

    #[test]
    fn test_base32_no_padding() {
        let enc = call(hap_encoding_base32_encode, r#"{"data":"Hi","padding":false}"#);
        assert!(!enc.as_str().unwrap().ends_with('='));
        let dec = call(hap_encoding_base32_decode, &format!(r#"{{"data":"{}"}}"#, enc.as_str().unwrap()));
        assert_eq!(dec.as_str().unwrap(), "Hi");
    }

    #[test]
    fn test_hex_roundtrip() {
        let enc = call(hap_encoding_hex_encode, r#"{"data":"ABC"}"#);
        assert_eq!(enc.as_str().unwrap(), "414243");
        let dec = call(hap_encoding_hex_decode, r#"{"data":"414243"}"#);
        assert_eq!(dec.as_str().unwrap(), "ABC");
    }

    #[test]
    fn test_hex_odd_length_error() {
        let result = call(hap_encoding_hex_decode, r#"{"data":"41424"}"#);
        assert!(result.get("error").is_some());
    }

    #[test]
    fn test_url_encode_component() {
        let enc = call(hap_encoding_url_encode, r#"{"data":"hello world&foo=bar"}"#);
        assert!(enc.as_str().unwrap().contains("%20") || enc.as_str().unwrap().contains("+"));
        assert!(!enc.as_str().unwrap().contains(' '));
    }

    #[test]
    fn test_url_decode() {
        let dec = call(hap_encoding_url_decode, r#"{"data":"hello%20world%26foo%3Dbar"}"#);
        assert_eq!(dec.as_str().unwrap(), "hello world&foo=bar");
    }

    #[test]
    fn test_html_encode_decode() {
        let enc = call(hap_encoding_html_encode, r#"{"data":"<div class=\"test\">&</div>"}"#);
        let s = enc.as_str().unwrap();
        assert!(s.contains("&lt;") && s.contains("&amp;"));
        let dec = call(hap_encoding_html_decode, &format!(r#"{{"data":"{}"}}"#, s.replace('"', "\\\"")));
        assert_eq!(dec.as_str().unwrap(), "<div class=\"test\">&</div>");
    }

    #[test]
    fn test_base58_roundtrip() {
        let enc = call(hap_encoding_base58_encode, r#"{"data":"Hello World"}"#);
        let dec = call(hap_encoding_base58_decode, &format!(r#"{{"data":"{}"}}"#, enc.as_str().unwrap()));
        assert_eq!(dec.as_str().unwrap(), "Hello World");
    }

    #[test]
    fn test_base58_ripple() {
        let enc = call(hap_encoding_base58_encode, r#"{"data":"test","alphabet":"ripple"}"#);
        let dec = call(hap_encoding_base58_decode, &format!(r#"{{"data":"{}","alphabet":"ripple"}}"#, enc.as_str().unwrap()));
        assert_eq!(dec.as_str().unwrap(), "test");
    }

    #[test]
    fn test_detect_encoding() {
        let tmp = std::env::temp_dir().join("hap_test_detect.txt");
        std::fs::write(&tmp, "Hello, 世界！这是 UTF-8 文本。").unwrap();
        let result = call(hap_encoding_detect_encoding, &format!(r#"{{"file_path":"{}"}}"#, tmp.to_string_lossy().replace('\\', "\\\\")));
        assert!(result.get("encoding").is_some());
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_convert_file() {
        let tmp_in = std::env::temp_dir().join("hap_conv_in.txt");
        let tmp_out = std::env::temp_dir().join("hap_conv_out.txt");
        std::fs::write(&tmp_in, "Hello UTF-8").unwrap();
        let result = call(hap_encoding_convert_file, &format!(
            r#"{{"input_path":"{}","output_path":"{}","from":"UTF-8","to":"UTF-8","bom":true}}"#,
            tmp_in.to_string_lossy().replace('\\', "\\\\"),
            tmp_out.to_string_lossy().replace('\\', "\\\\")
        ));
        assert!(result.get("size").is_some());
        let out_bytes = std::fs::read(&tmp_out).unwrap();
        assert_eq!(&out_bytes[..3], &[0xEF, 0xBB, 0xBF]); // BOM
        std::fs::remove_file(&tmp_in).ok();
        std::fs::remove_file(&tmp_out).ok();
    }

    #[test]
    fn test_describe_valid_json() {
        let ptr = hap_module_describe();
        assert!(!ptr.is_null());
        let s = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        let v: Value = serde_json::from_str(s).unwrap();
        assert_eq!(v["name"], "encoding");
        assert_eq!(v["functions"].as_array().unwrap().len(), 17);
        unsafe { hap_free_string(ptr as *mut _) };
    }
}
