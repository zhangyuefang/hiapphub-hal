use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[allow(dead_code)]
struct BleDevice {
    address: String,
    connected: bool,
}

static DEVICES: OnceLock<Mutex<HashMap<String, BleDevice>>> = OnceLock::new();

fn devices() -> &'static Mutex<HashMap<String, BleDevice>> {
    DEVICES.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ScanStartParams {
    duration_ms: Option<i32>,
    filter_name: Option<String>,
}

hap_fn!(hap_bluetooth_scan_start, ScanStartParams, |params| {
    let _duration = params.duration_ms.unwrap_or(10000);
    Ok(json!([]))
});

#[derive(Deserialize)]
#[allow(dead_code)]
struct EmptyParams {}

hap_fn!(hap_bluetooth_scan_stop, EmptyParams, |_params| {
    Ok(json!(true))
});

#[derive(Deserialize)]
#[allow(dead_code)]
struct DeviceIdParams {
    device_id: String,
}

hap_fn!(hap_bluetooth_connect, DeviceIdParams, |params| {
    let id = params.device_id.clone();
    devices().lock().unwrap().insert(id.clone(), BleDevice {
        address: id.clone(),
        connected: true,
    });
    Ok(json!({ "device_id": id, "connected": true }))
});

hap_fn!(hap_bluetooth_disconnect, DeviceIdParams, |params| {
    let mut map = devices().lock().unwrap();
    if let Some(dev) = map.get_mut(&params.device_id) {
        dev.connected = false;
        Ok(json!(true))
    } else {
        Err(HapError::invalid_param("device not found"))
    }
});

hap_fn!(hap_bluetooth_discover_services, DeviceIdParams, |params| {
    let map = devices().lock().unwrap();
    if !map.contains_key(&params.device_id) {
        return Err(HapError::invalid_param("device not connected"));
    }
    Ok(json!([]))
});

#[derive(Deserialize)]
#[allow(dead_code)]
struct ReadCharParams {
    device_id: String,
    service_uuid: String,
    characteristic_uuid: String,
}

hap_fn!(hap_bluetooth_read_characteristic, ReadCharParams, |params| {
    let map = devices().lock().unwrap();
    if !map.contains_key(&params.device_id) {
        return Err(HapError::invalid_param("device not connected"));
    }
    Ok(json!({ "data": "", "service": params.service_uuid, "characteristic": params.characteristic_uuid }))
});

#[derive(Deserialize)]
#[allow(dead_code)]
struct WriteCharParams {
    device_id: String,
    service_uuid: String,
    characteristic_uuid: String,
    data: String,
    with_response: Option<bool>,
}

hap_fn!(hap_bluetooth_write_characteristic, WriteCharParams, |params| {
    let map = devices().lock().unwrap();
    if !map.contains_key(&params.device_id) {
        return Err(HapError::invalid_param("device not connected"));
    }
    Ok(json!(true))
});

#[derive(Deserialize)]
#[allow(dead_code)]
struct SubscribeParams {
    device_id: String,
    service_uuid: String,
    characteristic_uuid: String,
    callback_id: String,
}

hap_fn!(hap_bluetooth_subscribe, SubscribeParams, |params| {
    let map = devices().lock().unwrap();
    if !map.contains_key(&params.device_id) {
        return Err(HapError::invalid_param("device not connected"));
    }
    Ok(json!(true))
});

#[derive(Deserialize)]
#[allow(dead_code)]
struct UnsubscribeParams {
    device_id: String,
    service_uuid: String,
    characteristic_uuid: String,
}

hap_fn!(hap_bluetooth_unsubscribe, UnsubscribeParams, |_params| {
    Ok(json!(true))
});

hap_fn!(hap_bluetooth_is_connected, DeviceIdParams, |params| {
    let map = devices().lock().unwrap();
    let connected = map.get(&params.device_id).map(|d| d.connected).unwrap_or(false);
    Ok(json!(connected))
});
