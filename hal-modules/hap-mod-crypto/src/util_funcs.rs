use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::json;
use base64::Engine;

// ---------- random_bytes ----------
#[derive(Deserialize)]
pub struct RandomBytesParams { pub length: u32 }
hap_fn!(hap_crypto_random_bytes, RandomBytesParams, |p| {
    let mut buf = vec![0u8; p.length as usize];
    getrandom::getrandom(&mut buf).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(hex::encode(&buf)))
});

// ---------- generate_uuid ----------
#[derive(Deserialize)]
pub struct GenUuidParams { pub version: Option<String> }
hap_fn!(hap_crypto_generate_uuid, GenUuidParams, |p| {
    let id = match p.version.as_deref().unwrap_or("v4") {
        "v7" => uuid::Uuid::now_v7().to_string(),
        _ => uuid::Uuid::new_v4().to_string(),
    };
    Ok(json!(id))
});

// ---------- random_string ----------
#[derive(Deserialize)]
pub struct RandomStringParams { pub length: Option<i32>, pub charset: Option<String>, pub custom_chars: Option<String> }
hap_fn!(hap_crypto_random_string, RandomStringParams, |p| {
    use rand::Rng;
    let len = p.length.unwrap_or(16) as usize;
    let chars: Vec<char> = match p.charset.as_deref().unwrap_or("alphanumeric") {
        "alpha" => ('a'..='z').chain('A'..='Z').collect(),
        "numeric" => ('0'..='9').collect(),
        "hex" => ('0'..='9').chain('a'..='f').collect(),
        "base62" => ('0'..='9').chain('a'..='z').chain('A'..='Z').collect(),
        "custom" => p.custom_chars.as_deref().unwrap_or("").chars().collect(),
        _ => ('0'..='9').chain('a'..='z').chain('A'..='Z').collect(),
    };
    if chars.is_empty() { return Err(HapError::invalid_param("charset is empty")); }
    let mut rng = rand::thread_rng();
    let s: String = (0..len).map(|_| chars[rng.gen_range(0..chars.len())]).collect();
    Ok(json!(s))
});

// ---------- generate_password ----------
#[derive(Deserialize)]
pub struct GenPasswordParams {
    pub length: Option<i32>, pub uppercase: Option<bool>, pub lowercase: Option<bool>,
    pub digits: Option<bool>, pub symbols: Option<bool>,
    pub exclude_chars: Option<String>, pub custom_chars: Option<String>,
}
hap_fn!(hap_crypto_generate_password, GenPasswordParams, |p| {
    use rand::Rng;
    let len = p.length.unwrap_or(16) as usize;
    let mut chars = Vec::new();
    if p.uppercase.unwrap_or(true) { chars.extend('A'..='Z'); }
    if p.lowercase.unwrap_or(true) { chars.extend('a'..='z'); }
    if p.digits.unwrap_or(true) { chars.extend('0'..='9'); }
    if p.symbols.unwrap_or(true) { chars.extend("!@#$%^&*()-_=+[]{}|;:,.<>?".chars()); }
    if let Some(ref custom) = p.custom_chars { chars.extend(custom.chars()); }
    if let Some(ref exclude) = p.exclude_chars {
        let ex: Vec<char> = exclude.chars().collect();
        chars.retain(|c| !ex.contains(c));
    }
    if chars.is_empty() { return Err(HapError::invalid_param("charset is empty")); }
    let mut rng = rand::thread_rng();
    let pwd: String = (0..len).map(|_| chars[rng.gen_range(0..chars.len())]).collect();
    Ok(json!(pwd))
});

// ---------- generate_totp ----------
#[derive(Deserialize)]
pub struct GenTotpParams { pub secret: String, pub digits: Option<i32>, pub period: Option<i32>, pub algorithm: Option<String> }
hap_fn!(hap_crypto_generate_totp, GenTotpParams, |p| {
    let algo = match p.algorithm.as_deref().unwrap_or("sha1") {
        "sha256" => totp_rs::Algorithm::SHA256,
        _ => totp_rs::Algorithm::SHA1,
    };
    let totp = totp_rs::TOTP::new(algo, p.digits.unwrap_or(6) as usize, 1, p.period.unwrap_or(30) as u64, p.secret.as_bytes().to_vec(), None, "".to_string())
        .map_err(|e| HapError::internal(e.to_string()))?;
    let code = totp.generate_current().map_err(|e| HapError::internal(e.to_string()))?;
    let time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let period = p.period.unwrap_or(30) as u64;
    let remaining = period - (time % period);
    Ok(json!({ "code": code, "remaining_seconds": remaining }))
});

