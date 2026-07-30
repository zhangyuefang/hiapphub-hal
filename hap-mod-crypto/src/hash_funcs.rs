use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::json;
use hmac::Mac;

// ---------- hash ----------
#[derive(Deserialize)]
pub struct HashParams { pub algorithm: String, pub data: String, pub encoding: Option<String> }
hap_fn!(hap_crypto_hash, HashParams, |p| {
    use sha2::{Sha256, Sha384, Sha512, Digest};
    use sha1::Sha1;
    use md5::Md5;
    use sha3::{Sha3_256, Sha3_512};
    let raw = if p.encoding.as_deref() == Some("hex") {
        hex::decode(&p.data).map_err(|e| HapError::invalid_param(format!("data hex decode: {e}")))?
    } else {
        p.data.as_bytes().to_vec()
    };
    let bytes = raw.as_slice();
    let hex_result = match p.algorithm.as_str() {
        "md5" => hex::encode(Md5::digest(bytes)),
        "sha1" => hex::encode(Sha1::digest(bytes)),
        "sha256" => hex::encode(Sha256::digest(bytes)),
        "sha384" => hex::encode(Sha384::digest(bytes)),
        "sha512" => hex::encode(Sha512::digest(bytes)),
        "sha3-256" => hex::encode(Sha3_256::digest(bytes)),
        "sha3-512" => hex::encode(Sha3_512::digest(bytes)),
        "blake3" => blake3::hash(bytes).to_hex().to_string(),
        _ => return Err(HapError::invalid_param(format!("unknown hash algorithm: {}", p.algorithm))),
    };
    Ok(json!(hex_result))
});

// ---------- hash_file ----------
#[derive(Deserialize)]
pub struct HashFileParams { pub algorithm: String, pub path: String, #[allow(dead_code)] pub callback_id: Option<String> }
hap_fn!(hap_crypto_hash_file, HashFileParams, |p| {
    use std::io::Read;
    use sha2::{Sha256, Sha512, Digest};
    let mut file = std::fs::File::open(&p.path)?;
    let hex_result = match p.algorithm.as_str() {
        "sha256" => {
            let mut hasher = Sha256::new();
            let mut buf = [0u8; 8192];
            loop {
                let n = file.read(&mut buf)?;
                if n == 0 { break; }
                hasher.update(&buf[..n]);
            }
            hex::encode(hasher.finalize())
        }
        "sha512" => {
            let mut hasher = Sha512::new();
            let mut buf = [0u8; 8192];
            loop {
                let n = file.read(&mut buf)?;
                if n == 0 { break; }
                hasher.update(&buf[..n]);
            }
            hex::encode(hasher.finalize())
        }
        "blake3" => {
            let mut hasher = blake3::Hasher::new();
            let mut buf = [0u8; 8192];
            loop {
                let n = file.read(&mut buf)?;
                if n == 0 { break; }
                hasher.update(&buf[..n]);
            }
            hasher.finalize().to_hex().to_string()
        }
        "md5" => {
            use md5::Md5;
            let mut hasher = Md5::new();
            let mut buf = [0u8; 8192];
            loop {
                let n = file.read(&mut buf)?;
                if n == 0 { break; }
                hasher.update(&buf[..n]);
            }
            hex::encode(hasher.finalize())
        }
        _ => return Err(HapError::invalid_param(format!("file hash not supported: {}", p.algorithm))),
    };
    Ok(json!(hex_result))
});

// ---------- hmac ----------
#[derive(Deserialize)]
pub struct HmacParams { pub algorithm: String, pub key: String, pub data: String }
hap_fn!(hap_crypto_hmac, HmacParams, |p| {
    let result = match p.algorithm.as_str() {
        "sha1" => {
            type HmacSha1 = hmac::Hmac<sha1::Sha1>;
            let mut mac = HmacSha1::new_from_slice(p.key.as_bytes())
                .map_err(|e| HapError::internal(e.to_string()))?;
            mac.update(p.data.as_bytes());
            hex::encode(mac.finalize().into_bytes())
        }
        "sha256" => {
            type HmacSha256 = hmac::Hmac<sha2::Sha256>;
            let mut mac = HmacSha256::new_from_slice(p.key.as_bytes())
                .map_err(|e| HapError::internal(e.to_string()))?;
            mac.update(p.data.as_bytes());
            hex::encode(mac.finalize().into_bytes())
        }
        "sha384" => {
            type HmacSha384 = hmac::Hmac<sha2::Sha384>;
            let mut mac = HmacSha384::new_from_slice(p.key.as_bytes())
                .map_err(|e| HapError::internal(e.to_string()))?;
            mac.update(p.data.as_bytes());
            hex::encode(mac.finalize().into_bytes())
        }
        "sha512" => {
            type HmacSha512 = hmac::Hmac<sha2::Sha512>;
            let mut mac = HmacSha512::new_from_slice(p.key.as_bytes())
                .map_err(|e| HapError::internal(e.to_string()))?;
            mac.update(p.data.as_bytes());
            hex::encode(mac.finalize().into_bytes())
        }
        _ => return Err(HapError::invalid_param(format!("unknown HMAC algorithm: {}", p.algorithm))),
    };
    Ok(json!(result))
});

// ---------- crc32 ----------
#[derive(Deserialize)]
pub struct Crc32Params { pub data: String }
hap_fn!(hap_crypto_crc32, Crc32Params, |p| {
    let crc = crc32fast::hash(p.data.as_bytes());
    Ok(json!(format!("{crc:08x}")))
});

// ---------- constant_time_eq ----------
#[derive(Deserialize)]
pub struct ConstTimeEqParams { pub a: String, pub b: String }
hap_fn!(hap_crypto_constant_time_eq, ConstTimeEqParams, |p| {
    use subtle::ConstantTimeEq;
    let eq: bool = p.a.as_bytes().ct_eq(p.b.as_bytes()).into();
    Ok(json!(eq))
});
