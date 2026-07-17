use hap_common::hap_fn;
use serde::Deserialize;
use serde_json::{json, Value};

fn next_tid() -> String { format!("tray_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()) }

#[derive(Deserialize)]
pub struct CreateParams { #[allow(dead_code)] pub icon_path: String, #[allow(dead_code)] pub tooltip: Option<String>, #[allow(dead_code)] pub callback_id: String }
hap_fn!(hap_tray_create, CreateParams, |_p| { Ok(json!({"tray_id": next_tid()})) });

#[derive(Deserialize)]
pub struct TrayIdParams { pub tray_id: String }

#[derive(Deserialize)]
pub struct SetIconParams { #[allow(dead_code)] pub tray_id: String, #[allow(dead_code)] pub icon_path: String }
hap_fn!(hap_tray_set_icon, SetIconParams, |_p| { Ok(json!(true)) });

#[derive(Deserialize)]
pub struct SetTooltipParams { #[allow(dead_code)] pub tray_id: String, #[allow(dead_code)] pub text: String }
hap_fn!(hap_tray_set_tooltip, SetTooltipParams, |_p| { Ok(json!(true)) });

#[derive(Deserialize)]
pub struct SetMenuParams { #[allow(dead_code)] pub tray_id: String, #[allow(dead_code)] pub items: Vec<Value> }
hap_fn!(hap_tray_set_menu, SetMenuParams, |_p| { Ok(json!(true)) });

#[derive(Deserialize)]
pub struct UpdateMenuItemParams { #[allow(dead_code)] pub tray_id: String, #[allow(dead_code)] pub item_id: String, #[allow(dead_code)] pub updates: Value }
hap_fn!(hap_tray_update_menu_item, UpdateMenuItemParams, |_p| { Ok(json!(true)) });

#[derive(Deserialize)]
pub struct DestroyParams { #[allow(dead_code)] pub tray_id: String }
hap_fn!(hap_tray_destroy, DestroyParams, |_p| { Ok(json!(true)) });

#[derive(Deserialize)]
pub struct ShowBalloonParams {
    #[allow(dead_code)] pub tray_id: String, #[allow(dead_code)] pub title: String,
    #[allow(dead_code)] pub message: String, #[allow(dead_code)] pub icon_type: Option<String>,
    #[allow(dead_code)] pub timeout_ms: Option<u32>,
}
hap_fn!(hap_tray_show_balloon, ShowBalloonParams, |_p| { Ok(json!(true)) });

#[derive(Deserialize)]
pub struct SetTitleParams { #[allow(dead_code)] pub tray_id: String, #[allow(dead_code)] pub title: String }
hap_fn!(hap_tray_set_title, SetTitleParams, |_p| { Ok(json!(true)) });

#[derive(Deserialize)]
pub struct SetVisibleParams { #[allow(dead_code)] pub tray_id: String, #[allow(dead_code)] pub visible: bool }
hap_fn!(hap_tray_set_visible, SetVisibleParams, |_p| { Ok(json!(true)) });

#[derive(Deserialize)]
pub struct SetBlinkParams { #[allow(dead_code)] pub tray_id: String, #[allow(dead_code)] pub blink: bool }
hap_fn!(hap_tray_set_blink, SetBlinkParams, |_p| { Ok(json!(true)) });

#[derive(Deserialize)]
pub struct SetBadgeParams { #[allow(dead_code)] pub tray_id: String, #[allow(dead_code)] pub count: i32 }
hap_fn!(hap_tray_set_badge, SetBadgeParams, |_p| { Ok(json!(true)) });

hap_fn!(hap_tray_list, Value, |_p| { Ok(json!([])) });
