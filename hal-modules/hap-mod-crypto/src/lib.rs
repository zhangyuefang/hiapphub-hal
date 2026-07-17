mod hash_funcs;
mod sym_funcs;
mod asym_funcs;
mod util_funcs;

use hap_common::{hap_free_string, hap_module_init};

hap_module_init!("crypto");
hap_free_string!();

// ---------- hap_module_describe ----------
#[no_mangle]
pub extern "C" fn hap_module_describe() -> *const std::os::raw::c_char {
    hap_common::ffi::str_to_c(include_str!("../manifest.json"))
}

#[cfg(test)]
mod tests {
    use std::ffi::{CStr, CString};
    use serde_json::{json, Value};

    fn call(func: extern "C" fn(*const std::os::raw::c_char) -> *const std::os::raw::c_char, json_str: &str) -> Value {
        let cs = CString::new(json_str).unwrap();
        let result = func(cs.as_ptr());
        assert!(!result.is_null());
        let s = unsafe { CStr::from_ptr(result) }.to_str().unwrap().to_string();
        unsafe { super::hap_free_string(result as *mut _) };
        serde_json::from_str(&s).unwrap()
    }

    #[test]
    fn test_hash_sha256() {
        let r = call(super::hash_funcs::hap_crypto_hash, r#"{"algorithm":"sha256","data":"hello"}"#);
        assert_eq!(r.as_str().unwrap(), "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
    }

    #[test]
    fn test_hash_md5() {
        let r = call(super::hash_funcs::hap_crypto_hash, r#"{"algorithm":"md5","data":"hello"}"#);
        assert_eq!(r.as_str().unwrap(), "5d41402abc4b2a76b9719d911017c592");
    }

    #[test]
    fn test_hash_blake3() {
        let r = call(super::hash_funcs::hap_crypto_hash, r#"{"algorithm":"blake3","data":"hello"}"#);
        assert_eq!(r.as_str().unwrap().len(), 64);
    }

    #[test]
    fn test_hmac_sha256() {
        let r = call(super::hash_funcs::hap_crypto_hmac, r#"{"algorithm":"sha256","key":"secret","data":"hello"}"#);
        assert_eq!(r.as_str().unwrap().len(), 64);
    }

    #[test]
    fn test_crc32() {
        let r = call(super::hash_funcs::hap_crypto_crc32, r#"{"data":"hello"}"#);
        assert_eq!(r.as_str().unwrap(), "3610a686");
    }

    #[test]
    fn test_constant_time_eq() {
        assert_eq!(call(super::hash_funcs::hap_crypto_constant_time_eq, r#"{"a":"abc","b":"abc"}"#), json!(true));
        assert_eq!(call(super::hash_funcs::hap_crypto_constant_time_eq, r#"{"a":"abc","b":"abd"}"#), json!(false));
    }

    #[test]
    fn test_random_bytes() {
        let r = call(super::util_funcs::hap_crypto_random_bytes, r#"{"length":16}"#);
        assert_eq!(r.as_str().unwrap().len(), 32);
    }

    #[test]
    fn test_generate_uuid_v4() {
        let r = call(super::util_funcs::hap_crypto_generate_uuid, r#"{}"#);
        assert_eq!(r.as_str().unwrap().len(), 36);
    }

    #[test]
    fn test_generate_uuid_v7() {
        let r = call(super::util_funcs::hap_crypto_generate_uuid, r#"{"version":"v7"}"#);
        assert_eq!(r.as_str().unwrap().len(), 36);
    }

    #[test]
    fn test_random_string() {
        let r = call(super::util_funcs::hap_crypto_random_string, r#"{"length":20,"charset":"hex"}"#);
        assert_eq!(r.as_str().unwrap().len(), 20);
    }

    #[test]
    fn test_generate_password() {
        let r = call(super::util_funcs::hap_crypto_generate_password, r#"{"length":24}"#);
        assert_eq!(r.as_str().unwrap().len(), 24);
    }

    #[test]
    fn test_generate_key_aes256() {
        let r = call(super::sym_funcs::hap_crypto_generate_key, r#"{"algorithm":"aes-256"}"#);
        assert_eq!(r.as_str().unwrap().len(), 64);
    }

    #[test]
    fn test_encrypt_decrypt_aes_gcm() {
        let key = call(super::sym_funcs::hap_crypto_generate_key, r#"{"algorithm":"aes-256"}"#);
        let key_str = key.as_str().unwrap();
        let enc = call(super::sym_funcs::hap_crypto_encrypt, &format!(
            r#"{{"algorithm":"aes-256-gcm","key":"{key_str}","data":"secret message"}}"#
        ));
        let ct = enc["ciphertext"].as_str().unwrap();
        let iv = enc["iv"].as_str().unwrap();
        let dec = call(super::sym_funcs::hap_crypto_decrypt, &format!(
            r#"{{"algorithm":"aes-256-gcm","key":"{key_str}","ciphertext":"{ct}","iv":"{iv}"}}"#
        ));
        assert_eq!(dec.as_str().unwrap(), "secret message");
    }

    #[test]
    fn test_encrypt_decrypt_chacha20() {
        let key = call(super::sym_funcs::hap_crypto_generate_key, r#"{"algorithm":"chacha20"}"#);
        let key_str = key.as_str().unwrap();
        let enc = call(super::sym_funcs::hap_crypto_encrypt, &format!(
            r#"{{"algorithm":"chacha20-poly1305","key":"{key_str}","data":"hello chacha"}}"#
        ));
        let ct = enc["ciphertext"].as_str().unwrap();
        let iv = enc["iv"].as_str().unwrap();
        let dec = call(super::sym_funcs::hap_crypto_decrypt, &format!(
            r#"{{"algorithm":"chacha20-poly1305","key":"{key_str}","ciphertext":"{ct}","iv":"{iv}"}}"#
        ));
        assert_eq!(dec.as_str().unwrap(), "hello chacha");
    }

    #[test]
    fn test_derive_key_pbkdf2() {
        let r = call(super::sym_funcs::hap_crypto_derive_key, r#"{"password":"mypass","salt":"somesalt","algorithm":"pbkdf2","iterations":1000}"#);
        assert_eq!(r.as_str().unwrap().len(), 64);
    }

    #[test]
    fn test_bcrypt_roundtrip() {
        let hash = call(super::sym_funcs::hap_crypto_bcrypt_hash, r#"{"password":"test123","rounds":4}"#);
        let hash_str = hash.as_str().unwrap();
        let verify = call(super::sym_funcs::hap_crypto_bcrypt_verify, &format!(
            r#"{{"password":"test123","hash":"{hash_str}"}}"#
        ));
        assert_eq!(verify, json!(true));
        let verify_bad = call(super::sym_funcs::hap_crypto_bcrypt_verify, &format!(
            r#"{{"password":"wrong","hash":"{hash_str}"}}"#
        ));
        assert_eq!(verify_bad, json!(false));
    }

    #[test]
    fn test_encrypt_decrypt_with_password() {
        let enc = call(super::sym_funcs::hap_crypto_encrypt_with_password, r#"{"password":"mypassword","data":"sensitive data"}"#);
        let encrypted = enc.as_str().unwrap();
        let dec = call(super::sym_funcs::hap_crypto_decrypt_with_password, &format!(
            r#"{{"password":"mypassword","encrypted":"{encrypted}"}}"#
        ));
        assert_eq!(dec.as_str().unwrap(), "sensitive data");
    }

    #[test]
    fn test_ed25519_sign_verify() {
        let kp = call(super::asym_funcs::hap_crypto_generate_keypair, r#"{"algorithm":"ed25519"}"#);
        let sk = kp["private_key"].as_str().unwrap();
        let pk = kp["public_key"].as_str().unwrap();
        let sig = call(super::asym_funcs::hap_crypto_sign, &format!(
            r#"{{"algorithm":"ed25519","private_key":"{sk}","data":"hello"}}"#
        ));
        let sig_str = sig.as_str().unwrap();
        let valid = call(super::asym_funcs::hap_crypto_verify, &format!(
            r#"{{"algorithm":"ed25519","public_key":"{pk}","data":"hello","signature":"{sig_str}"}}"#
        ));
        assert_eq!(valid, json!(true));
    }

    #[test]
    fn test_totp_secret() {
        let r = call(super::util_funcs::hap_crypto_generate_totp_secret, r#"{"issuer":"TestApp","account":"user@test.com"}"#);
        assert!(r["secret"].as_str().unwrap().len() > 10);
        assert!(r["uri"].as_str().unwrap().starts_with("otpauth://totp/"));
    }

    #[test]
    fn test_totp_generate() {
        let r = call(super::util_funcs::hap_crypto_generate_totp, r#"{"secret":"JBSWY3DPEHPK3PXP"}"#);
        assert_eq!(r["code"].as_str().unwrap().len(), 6);
        assert!(r["remaining_seconds"].as_i64().unwrap() > 0);
    }

    #[test]
    fn test_pem_der_roundtrip() {
        // Create a simple PEM
        let pem_str = "-----BEGIN TEST-----\nSGVsbG8gV29ybGQ=\n-----END TEST-----\n";
        let escaped = pem_str.replace('\n', "\\n");
        let der = call(super::util_funcs::hap_crypto_pem_to_der, &format!(r#"{{"pem":"{escaped}"}}"#));
        let der_str = der.as_str().unwrap();
        let pem_back = call(super::util_funcs::hap_crypto_der_to_pem, &format!(
            r#"{{"der":"{der_str}","label":"TEST"}}"#
        ));
        assert!(pem_back.as_str().unwrap().contains("-----BEGIN TEST-----"));
    }

    #[test]
    fn test_hash_file() {
        let tmp = std::env::temp_dir().join("hap_crypto_hashfile.txt");
        std::fs::write(&tmp, "hello").unwrap();
        let path = tmp.to_string_lossy().replace('\\', "\\\\");
        let r = call(super::hash_funcs::hap_crypto_hash_file, &format!(r#"{{"algorithm":"sha256","path":"{path}"}}"#));
        assert_eq!(r.as_str().unwrap(), "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_rsa_encrypt_decrypt() {
        let kp = call(super::asym_funcs::hap_crypto_generate_keypair, r#"{"algorithm":"rsa-2048"}"#);
        let sk = kp["private_key"].as_str().unwrap();
        let pk = kp["public_key"].as_str().unwrap();
        let ct = call(super::asym_funcs::hap_crypto_rsa_encrypt, &format!(
            r#"{{"public_key":{},"data":"hello rsa"}}"#, serde_json::to_string(pk).unwrap()
        ));
        let ct_str = ct.as_str().unwrap();
        let pt = call(super::asym_funcs::hap_crypto_rsa_decrypt, &format!(
            r#"{{"private_key":{},"data":"{}"}}"#, serde_json::to_string(sk).unwrap(), ct_str
        ));
        assert_eq!(pt.as_str().unwrap(), "hello rsa");
    }

    #[test]
    fn test_encrypt_decrypt_file() {
        let key = call(super::sym_funcs::hap_crypto_generate_key, r#"{"algorithm":"aes-256"}"#);
        let key_str = key.as_str().unwrap();
        let tmp = std::env::temp_dir();
        let src = tmp.join("hap_crypto_encfile_src.txt");
        let enc = tmp.join("hap_crypto_encfile_enc.bin");
        let dec = tmp.join("hap_crypto_encfile_dec.txt");
        std::fs::write(&src, "file encryption test data").unwrap();
        let sp = src.to_string_lossy().replace('\\', "\\\\");
        let ep = enc.to_string_lossy().replace('\\', "\\\\");
        let dp = dec.to_string_lossy().replace('\\', "\\\\");
        let r = call(super::sym_funcs::hap_crypto_encrypt_file, &format!(
            r#"{{"path":"{sp}","output_path":"{ep}","key":"{key_str}"}}"#
        ));
        let iv = r["iv"].as_str().unwrap();
        let tag = r.get("tag").and_then(|v| v.as_str()).unwrap_or("");
        call(super::sym_funcs::hap_crypto_decrypt_file, &format!(
            r#"{{"path":"{ep}","output_path":"{dp}","key":"{key_str}","iv":"{iv}","tag":"{tag}"}}"#
        ));
        assert_eq!(std::fs::read_to_string(&dec).unwrap(), "file encryption test data");
        std::fs::remove_file(&src).ok();
        std::fs::remove_file(&enc).ok();
        std::fs::remove_file(&dec).ok();
    }

    #[test]
    fn test_rsa_sign_verify() {
        let kp = call(super::asym_funcs::hap_crypto_generate_keypair, r#"{"algorithm":"rsa-2048"}"#);
        let sk = kp["private_key"].as_str().unwrap();
        let pk = kp["public_key"].as_str().unwrap();
        let sig = call(super::asym_funcs::hap_crypto_sign, &format!(
            r#"{{"algorithm":"rsa-sha256","private_key":{},"data":"hello"}}"#, serde_json::to_string(sk).unwrap()
        ));
        let sig_str = sig.as_str().unwrap();
        let valid = call(super::asym_funcs::hap_crypto_verify, &format!(
            r#"{{"algorithm":"rsa-sha256","public_key":{},"data":"hello","signature":"{}"}}"#,
            serde_json::to_string(pk).unwrap(), sig_str
        ));
        assert_eq!(valid, json!(true));
    }

    #[test]
    fn test_x25519_keypair() {
        let kp = call(super::asym_funcs::hap_crypto_generate_keypair, r#"{"algorithm":"x25519"}"#);
        assert_eq!(kp["private_key"].as_str().unwrap().len(), 64);
        assert_eq!(kp["public_key"].as_str().unwrap().len(), 64);
    }

    #[test]
    fn test_describe() {
        let ptr = super::hap_module_describe();
        let s = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        let v: Value = serde_json::from_str(s).unwrap();
        assert_eq!(v["name"], "crypto");
        // encrypt_file + decrypt_file not yet impl, but described (29 functions listed, minus encrypt_file/decrypt_file = 27 tested)
        assert!(v["functions"].as_array().unwrap().len() >= 27);
        unsafe { super::hap_free_string(ptr as *mut _) };
    }
}
