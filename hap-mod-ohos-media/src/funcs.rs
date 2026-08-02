use hap_common::{hap_fn, HapError};
use serde_json::{json, Value};

hap_fn!(hap_ohos_media_pick_photo, Value, |params| {
    let max_count = params.get("max_count").and_then(|v| v.as_u64()).unwrap_or(1);
    let mime_type = params.get("mime_type").and_then(|v| v.as_str()).unwrap_or("image/*");
    Ok(json!({
        "action": "ohos.want.action.photoPicker",
        "max_count": max_count,
        "mime_type": mime_type,
        "delegate": "arkts"
    }))
});

hap_fn!(hap_ohos_media_pick_video, Value, |params| {
    let max_count = params.get("max_count").and_then(|v| v.as_u64()).unwrap_or(1);
    Ok(json!({
        "action": "ohos.want.action.videoPicker",
        "max_count": max_count,
        "delegate": "arkts"
    }))
});

hap_fn!(hap_ohos_media_save_to_gallery, Value, |params| {
    let source = params.get("source_path").and_then(|v| v.as_str())
        .ok_or_else(|| HapError::invalid_param("source_path required"))?;
    let display_name = params.get("display_name").and_then(|v| v.as_str())
        .ok_or_else(|| HapError::invalid_param("display_name required"))?;
    Ok(json!({
        "action": "photoAccessHelper.createAsset",
        "source_path": source,
        "display_name": display_name,
        "delegate": "arkts"
    }))
});

hap_fn!(hap_ohos_media_play_audio, Value, |params| {
    let path = params.get("path").and_then(|v| v.as_str())
        .ok_or_else(|| HapError::invalid_param("path required"))?;
    Ok(json!({
        "action": "audioRenderer.play",
        "path": path,
        "delegate": "arkts"
    }))
});

hap_fn!(hap_ohos_media_stop_audio, Value, |_params| {
    Ok(json!({ "action": "audioRenderer.stop", "delegate": "arkts" }))
});

hap_fn!(hap_ohos_media_get_volume, Value, |_params| {
    Ok(json!({ "action": "audioVolumeManager.getVolume", "delegate": "arkts" }))
});

hap_fn!(hap_ohos_media_capture_image, Value, |params| {
    let quality = params.get("quality").and_then(|v| v.as_u64()).unwrap_or(90);
    Ok(json!({
        "action": "cameraPicker.capture",
        "quality": quality,
        "delegate": "arkts"
    }))
});

hap_fn!(hap_ohos_media_record_audio, Value, |params| {
    let output = params.get("output_path").and_then(|v| v.as_str())
        .ok_or_else(|| HapError::invalid_param("output_path required"))?;
    let format = params.get("format").and_then(|v| v.as_str()).unwrap_or("m4a");
    Ok(json!({
        "action": "audioCapturer.start",
        "output_path": output,
        "format": format,
        "delegate": "arkts"
    }))
});

hap_fn!(hap_ohos_media_stop_recording, Value, |_params| {
    Ok(json!({ "action": "audioCapturer.stop", "delegate": "arkts" }))
});

hap_fn!(hap_ohos_media_get_media_info, Value, |params| {
    let path = params.get("path").and_then(|v| v.as_str())
        .ok_or_else(|| HapError::invalid_param("path required"))?;
    Ok(json!({
        "action": "photoAccessHelper.getFileInfo",
        "path": path,
        "delegate": "arkts"
    }))
});
