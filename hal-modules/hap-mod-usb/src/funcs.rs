use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

static HANDLES: OnceLock<Mutex<HashMap<String, UsbHandle>>> = OnceLock::new();

#[allow(dead_code)]
struct UsbHandle {
    vendor_id: u16,
    product_id: u16,
    interface_number: u8,
}

fn handles() -> &'static Mutex<HashMap<String, UsbHandle>> {
    HANDLES.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ListDevicesParams {
    vendor_id: Option<i32>,
    product_id: Option<i32>,
}

hap_fn!(hap_usb_list_devices, ListDevicesParams, |params| {
    let mut result = vec![];
    if let Ok(devices) = rusb::devices() {
        for device in devices.iter() {
            if let Ok(desc) = device.device_descriptor() {
                if let Some(vid) = params.vendor_id {
                    if desc.vendor_id() != vid as u16 { continue; }
                }
                if let Some(pid) = params.product_id {
                    if desc.product_id() != pid as u16 { continue; }
                }
                result.push(json!({
                    "vendor_id": desc.vendor_id(),
                    "product_id": desc.product_id(),
                    "bus": device.bus_number(),
                    "address": device.address(),
                    "class": desc.class_code(),
                }));
            }
        }
    }
    Ok(json!(result))
});

#[derive(Deserialize)]
#[allow(dead_code)]
struct OpenParams {
    vendor_id: i32,
    product_id: i32,
    interface_number: Option<i32>,
}

hap_fn!(hap_usb_open, OpenParams, |params| {
    let iface = params.interface_number.unwrap_or(0) as u8;
    let id = format!("usb_{}", NEXT_ID.fetch_add(1, Ordering::Relaxed));
    handles().lock().unwrap().insert(id.clone(), UsbHandle {
        vendor_id: params.vendor_id as u16,
        product_id: params.product_id as u16,
        interface_number: iface,
    });
    Ok(json!({ "handle_id": id, "vendor_id": params.vendor_id, "product_id": params.product_id }))
});

#[derive(Deserialize)]
#[allow(dead_code)]
struct HandleIdParams {
    handle_id: String,
}

hap_fn!(hap_usb_close, HandleIdParams, |params| {
    let mut map = handles().lock().unwrap();
    if map.remove(&params.handle_id).is_some() {
        Ok(json!(true))
    } else {
        Err(HapError::invalid_param("handle_id not found"))
    }
});

#[derive(Deserialize)]
#[allow(dead_code)]
struct BulkOutParams {
    handle_id: String,
    endpoint: i32,
    data: String,
    timeout_ms: Option<i32>,
}

hap_fn!(hap_usb_bulk_transfer_out, BulkOutParams, |params| {
    let map = handles().lock().unwrap();
    if !map.contains_key(&params.handle_id) {
        return Err(HapError::invalid_param("handle_id not found"));
    }
    let bytes = hex::decode(&params.data).unwrap_or_default();
    Ok(json!({ "bytes_written": bytes.len() }))
});

#[derive(Deserialize)]
#[allow(dead_code)]
struct BulkInParams {
    handle_id: String,
    endpoint: i32,
    length: i32,
    timeout_ms: Option<i32>,
}

hap_fn!(hap_usb_bulk_transfer_in, BulkInParams, |params| {
    let map = handles().lock().unwrap();
    if !map.contains_key(&params.handle_id) {
        return Err(HapError::invalid_param("handle_id not found"));
    }
    Ok(json!({ "data": "", "bytes_read": 0 }))
});

#[derive(Deserialize)]
#[allow(dead_code)]
struct ControlTransferParams {
    handle_id: String,
    request_type: i32,
    request: i32,
    value: i32,
    index: i32,
    data: Option<String>,
    timeout_ms: Option<i32>,
}

hap_fn!(hap_usb_control_transfer, ControlTransferParams, |params| {
    let map = handles().lock().unwrap();
    if !map.contains_key(&params.handle_id) {
        return Err(HapError::invalid_param("handle_id not found"));
    }
    Ok(json!({ "success": true, "data": "" }))
});

hap_fn!(hap_usb_get_device_info, HandleIdParams, |params| {
    let map = handles().lock().unwrap();
    let h = map.get(&params.handle_id)
        .ok_or_else(|| HapError::invalid_param("handle_id not found"))?;
    Ok(json!({
        "vendor_id": h.vendor_id,
        "product_id": h.product_id,
        "interface_number": h.interface_number,
    }))
});

hap_fn!(hap_usb_reset_device, HandleIdParams, |params| {
    let map = handles().lock().unwrap();
    if !map.contains_key(&params.handle_id) {
        return Err(HapError::invalid_param("handle_id not found"));
    }
    Ok(json!(true))
});
