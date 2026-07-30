use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::{json, Value};

// ---------- store ----------
#[derive(Deserialize)] pub struct StoreParams { pub service: String, pub account: String, pub password: String, #[allow(dead_code)] pub label: Option<String> }
hap_fn!(hap_keychain_store, StoreParams, |p| {
    let entry = keyring::Entry::new(&p.service, &p.account).map_err(|e| HapError::internal(e.to_string()))?;
    entry.set_password(&p.password).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

// ---------- retrieve ----------
#[derive(Deserialize)] pub struct RetrieveParams { pub service: String, pub account: String }
hap_fn!(hap_keychain_retrieve, RetrieveParams, |p| {
    let entry = keyring::Entry::new(&p.service, &p.account).map_err(|e| HapError::internal(e.to_string()))?;
    match entry.get_password() {
        Ok(pw) => Ok(json!(pw)),
        Err(_) => Ok(json!("")),
    }
});

// ---------- delete ----------
hap_fn!(hap_keychain_delete, RetrieveParams, |p| {
    let entry = keyring::Entry::new(&p.service, &p.account).map_err(|e| HapError::internal(e.to_string()))?;
    match entry.delete_credential() {
        Ok(_) => Ok(json!(true)),
        Err(_) => Ok(json!(false)),
    }
});

// ---------- has ----------
hap_fn!(hap_keychain_has, RetrieveParams, |p| {
    let entry = keyring::Entry::new(&p.service, &p.account).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(entry.get_password().is_ok()))
});

// ---------- list ----------
#[derive(Deserialize)] pub struct ServiceParams { pub service: String }
hap_fn!(hap_keychain_list, ServiceParams, |_p| {
    Ok(json!([]))
});

// ---------- update ----------
hap_fn!(hap_keychain_update, StoreParams, |p| {
    let entry = keyring::Entry::new(&p.service, &p.account).map_err(|e| HapError::internal(e.to_string()))?;
    entry.set_password(&p.password).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

// ---------- store_binary ----------
#[derive(Deserialize)] pub struct StoreBinaryParams { pub service: String, pub account: String, pub data: String }
hap_fn!(hap_keychain_store_binary, StoreBinaryParams, |p| {
    let entry = keyring::Entry::new(&p.service, &p.account).map_err(|e| HapError::internal(e.to_string()))?;
    entry.set_password(&p.data).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

// ---------- retrieve_binary ----------
hap_fn!(hap_keychain_retrieve_binary, RetrieveParams, |p| {
    let entry = keyring::Entry::new(&p.service, &p.account).map_err(|e| HapError::internal(e.to_string()))?;
    match entry.get_password() {
        Ok(data) => Ok(json!(data)),
        Err(_) => Ok(json!("")),
    }
});

// ---------- clear ----------
hap_fn!(hap_keychain_clear, ServiceParams, |_p| {
    Ok(json!(0))
});

// ---------- biometric ----------
hap_fn!(hap_keychain_biometric_available, Value, |_p| {
    Ok(json!({"available": false, "type": "none"}))
});

#[derive(Deserialize)] pub struct BiometricAuthParams { #[allow(dead_code)] pub reason: String }
hap_fn!(hap_keychain_biometric_authenticate, BiometricAuthParams, |_p| {
    Ok(json!(false))
});
