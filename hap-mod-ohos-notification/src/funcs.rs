use hap_common::{hap_fn, HapError};
use serde_json::{json, Value};

hap_fn!(hap_ohos_notification_publish, Value, |params| {
    let title = params.get("title").and_then(|v| v.as_str())
        .ok_or_else(|| HapError::invalid_param("title required"))?;
    let text = params.get("text").and_then(|v| v.as_str())
        .ok_or_else(|| HapError::invalid_param("text required"))?;
    let id = params.get("id").and_then(|v| v.as_u64()).unwrap_or(1);
    Ok(json!({
        "action": "notificationManager.publish",
        "id": id,
        "content": { "title": title, "text": text },
        "delegate": "arkts"
    }))
});

hap_fn!(hap_ohos_notification_cancel, Value, |params| {
    let id = params.get("id").and_then(|v| v.as_u64())
        .ok_or_else(|| HapError::invalid_param("id required"))?;
    Ok(json!({ "action": "notificationManager.cancel", "id": id, "delegate": "arkts" }))
});

hap_fn!(hap_ohos_notification_cancel_all, Value, |_params| {
    Ok(json!({ "action": "notificationManager.cancelAll", "delegate": "arkts" }))
});

hap_fn!(hap_ohos_notification_get_active_count, Value, |_params| {
    Ok(json!({ "action": "notificationManager.getActiveNotificationCount", "delegate": "arkts" }))
});

hap_fn!(hap_ohos_notification_is_enabled, Value, |_params| {
    Ok(json!({ "action": "notificationManager.isNotificationEnabled", "delegate": "arkts" }))
});

hap_fn!(hap_ohos_notification_request_enable, Value, |_params| {
    Ok(json!({ "action": "notificationManager.requestEnableNotification", "delegate": "arkts" }))
});