// ---------- verify_totp ----------
#[derive(Deserialize)]
pub struct VerifyTotpParams { pub secret: String, pub code: String, pub digits: Option<i32>, pub period: Option<i32>, pub window: Option<i32> }
hap_fn!(hap_crypto_verify_totp, VerifyTotpParams, |p| {
    let algo = totp_rs::Algorithm::SHA1;
    let skew = p.window.unwrap_or(1) as u8;
    let totp = totp_rs::TOTP::new(algo, p.digits.unwrap_or(6) as usize, skew, p.period.unwrap_or(30) as u64, p.secret.as_bytes().to_vec(), None, "".to_string())
        .map_err(|e| HapError::internal(e.to_string()))?;
    let time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    Ok(json!(totp.check(&p.code, time)))
});

// ---------- generate_totp_secret ----------
#[derive(Deserialize)]
pub struct GenTotpSecretParams {
    pub issuer: Option<String>, pub account: Option<String>,
    pub length: Option<i32>, pub algorithm: Option<String>,
    pub digits: Option<i32>, pub period: Option<i32>,
}
hap_fn!(hap_crypto_generate_totp_secret, GenTotpSecretParams, |p| {
    let len = p.length.unwrap_or(20) as usize;
    let mut bytes = vec![0u8; len];
    getrandom::getrandom(&mut bytes).map_err(|e| HapError::internal(e.to_string()))?;
    let secret = data_encoding::BASE32_NOPAD.encode(&bytes);
    let issuer = p.issuer.as_deref().unwrap_or("HiAppHub");
    let account = p.account.as_deref().unwrap_or("user");
    let digits = p.digits.unwrap_or(6);
    let period = p.period.unwrap_or(30);
    let algo = p.algorithm.as_deref().unwrap_or("sha1").to_uppercase();
    let uri = format!("otpauth://totp/{issuer}:{account}?secret={secret}&issuer={issuer}&algorithm={algo}&digits={digits}&period={period}");
    Ok(json!({ "secret": secret, "uri": uri }))
});

// ---------- x509_info ----------
#[derive(Deserialize)]
pub struct X509InfoParams { pub cert_pem: String }
hap_fn!(hap_crypto_x509_info, X509InfoParams, |p| {
    use x509_parser::prelude::*;
    let parsed = ::pem::parse(&p.cert_pem).map_err(|e| HapError::invalid_param(format!("PEM parse failed: {e}")))?;
    let (_, cert) = X509Certificate::from_der(parsed.contents())
        .map_err(|e| HapError::invalid_param(format!("X.509 parse failed: {e}")))?;
    let subject = cert.subject().to_string();
    let issuer = cert.issuer().to_string();
    let serial = cert.raw_serial_as_string();
    Ok(json!({
        "subject": subject,
        "issuer": issuer,
        "serial": serial,
        "valid_from": cert.validity().not_before.to_rfc2822(),
        "valid_to": cert.validity().not_after.to_rfc2822(),
        "is_ca": cert.is_ca(),
    }))
});

// ---------- pem_to_der ----------
#[derive(Deserialize)]
pub struct PemToDerParams { pub pem: String }
hap_fn!(hap_crypto_pem_to_der, PemToDerParams, |p| {
    let parsed = ::pem::parse(&p.pem).map_err(|e| HapError::invalid_param(format!("PEM parse failed: {e}")))?;
    Ok(json!(base64::engine::general_purpose::STANDARD.encode(parsed.contents())))
});

// ---------- der_to_pem ----------
#[derive(Deserialize)]
pub struct DerToPemParams { pub der: String, pub label: String }
hap_fn!(hap_crypto_der_to_pem, DerToPemParams, |p| {
    let der_bytes = base64::engine::general_purpose::STANDARD.decode(&p.der)
        .map_err(|e| HapError::invalid_param(format!("base64: {e}")))?;
    let pem_obj = ::pem::Pem::new(p.label, der_bytes);
    Ok(json!(::pem::encode(&pem_obj)))
});
