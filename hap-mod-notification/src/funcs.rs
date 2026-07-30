use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::{json, Value};

fn next_nid() -> String {
    format!("notif_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis())
}

#[derive(Deserialize)]
pub struct SendParams {
    pub title: String, pub body: String,
    #[allow(dead_code)] pub icon: Option<String>,
    #[allow(dead_code)] pub actions: Option<Vec<Value>>,
    #[allow(dead_code)] pub sound: Option<bool>,
    #[allow(dead_code)] pub silent: Option<bool>,
    #[allow(dead_code)] pub timeout_ms: Option<u32>,
    #[allow(dead_code)] pub callback_id: Option<String>,
}
hap_fn!(hap_notification_send, SendParams, |p| {
    notify_rust::Notification::new().summary(&p.title).body(&p.body).show()
        .map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!({"notification_id": next_nid()}))
});

#[derive(Deserialize)]
pub struct CloseParams { #[allow(dead_code)] pub notification_id: String }
hap_fn!(hap_notification_close, CloseParams, |_p| { Ok(json!(true)) });

hap_fn!(hap_notification_close_all, Value, |_p| { Ok(json!(true)) });
hap_fn!(hap_notification_request_permission, Value, |_p| { Ok(json!("granted")) });
hap_fn!(hap_notification_is_permitted, Value, |_p| { Ok(json!(true)) });

#[derive(Deserialize)]
pub struct SetBadgeParams { #[allow(dead_code)] pub count: i32 }
hap_fn!(hap_notification_set_badge, SetBadgeParams, |_p| { Ok(json!(true)) });

#[derive(Deserialize)]
pub struct ScheduleParams {
    pub title: String, pub body: String, #[allow(dead_code)] pub delay_ms: u32,
    #[allow(dead_code)] pub icon: Option<String>, #[allow(dead_code)] pub repeat: Option<bool>,
    #[allow(dead_code)] pub callback_id: Option<String>,
}
hap_fn!(hap_notification_schedule, ScheduleParams, |_p| {
    Ok(json!({"notification_id": next_nid()}))
});

#[derive(Deserialize)]
pub struct CancelScheduleParams { #[allow(dead_code)] pub notification_id: String }
hap_fn!(hap_notification_cancel_schedule, CancelScheduleParams, |_p| { Ok(json!(true)) });
