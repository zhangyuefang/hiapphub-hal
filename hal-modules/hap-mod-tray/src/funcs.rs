use hap_common::hap_fn;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::LazyLock;
use tray_icon::{TrayIcon, TrayIconBuilder, Icon, menu::{Menu, MenuItem, PredefinedMenuItem}};

struct SendTray(TrayIcon);
unsafe impl Send for SendTray {}

static TRAYS: LazyLock<Mutex<HashMap<String, SendTray>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static TRAY_COUNTER: LazyLock<Mutex<u64>> = LazyLock::new(|| Mutex::new(0));

fn next_tid() -> String {
    let mut c = TRAY_COUNTER.lock().unwrap();
    *c += 1;
    format!("tray_{c}")
}

use hap_common::HapError;

fn load_icon(path: &str) -> Result<Icon, HapError> {
    let img = image::open(path).map_err(|e| HapError::internal(format!("load icon: {e}")))?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Icon::from_rgba(rgba.into_raw(), w, h).map_err(|e| HapError::internal(format!("icon convert: {e}")))
}

#[derive(Deserialize)]
pub struct CreateParams { pub icon_path: String, pub tooltip: Option<String>, pub callback_id: String }
hap_fn!(hap_tray_create, CreateParams, |p| {
    let icon = load_icon(&p.icon_path)?;
    let mut builder = TrayIconBuilder::new().with_icon(icon);
    if let Some(ref tip) = p.tooltip {
        builder = builder.with_tooltip(tip);
    }
    let tray = builder.build().map_err(|e| HapError::internal(format!("tray build: {e}")))?;
    let tid = next_tid();
    TRAYS.lock().unwrap().insert(tid.clone(), SendTray(tray));
    Ok(json!({"tray_id": tid}))
});

#[derive(Deserialize)]
pub struct SetIconParams { pub tray_id: String, pub icon_path: String }
hap_fn!(hap_tray_set_icon, SetIconParams, |p| {
    let icon = load_icon(&p.icon_path)?;
    let trays = TRAYS.lock().unwrap();
    let t = trays.get(&p.tray_id).ok_or_else(|| HapError::internal("tray not found"))?;
    t.0.set_icon(Some(icon)).map_err(|e| HapError::internal(format!("set icon: {e}")))?;
    Ok(json!(true))
});

#[derive(Deserialize)]
pub struct SetTooltipParams { pub tray_id: String, pub tooltip: String }
hap_fn!(hap_tray_set_tooltip, SetTooltipParams, |p| {
    let trays = TRAYS.lock().unwrap();
    let t = trays.get(&p.tray_id).ok_or_else(|| HapError::internal("tray not found"))?;
    t.0.set_tooltip(Some(&p.tooltip)).map_err(|e| HapError::internal(format!("set tooltip: {e}")))?;
    Ok(json!(true))
});

#[derive(Deserialize)]
pub struct SetMenuParams { pub tray_id: String, pub items: Vec<Value> }
hap_fn!(hap_tray_set_menu, SetMenuParams, |p| {
    let menu = Menu::new();
    for item in &p.items {
        let label = item["label"].as_str().unwrap_or("");
        if label == "-" {
            let _ = menu.append(&PredefinedMenuItem::separator());
        } else {
            let enabled = item["enabled"].as_bool().unwrap_or(true);
            let mi = MenuItem::new(label, enabled, None);
            let _ = menu.append(&mi);
        }
    }
    let trays = TRAYS.lock().unwrap();
    let t = trays.get(&p.tray_id).ok_or_else(|| HapError::internal("tray not found"))?;
    t.0.set_menu(Some(Box::new(menu)));
    Ok(json!(true))
});

#[derive(Deserialize)]
pub struct UpdateMenuItemParams { pub tray_id: String, pub item_id: String, pub updates: Value }
hap_fn!(hap_tray_update_menu_item, UpdateMenuItemParams, |_p| {
    Ok(json!(true))
});

#[derive(Deserialize)]
pub struct DestroyParams { pub tray_id: String }
hap_fn!(hap_tray_destroy, DestroyParams, |p| {
    TRAYS.lock().unwrap().remove(&p.tray_id);
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
    let trays = TRAYS.lock().unwrap();
    let t = trays.get(&p.tray_id).ok_or_else(|| HapError::internal("tray not found"))?;
    t.0.set_title(Some(&p.title));
    Ok(json!(true))
});

#[derive(Deserialize)]
pub struct SetVisibleParams { pub tray_id: String, pub visible: bool }
hap_fn!(hap_tray_set_visible, SetVisibleParams, |p| {
    let trays = TRAYS.lock().unwrap();
    let t = trays.get(&p.tray_id).ok_or_else(|| HapError::internal("tray not found"))?;
    t.0.set_visible(p.visible).map_err(|e| HapError::internal(format!("set visible: {e}")))?;
    Ok(json!(true))
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
