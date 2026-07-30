use hap_common::hap_fn;
use hap_common::HapError;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::LazyLock;

#[cfg(target_os = "macos")]
use crate::macos;

static TRAYS: LazyLock<Mutex<HashMap<String, TrayState>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static TRAY_COUNTER: LazyLock<Mutex<u64>> = LazyLock::new(|| Mutex::new(0));
static MENU_EVENTS: LazyLock<Mutex<Vec<String>>> = LazyLock::new(|| Mutex::new(Vec::new()));
static TRAY_CALLERS: LazyLock<Mutex<HashMap<String, String>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

struct TrayState {
    #[cfg(target_os = "macos")]
    item: *mut objc::runtime::Object,
}
unsafe impl Send for TrayState {}

fn next_tid() -> String {
    let mut c = TRAY_COUNTER.lock().unwrap();
    *c += 1;
    format!("tray_{c}")
}

#[derive(Deserialize)]
pub struct CreateParams {
    pub icon_path: Option<String>,
    pub tooltip: Option<String>,
    pub callback_id: Option<String>,
    #[serde(rename = "_caller")]
    pub caller: Option<String>,
}
hap_fn!(hap_tray_create, CreateParams, |p| {
    #[cfg(target_os = "macos")]
    {
        let item = macos::create_status_item(
            p.icon_path.as_deref().unwrap_or(""),
            p.tooltip.as_deref().unwrap_or(""),
        )?;
        let tid = next_tid();
        TRAYS.lock().unwrap().insert(tid.clone(), TrayState { item });
        if let Some(ref caller) = p.caller {
            TRAY_CALLERS.lock().unwrap().insert(tid.clone(), caller.clone());
        }
        Ok(json!({"tray_id": tid}))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = p;
        Err(HapError::internal("tray not supported on this platform"))
    }
});

#[derive(Deserialize)]
pub struct SetIconParams { pub tray_id: String, pub icon_path: String }
hap_fn!(hap_tray_set_icon, SetIconParams, |p| {
    #[cfg(target_os = "macos")]
    {
        let trays = TRAYS.lock().unwrap();
        let t = trays.get(&p.tray_id).ok_or_else(|| HapError::internal("tray not found"))?;
        macos::set_icon(t.item, &p.icon_path)?;
        Ok(json!(true))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = p;
        Ok(json!(true))
    }
});

#[derive(Deserialize)]
pub struct SetTooltipParams { pub tray_id: String, pub tooltip: String }
hap_fn!(hap_tray_set_tooltip, SetTooltipParams, |p| {
    #[cfg(target_os = "macos")]
    {
        let trays = TRAYS.lock().unwrap();
        let t = trays.get(&p.tray_id).ok_or_else(|| HapError::internal("tray not found"))?;
        macos::set_tooltip(t.item, &p.tooltip);
        Ok(json!(true))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = p;
        Ok(json!(true))
    }
});

#[derive(Deserialize)]
pub struct SetMenuParams { pub tray_id: String, pub items: Vec<Value> }
hap_fn!(hap_tray_set_menu, SetMenuParams, |p| {
    #[cfg(target_os = "macos")]
    {
        let trays = TRAYS.lock().unwrap();
        let t = trays.get(&p.tray_id).ok_or_else(|| HapError::internal("tray not found"))?;
        macos::set_menu(t.item, &p.items);
        Ok(json!(true))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = p;
        Ok(json!(true))
    }
});

#[derive(Deserialize)]
pub struct UpdateMenuItemParams { pub tray_id: String, pub item_id: String, pub updates: Value }
hap_fn!(hap_tray_update_menu_item, UpdateMenuItemParams, |_p| {
    Ok(json!(true))
});

#[derive(Deserialize)]
pub struct DestroyParams { pub tray_id: String }
hap_fn!(hap_tray_destroy, DestroyParams, |p| {
    #[cfg(target_os = "macos")]
    {
        let mut trays = TRAYS.lock().unwrap();
        if let Some(t) = trays.remove(&p.tray_id) {
            macos::remove_status_item(t.item);
        }
    }
    #[cfg(not(target_os = "macos"))]
    { let _ = p; }
    TRAY_CALLERS.lock().unwrap().remove(&p.tray_id);
    Ok(json!(true))
});

#[derive(Deserialize)]
pub struct ShowBalloonParams {
    pub tray_id: String, pub title: String,
    pub message: String, pub icon_type: Option<String>,
    pub timeout_ms: Option<u32>,
}
hap_fn!(hap_tray_show_balloon, ShowBalloonParams, |_p| {
    Ok(json!(true))
});

#[derive(Deserialize)]
pub struct SetTitleParams { pub tray_id: String, pub title: String }
hap_fn!(hap_tray_set_title, SetTitleParams, |p| {
    #[cfg(target_os = "macos")]
    {
        let trays = TRAYS.lock().unwrap();
        let t = trays.get(&p.tray_id).ok_or_else(|| HapError::internal("tray not found"))?;
        macos::set_title(t.item, &p.title);
        Ok(json!(true))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = p;
        Ok(json!(true))
    }
});

#[derive(Deserialize)]
pub struct SetVisibleParams { pub tray_id: String, pub visible: bool }
hap_fn!(hap_tray_set_visible, SetVisibleParams, |p| {
    #[cfg(target_os = "macos")]
    {
        let trays = TRAYS.lock().unwrap();
        let t = trays.get(&p.tray_id).ok_or_else(|| HapError::internal("tray not found"))?;
        macos::set_visible(t.item, p.visible);
        Ok(json!(true))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = p;
        Ok(json!(true))
    }
});

#[derive(Deserialize)]
pub struct SetBlinkParams { pub tray_id: String, pub blink: bool }
hap_fn!(hap_tray_set_blink, SetBlinkParams, |_p| {
    Ok(json!(true))
});

#[derive(Deserialize)]
pub struct SetBadgeParams { pub tray_id: String, pub count: i32 }
hap_fn!(hap_tray_set_badge, SetBadgeParams, |_p| {
    Ok(json!(true))
});

hap_fn!(hap_tray_list, Value, |_p| {
    let trays = TRAYS.lock().unwrap();
    let list: Vec<Value> = trays.keys().map(|k| json!({"tray_id": k})).collect();
    Ok(json!(list))
});

hap_fn!(hap_tray_poll_events, Value, |_p| {
    let mut events = MENU_EVENTS.lock().unwrap();
    let result: Vec<String> = events.drain(..).collect();
    Ok(json!(result))
});

pub fn push_menu_event(item_id: String) {
    MENU_EVENTS.lock().unwrap().push(item_id);
}

pub fn get_caller_for_item(item_ptr: usize) -> Option<String> {
    let trays = TRAYS.lock().unwrap();
    let callers = TRAY_CALLERS.lock().unwrap();
    for (tid, state) in trays.iter() {
        #[cfg(target_os = "macos")]
        if state.item as usize == item_ptr {
            return callers.get(tid).cloned();
        }
    }
    callers.values().next().cloned()
}

pub fn emit_tray_event_for(item_ptr: usize, json: &str) {
    if let Some(caller) = get_caller_for_item(item_ptr) {
        let target = format!("tray-event@{}", caller);
        hap_common::context::emit_callback(&target, json);
    } else {
        hap_common::context::emit_callback("tray-event", json);
    }
}

pub fn emit_tray_event(json: &str) {
    let caller = TRAY_CALLERS.lock().unwrap().values().next().cloned();
    if let Some(caller) = caller {
        let target = format!("tray-event@{}", caller);
        hap_common::context::emit_callback(&target, json);
    } else {
        hap_common::context::emit_callback("tray-event", json);
    }
}
