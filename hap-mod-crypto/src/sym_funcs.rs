use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::json;
use base64::Engine;

// ---------- generate_key ----------
#[derive(Deserialize)]
pub struct GenKeyParams { pub algorithm: String, pub format: Option<String> }
hap_fn!(hap_crypto_generate_key, GenKeyParams, |p| {
    let key_len = match p.algorithm.as_str() {
        "aes-128" => 16,
        "aes-256" | "chacha20" => 32,
        _ => return Err(HapError::invalid_param(format!("unknown algorithm: {}", p.algorithm))),
    };
    let mut key = vec![0u8; key_len];
    getrandom::getrandom(&mut key).map_err(|e| HapError::internal(e.to_string()))?;
    let fmt = p.format.as_deref().unwrap_or("hex");
    Ok(json!(match fmt {
        "base64" => base64::engine::general_purpose::STANDARD.encode(&key),
        _ => hex::encode(&key),
    }))
});

// ---------- encrypt (AES-256-GCM / ChaCha20-Poly1305) ----------
#[derive(Deserialize)]
pub struct EncryptParams { pub algorithm: String, pub key: String, pub data: String, pub iv: Option<String>, #[allow(dead_code)] pub aad: Option<String>, pub encoding: Option<String> }
hap_fn!(hap_crypto_encrypt, EncryptParams, |p| {
    use aes_gcm::aead::Aead;
    let key_bytes = hex::decode(&p.key).map_err(|e| HapError::invalid_param(format!("key hex decode failed: {e}")))?;
    let data_bytes = if p.encoding.as_deref() == Some("hex") {
        hex::decode(&p.data).map_err(|e| HapError::invalid_param(format!("data hex decode: {e}")))?
    } else {
        p.data.as_bytes().to_vec()
    };

    match p.algorithm.as_str() {
        "aes-256-gcm" => {
            use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
            if key_bytes.len() != 32 { return Err(HapError::invalid_param("AES-256 key must be 32 bytes")); }
            let cipher = Aes256Gcm::new_from_slice(&key_bytes).map_err(|e| HapError::internal(e.to_string()))?;
            let iv_bytes = if let Some(ref iv) = p.iv {
                hex::decode(iv).map_err(|e| HapError::invalid_param(format!("iv hex decode failed: {e}")))?
            } else {
                let mut iv = [0u8; 12];
                getrandom::getrandom(&mut iv).map_err(|e| HapError::internal(e.to_string()))?;
                iv.to_vec()
            };
            let nonce = Nonce::from_slice(&iv_bytes);
            let ct = cipher.encrypt(nonce, data_bytes.as_ref())
                .map_err(|e| HapError::internal(format!("encryption failed: {e}")))?;
            Ok(json!({
                "ciphertext": base64::engine::general_purpose::STANDARD.encode(&ct),
                "iv": hex::encode(&iv_bytes),
            }))
        }
        "chacha20-poly1305" => {
            use chacha20poly1305::{ChaCha20Poly1305, KeyInit, Nonce};
            if key_bytes.len() != 32 { return Err(HapError::invalid_param("ChaCha20 key must be 32 bytes")); }
            let cipher = ChaCha20Poly1305::new_from_slice(&key_bytes).map_err(|e| HapError::internal(e.to_string()))?;
            let iv_bytes = if let Some(ref iv) = p.iv {
                hex::decode(iv).map_err(|e| HapError::invalid_param(format!("iv hex decode failed: {e}")))?
            } else {
                let mut iv = [0u8; 12];
                getrandom::getrandom(&mut iv).map_err(|e| HapError::internal(e.to_string()))?;
                iv.to_vec()
            };
            let nonce = Nonce::from_slice(&iv_bytes);
            let ct = cipher.encrypt(nonce, data_bytes.as_ref())
                .map_err(|e| HapError::internal(format!("encryption failed: {e}")))?;
            Ok(json!({
                "ciphertext": base64::engine::general_purpose::STANDARD.encode(&ct),
                "iv": hex::encode(&iv_bytes),
            }))
        }
        _ => Err(HapError::invalid_param(format!("unsupported algorithm: {}", p.algorithm))),
    }
});

// ---------- decrypt ----------
#[derive(Deserialize)]
pub struct DecryptParams { pub algorithm: String, pub key: String, pub ciphertext: String, pub iv: String, #[allow(dead_code)] pub tag: Option<String>, #[allow(dead_code)] pub aad: Option<String>, pub encoding: Option<String> }
hap_fn!(hap_crypto_decrypt, DecryptParams, |p| {
    use aes_gcm::aead::Aead;
    let key_bytes = hex::decode(&p.key).map_err(|e| HapError::invalid_param(format!("key: {e}")))?;
    let iv_bytes = hex::decode(&p.iv).map_err(|e| HapError::invalid_param(format!("iv: {e}")))?;
    let ct_bytes = base64::engine::general_purpose::STANDARD.decode(&p.ciphertext)
        .map_err(|e| HapError::invalid_param(format!("ciphertext: {e}")))?;
    let want_hex = p.encoding.as_deref() == Some("hex");

    match p.algorithm.as_str() {
        "aes-256-gcm" => {
            use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
            let cipher = Aes256Gcm::new_from_slice(&key_bytes).map_err(|e| HapError::internal(e.to_string()))?;
            let nonce = Nonce::from_slice(&iv_bytes);
            let pt = cipher.decrypt(nonce, ct_bytes.as_ref())
                .map_err(|_| HapError::internal("decryption failed: key/IV/ciphertext mismatch"))?;
            if want_hex { Ok(json!(hex::encode(&pt))) } else { Ok(json!(String::from_utf8_lossy(&pt).into_owned())) }
        }
        "chacha20-poly1305" => {
            use chacha20poly1305::{ChaCha20Poly1305, KeyInit, Nonce};
            let cipher = ChaCha20Poly1305::new_from_slice(&key_bytes).map_err(|e| HapError::internal(e.to_string()))?;
            let nonce = Nonce::from_slice(&iv_bytes);
            let pt = cipher.decrypt(nonce, ct_bytes.as_ref())
                .map_err(|_| HapError::internal("decryption failed: key/IV/ciphertext mismatch"))?;
            if want_hex { Ok(json!(hex::encode(&pt))) } else { Ok(json!(String::from_utf8_lossy(&pt).into_owned())) }
        }
        _ => Err(HapError::invalid_param(format!("unsupported: {}", p.algorithm))),
    }
});

// ---------- derive_key ----------
#[derive(Deserialize)]
pub struct DeriveKeyParams { pub password: String, pub salt: String, pub algorithm: String, pub iterations: Option<i32>, pub key_length: Option<i32> }
hap_fn!(hap_crypto_derive_key, DeriveKeyParams, |p| {
    let key_len = p.key_length.unwrap_or(32) as usize;
    let mut output = vec![0u8; key_len];

    match p.algorithm.as_str() {
        "pbkdf2" => {
            let iters = p.iterations.unwrap_or(100_000) as u32;
            ring::pbkdf2::derive(
                ring::pbkdf2::PBKDF2_HMAC_SHA256,
                std::num::NonZeroU32::new(iters).unwrap(),
                p.salt.as_bytes(),
                p.password.as_bytes(),
                &mut output,
            );
        }
        "argon2id" => {
            let params = argon2::Params::new(65536, p.iterations.unwrap_or(3) as u32, 4, Some(key_len))
                .map_err(|e| HapError::internal(e.to_string()))?;
            let argon = argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
            argon.hash_password_into(p.password.as_bytes(), p.salt.as_bytes(), &mut output)
                .map_err(|e| HapError::internal(e.to_string()))?;
        }
        _ => return Err(HapError::invalid_param(format!("unsupported: {}", p.algorithm))),
    }
    Ok(json!(hex::encode(&output)))
});

// ---------- bcrypt_hash ----------
#[derive(Deserialize)]
pub struct BcryptHashParams { pub password: String, pub rounds: Option<i32> }
hap_fn!(hap_crypto_bcrypt_hash, BcryptHashParams, |p| {
    let cost = p.rounds.unwrap_or(12) as u32;
    let hash = bcrypt::hash(&p.password, cost).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(hash))
});

// ---------- bcrypt_verify ----------
#[derive(Deserialize)]
pub struct BcryptVerifyParams { pub password: String, pub hash: String }
hap_fn!(hap_crypto_bcrypt_verify, BcryptVerifyParams, |p| {
    let valid = bcrypt::verify(&p.password, &p.hash).unwrap_or(false);
    Ok(json!(valid))
});

// ---------- encrypt_with_password ----------
#[derive(Deserialize)]
pub struct EncWithPwdParams { pub password: String, pub data: String, #[allow(dead_code)] pub algorithm: Option<String> }
hap_fn!(hap_crypto_encrypt_with_password, EncWithPwdParams, |p| {
    use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
    let mut salt = [0u8; 16];
    getrandom::getrandom(&mut salt).map_err(|e| HapError::internal(e.to_string()))?;
    let mut key = [0u8; 32];
    ring::pbkdf2::derive(
        ring::pbkdf2::PBKDF2_HMAC_SHA256,
        std::num::NonZeroU32::new(100_000).unwrap(),
        &salt, p.password.as_bytes(), &mut key,
    );
    let mut iv = [0u8; 12];
    getrandom::getrandom(&mut iv).map_err(|e| HapError::internal(e.to_string()))?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| HapError::internal(e.to_string()))?;
    let ct = cipher.encrypt(Nonce::from_slice(&iv), p.data.as_bytes().as_ref())
        .map_err(|e| HapError::internal(format!("encryption failed: {e}")))?;
    // Format: salt(16) + iv(12) + ciphertext (includes 16-byte tag)
    let mut combined = Vec::with_capacity(16 + 12 + ct.len());
    combined.extend_from_slice(&salt);
    combined.extend_from_slice(&iv);
    combined.extend_from_slice(&ct);
    Ok(json!(base64::engine::general_purpose::STANDARD.encode(&combined)))
});

// ---------- encrypt_file ----------
#[derive(Deserialize)]
pub struct EncryptFileParams {
    pub path: String, pub output_path: String, pub key: String,
    #[allow(dead_code)] pub algorithm: Option<String>, #[allow(dead_code)] pub callback_id: Option<String>,
}
hap_fn!(hap_crypto_encrypt_file, EncryptFileParams, |p| {
    use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
    let key_bytes = hex::decode(&p.key).map_err(|e| HapError::invalid_param(format!("key: {e}")))?;
    if key_bytes.len() != 32 { return Err(HapError::invalid_param("key must be 32 bytes")); }
    let data = std::fs::read(&p.path)?;
    let mut iv = [0u8; 12];
    getrandom::getrandom(&mut iv).map_err(|e| HapError::internal(e.to_string()))?;
    let cipher = Aes256Gcm::new_from_slice(&key_bytes).map_err(|e| HapError::internal(e.to_string()))?;
    let ct = cipher.encrypt(Nonce::from_slice(&iv), data.as_ref())
        .map_err(|e| HapError::internal(format!("encryption failed: {e}")))?;
    if let Some(parent) = std::path::Path::new(&p.output_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&p.output_path, &ct)?;
    Ok(json!({ "size": ct.len() as i64, "iv": hex::encode(iv), "tag": "" }))
});

// ---------- decrypt_file ----------
#[derive(Deserialize)]
pub struct DecryptFileParams {
    pub path: String, pub output_path: String, pub key: String,
    pub iv: String, #[allow(dead_code)] pub tag: String,
    #[allow(dead_code)] pub algorithm: Option<String>, #[allow(dead_code)] pub callback_id: Option<String>,
}
hap_fn!(hap_crypto_decrypt_file, DecryptFileParams, |p| {
    use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
    let key_bytes = hex::decode(&p.key).map_err(|e| HapError::invalid_param(format!("key: {e}")))?;
    let iv_bytes = hex::decode(&p.iv).map_err(|e| HapError::invalid_param(format!("iv: {e}")))?;
    let ct = std::fs::read(&p.path)?;
    let cipher = Aes256Gcm::new_from_slice(&key_bytes).map_err(|e| HapError::internal(e.to_string()))?;
    let pt = cipher.decrypt(Nonce::from_slice(&iv_bytes), ct.as_ref())
        .map_err(|_| HapError::internal("decryption failed"))?;
    if let Some(parent) = std::path::Path::new(&p.output_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&p.output_path, &pt)?;
    Ok(json!({ "size": pt.len() as i64 }))
});

// ---------- decrypt_with_password ----------
#[derive(Deserialize)]
pub struct DecWithPwdParams { pub password: String, pub encrypted: String }
hap_fn!(hap_crypto_decrypt_with_password, DecWithPwdParams, |p| {
    use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
    let data = base64::engine::general_purpose::STANDARD.decode(&p.encrypted)
        .map_err(|e| HapError::invalid_param(format!("base64: {e}")))?;
    if data.len() < 28 { return Err(HapError::invalid_param("ciphertext too short")); }
    let salt = &data[..16];
    let iv = &data[16..28];
    let ct = &data[28..];
    let mut key = [0u8; 32];
    ring::pbkdf2::derive(
        ring::pbkdf2::PBKDF2_HMAC_SHA256,
        std::num::NonZeroU32::new(100_000).unwrap(),
        salt, p.password.as_bytes(), &mut key,
    );
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| HapError::internal(e.to_string()))?;
    let pt = cipher.decrypt(Nonce::from_slice(iv), ct)
        .map_err(|_| HapError::internal("decryption failed: wrong password or corrupted data"))?;
    Ok(json!(String::from_utf8_lossy(&pt).into_owned()))
});
