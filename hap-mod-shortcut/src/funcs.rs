use hap_common::hap_fn;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

static SHORTCUTS: LazyLock<Mutex<HashMap<String, String>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Deserialize)]
pub struct RegisterParams { pub accelerator: String, pub callback_id: String }
hap_fn!(hap_shortcut_register, RegisterParams, |p| {
    SHORTCUTS.lock().unwrap().insert(p.accelerator.clone(), p.callback_id.clone());
    Ok(json!(true))
});

#[derive(Deserialize)]
pub struct UnregisterParams { pub accelerator: String }
hap_fn!(hap_shortcut_unregister, UnregisterParams, |p| {
    SHORTCUTS.lock().unwrap().remove(&p.accelerator);
    Ok(json!(true))
});

hap_fn!(hap_shortcut_unregister_all, Value, |_p| {
    let mut map = SHORTCUTS.lock().unwrap();
    let count = map.len() as i32;
    map.clear();
    Ok(json!(count))
});

#[derive(Deserialize)]
pub struct IsRegisteredParams { pub accelerator: String }
hap_fn!(hap_shortcut_is_registered, IsRegisteredParams, |p| {
    Ok(json!(SHORTCUTS.lock().unwrap().contains_key(&p.accelerator)))
});

hap_fn!(hap_shortcut_list, Value, |_p| {
    let map = SHORTCUTS.lock().unwrap();
    let list: Vec<Value> = map.iter().map(|(acc, cb)| json!({"accelerator": acc, "callback_id": cb})).collect();
    Ok(json!(list))
});
