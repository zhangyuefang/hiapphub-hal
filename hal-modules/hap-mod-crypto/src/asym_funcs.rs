use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::json;
use base64::Engine;

// ---------- generate_keypair ----------
#[derive(Deserialize)]
pub struct GenKeypairParams { pub algorithm: String, #[allow(dead_code)] pub format: Option<String> }
hap_fn!(hap_crypto_generate_keypair, GenKeypairParams, |p| {
    match p.algorithm.as_str() {
        "ed25519" => {
            use ed25519_dalek::SigningKey;
            let mut rng = rand::rngs::OsRng;
            let sk = SigningKey::generate(&mut rng);
            let pk = sk.verifying_key();
            Ok(json!({
                "private_key": hex::encode(sk.to_bytes()),
                "public_key": hex::encode(pk.to_bytes()),
            }))
        }
        "x25519" => {
            use x25519_dalek::{StaticSecret, PublicKey};
            let sk = StaticSecret::random_from_rng(rand::rngs::OsRng);
            let pk = PublicKey::from(&sk);
            Ok(json!({
                "private_key": hex::encode(sk.to_bytes()),
                "public_key": hex::encode(pk.to_bytes()),
            }))
        }
        "rsa-2048" | "rsa-4096" => {
            use rsa::{RsaPrivateKey, pkcs8::EncodePrivateKey, pkcs8::EncodePublicKey};
            let bits = if p.algorithm == "rsa-4096" { 4096 } else { 2048 };
            let mut rng = rand::rngs::OsRng;
            let sk = RsaPrivateKey::new(&mut rng, bits)
                .map_err(|e| HapError::internal(format!("RSA key generation failed: {e}")))?;
            let pk = sk.to_public_key();
            let sk_pem = sk.to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
                .map_err(|e| HapError::internal(e.to_string()))?;
            let pk_pem = pk.to_public_key_pem(rsa::pkcs8::LineEnding::LF)
                .map_err(|e| HapError::internal(e.to_string()))?;
            Ok(json!({
                "private_key": sk_pem.as_str(),
                "public_key": pk_pem,
            }))
        }
        _ => Err(HapError::invalid_param(format!("unsupported: {}", p.algorithm))),
    }
});

// ---------- sign ----------
#[derive(Deserialize)]
pub struct SignParams { pub algorithm: String, pub private_key: String, pub data: String, pub encoding: Option<String> }
hap_fn!(hap_crypto_sign, SignParams, |p| {
    let is_hex = p.encoding.as_deref() == Some("hex");
    let data_bytes = if is_hex {
        hex::decode(&p.data).map_err(|e| HapError::invalid_param(format!("data hex: {e}")))?
    } else {
        p.data.as_bytes().to_vec()
    };
    match p.algorithm.as_str() {
        "ed25519" => {
            use ed25519_dalek::{SigningKey, Signer};
            let key_bytes = hex::decode(&p.private_key)
                .map_err(|e| HapError::invalid_param(format!("key hex: {e}")))?;
            let sk = SigningKey::from_bytes(
                key_bytes.as_slice().try_into()
                    .map_err(|_| HapError::invalid_param("ed25519 key must be 32 bytes"))?
            );
            let sig = sk.sign(&data_bytes);
            if is_hex {
                Ok(json!(hex::encode(sig.to_bytes())))
            } else {
                Ok(json!(base64::engine::general_purpose::STANDARD.encode(sig.to_bytes())))
            }
        }
        "rsa-sha256" => {
            use rsa::{RsaPrivateKey, pkcs8::DecodePrivateKey, Pkcs1v15Sign};
            use sha2::{Sha256, Digest};
            let sk = RsaPrivateKey::from_pkcs8_pem(&p.private_key)
                .map_err(|e| HapError::invalid_param(format!("RSA private key parse failed: {e}")))?;
            let digest = Sha256::digest(&data_bytes);
            let sig = sk.sign(Pkcs1v15Sign::new::<Sha256>(), &digest)
                .map_err(|e| HapError::internal(format!("signing failed: {e}")))?;
            if is_hex {
                Ok(json!(hex::encode(&sig)))
            } else {
                Ok(json!(base64::engine::general_purpose::STANDARD.encode(&sig)))
            }
        }
        _ => Err(HapError::invalid_param(format!("unsupported: {}", p.algorithm))),
    }
});

// ---------- verify ----------
#[derive(Deserialize)]
pub struct VerifyParams { pub algorithm: String, pub public_key: String, pub data: String, pub signature: String, pub encoding: Option<String> }
hap_fn!(hap_crypto_verify, VerifyParams, |p| {
    let is_hex = p.encoding.as_deref() == Some("hex");
    let sig_bytes = if is_hex {
        hex::decode(&p.signature).map_err(|e| HapError::invalid_param(format!("signature hex: {e}")))?
    } else {
        base64::engine::general_purpose::STANDARD.decode(&p.signature)
            .map_err(|e| HapError::invalid_param(format!("signature base64: {e}")))?
    };
    let data_bytes = if is_hex {
        hex::decode(&p.data).map_err(|e| HapError::invalid_param(format!("data hex: {e}")))?
    } else {
        p.data.as_bytes().to_vec()
    };
    match p.algorithm.as_str() {
        "ed25519" => {
            use ed25519_dalek::{VerifyingKey, Verifier, Signature};
            let pk_bytes = hex::decode(&p.public_key)
                .map_err(|e| HapError::invalid_param(format!("key hex: {e}")))?;
            let pk = VerifyingKey::from_bytes(
                pk_bytes.as_slice().try_into()
                    .map_err(|_| HapError::invalid_param("ed25519 public key must be 32 bytes"))?
            ).map_err(|e| HapError::invalid_param(format!("invalid public key: {e}")))?;
            let sig = Signature::from_bytes(
                sig_bytes.as_slice().try_into()
                    .map_err(|_| HapError::invalid_param("signature must be 64 bytes"))?
            );
            Ok(json!(pk.verify(&data_bytes, &sig).is_ok()))
        }
        "rsa-sha256" => {
            use rsa::{RsaPublicKey, pkcs8::DecodePublicKey, Pkcs1v15Sign};
            use sha2::{Sha256, Digest};
            let pk = RsaPublicKey::from_public_key_pem(&p.public_key)
                .map_err(|e| HapError::invalid_param(format!("RSA public key parse failed: {e}")))?;
            let digest = Sha256::digest(&data_bytes);
            Ok(json!(pk.verify(Pkcs1v15Sign::new::<Sha256>(), &digest, &sig_bytes).is_ok()))
        }
        _ => Err(HapError::invalid_param(format!("unsupported: {}", p.algorithm))),
    }
});

// ---------- rsa_encrypt ----------
#[derive(Deserialize)]
pub struct RsaEncParams { pub public_key: String, pub data: String, pub padding: Option<String> }
hap_fn!(hap_crypto_rsa_encrypt, RsaEncParams, |p| {
    use rsa::{RsaPublicKey, pkcs8::DecodePublicKey, Oaep, Pkcs1v15Encrypt};
    let pk = RsaPublicKey::from_public_key_pem(&p.public_key)
        .map_err(|e| HapError::invalid_param(format!("RSA public key parse failed: {e}")))?;
    let mut rng = rand::rngs::OsRng;
    let ct = match p.padding.as_deref().unwrap_or("oaep") {
        "pkcs1" => pk.encrypt(&mut rng, Pkcs1v15Encrypt, p.data.as_bytes()),
        _ => pk.encrypt(&mut rng, Oaep::new::<sha2::Sha256>(), p.data.as_bytes()),
    }.map_err(|e| HapError::internal(format!("RSA encryption failed: {e}")))?;
    Ok(json!(base64::engine::general_purpose::STANDARD.encode(&ct)))
});

// ---------- rsa_decrypt ----------
#[derive(Deserialize)]
pub struct RsaDecParams { pub private_key: String, pub data: String, pub padding: Option<String> }
hap_fn!(hap_crypto_rsa_decrypt, RsaDecParams, |p| {
    use rsa::{RsaPrivateKey, pkcs8::DecodePrivateKey, Oaep, Pkcs1v15Encrypt};
    let sk = RsaPrivateKey::from_pkcs8_pem(&p.private_key)
        .map_err(|e| HapError::invalid_param(format!("RSA private key parse failed: {e}")))?;
    let ct = base64::engine::general_purpose::STANDARD.decode(&p.data)
        .map_err(|e| HapError::invalid_param(format!("base64: {e}")))?;
    let pt = match p.padding.as_deref().unwrap_or("oaep") {
        "pkcs1" => sk.decrypt(Pkcs1v15Encrypt, &ct),
        _ => sk.decrypt(Oaep::new::<sha2::Sha256>(), &ct),
    }.map_err(|e| HapError::internal(format!("RSA decryption failed: {e}")))?;
    Ok(json!(String::from_utf8_lossy(&pt).into_owned()))
});
