use hap_common::{hap_fn, HapError};
use serde_json::{json, Value};

hap_fn!(hap_ohos_ability_start_ability, Value, |params| {
    let bundle = params.get("bundle_name").and_then(|v| v.as_str())
        .ok_or_else(|| HapError::invalid_param("bundle_name required"))?;
    let ability = params.get("ability_name").and_then(|v| v.as_str())
        .ok_or_else(|| HapError::invalid_param("ability_name required"))?;
    Ok(json!({
        "action": "context.startAbility",
        "want": { "bundleName": bundle, "abilityName": ability, "parameters": params.get("params") },
        "delegate": "arkts"
    }))
});

hap_fn!(hap_ohos_ability_get_context, Value, |_params| {
    Ok(json!({ "action": "context.getApplicationContext", "delegate": "arkts" }))
});

hap_fn!(hap_ohos_ability_get_app_info, Value, |_params| {
    Ok(json!({ "action": "bundleManager.getBundleInfoForSelf", "delegate": "arkts" }))
});

hap_fn!(hap_ohos_ability_request_permission, Value, |params| {
    let perm = params.get("permission").and_then(|v| v.as_str())
        .ok_or_else(|| HapError::invalid_param("permission required"))?;
    Ok(json!({
        "action": "abilityAccessCtrl.requestPermissionsFromUser",
        "permissions": [perm],
        "delegate": "arkts"
    }))
});

hap_fn!(hap_ohos_ability_check_permission, Value, |params| {
    let perm = params.get("permission").and_then(|v| v.as_str())
        .ok_or_else(|| HapError::invalid_param("permission required"))?;
    Ok(json!({
        "action": "abilityAccessCtrl.checkAccessToken",
        "permission": perm,
        "delegate": "arkts"
    }))
});

hap_fn!(hap_ohos_ability_show_toast, Value, |params| {
    let msg = params.get("message").and_then(|v| v.as_str())
        .ok_or_else(|| HapError::invalid_param("message required"))?;
    let duration = params.get("duration").and_then(|v| v.as_u64()).unwrap_or(2000);
    Ok(json!({
        "action": "promptAction.showToast",
        "message": msg,
        "duration": duration,
        "delegate": "arkts"
    }))
});

hap_fn!(hap_ohos_ability_show_dialog, Value, |params| {
    let title = params.get("title").and_then(|v| v.as_str())
        .ok_or_else(|| HapError::invalid_param("title required"))?;
    let message = params.get("message").and_then(|v| v.as_str())
        .ok_or_else(|| HapError::invalid_param("message required"))?;
    let buttons = params.get("buttons").cloned().unwrap_or(json!([{"text": "OK", "color": "#007DFF"}]));
    Ok(json!({
        "action": "promptAction.showDialog",
        "title": title,
        "message": message,
        "buttons": buttons,
        "delegate": "arkts"
    }))
});

hap_fn!(hap_ohos_ability_set_brightness, Value, |params| {
    let value = params.get("value").and_then(|v| v.as_f64())
        .ok_or_else(|| HapError::invalid_param("value required (0-255)"))?;
    Ok(json!({
        "action": "brightness.setValue",
        "value": value,
        "delegate": "arkts"
    }))
});

hap_fn!(hap_ohos_ability_vibrate, Value, |params| {
    let duration = params.get("duration").and_then(|v| v.as_u64()).unwrap_or(200);
    Ok(json!({
        "action": "vibrator.startVibration",
        "duration": duration,
        "delegate": "arkts"
    }))
});

hap_fn!(hap_ohos_ability_get_display_info, Value, |_params| {
    Ok(json!({ "action": "display.getDefaultDisplaySync", "delegate": "arkts" }))
});
