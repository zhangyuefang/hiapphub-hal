use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use rusb::{DeviceHandle, GlobalContext};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

static HANDLES: OnceLock<Mutex<HashMap<String, UsbHandle>>> = OnceLock::new();

struct UsbHandle {
    handle: DeviceHandle<GlobalContext>,
    interface_number: u8,
    claimed: bool,
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
    let devices = rusb::devices()
        .map_err(|e| HapError::internal(format!("usb enumerate: {e}")))?;
    for device in devices.iter() {
        if let Ok(desc) = device.device_descriptor() {
            if let Some(vid) = params.vendor_id {
                if desc.vendor_id() != vid as u16 { continue; }
            }
            if let Some(pid) = params.product_id {
                if desc.product_id() != pid as u16 { continue; }
            }
            let (manufacturer, product) = device.open()
                .map(|h| (
                    h.read_manufacturer_string_ascii(&desc).unwrap_or_default(),
                    h.read_product_string_ascii(&desc).unwrap_or_default(),
                ))
                .unwrap_or_default();
            result.push(json!({
                "vendor_id": desc.vendor_id(),
                "product_id": desc.product_id(),
                "bus": device.bus_number(),
                "address": device.address(),
                "class": desc.class_code(),
                "manufacturer": manufacturer,
                "product": product,
            }));
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
    let vid = params.vendor_id as u16;
    let pid = params.product_id as u16;
    let iface = params.interface_number.unwrap_or(0) as u8;

    let handle = rusb::open_device_with_vid_pid(vid, pid)
        .ok_or_else(|| HapError::internal(format!("device {:04x}:{:04x} not found or cannot open", vid, pid)))?;

    if handle.kernel_driver_active(iface).unwrap_or(false) {
        handle.detach_kernel_driver(iface)
            .map_err(|e| HapError::internal(format!("detach kernel driver: {e}")))?;
    }
    handle.claim_interface(iface)
        .map_err(|e| HapError::internal(format!("claim interface {}: {e}", iface)))?;

    let id = format!("usb_{}", NEXT_ID.fetch_add(1, Ordering::Relaxed));
    handles().lock().unwrap().insert(id.clone(), UsbHandle {
        handle,
        interface_number: iface,
        claimed: true,
    });
    Ok(json!({ "handle_id": id, "vendor_id": params.vendor_id, "product_id": params.product_id, "interface": iface }))
});

#[derive(Deserialize)]
#[allow(dead_code)]
struct HandleIdParams {
    handle_id: String,
}

hap_fn!(hap_usb_close, HandleIdParams, |params| {
    let mut map = handles().lock().unwrap();
    if let Some(h) = map.remove(&params.handle_id) {
        if h.claimed {
            let _ = h.handle.release_interface(h.interface_number);
        }
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
    let h = map.get(&params.handle_id)
        .ok_or_else(|| HapError::invalid_param("handle_id not found"))?;

    let bytes = hex::decode(&params.data)
        .map_err(|e| HapError::invalid_param(format!("invalid hex data: {e}")))?;
    let timeout = Duration::from_millis(params.timeout_ms.unwrap_or(5000).max(0) as u64);
    let endpoint = params.endpoint as u8;

    let written = h.handle.write_bulk(endpoint, &bytes, timeout)
        .map_err(|e| HapError::internal(format!("bulk write: {e}")))?;

    Ok(json!({ "bytes_written": written }))
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
    let h = map.get(&params.handle_id)
        .ok_or_else(|| HapError::invalid_param("handle_id not found"))?;

    let timeout = Duration::from_millis(params.timeout_ms.unwrap_or(5000).max(0) as u64);
    let endpoint = params.endpoint as u8;
    let mut buf = vec![0u8; params.length.max(0) as usize];

    let read = h.handle.read_bulk(endpoint, &mut buf, timeout)
        .map_err(|e| HapError::internal(format!("bulk read: {e}")))?;

    buf.truncate(read);
    Ok(json!({ "data": hex::encode(&buf), "bytes_read": read }))
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
    let h = map.get(&params.handle_id)
        .ok_or_else(|| HapError::invalid_param("handle_id not found"))?;

    let timeout = Duration::from_millis(params.timeout_ms.unwrap_or(5000).max(0) as u64);
    let request_type = params.request_type as u8;
    let request = params.request as u8;
    let value = params.value as u16;
    let index = params.index as u16;

    let is_in = (request_type & 0x80) != 0;

    if is_in {
        let mut buf = vec![0u8; 256];
        let read = h.handle.read_control(request_type, request, value, index, &mut buf, timeout)
            .map_err(|e| HapError::internal(format!("control read: {e}")))?;
        buf.truncate(read);
        Ok(json!({ "success": true, "data": hex::encode(&buf), "bytes_read": read }))
    } else {
        let data = params.data.as_deref().unwrap_or("");
        let bytes = hex::decode(data)
            .map_err(|e| HapError::invalid_param(format!("invalid hex data: {e}")))?;
        let written = h.handle.write_control(request_type, request, value, index, &bytes, timeout)
            .map_err(|e| HapError::internal(format!("control write: {e}")))?;
        Ok(json!({ "success": true, "bytes_written": written }))
    }
});

hap_fn!(hap_usb_get_device_info, HandleIdParams, |params| {
    let map = handles().lock().unwrap();
    let h = map.get(&params.handle_id)
        .ok_or_else(|| HapError::invalid_param("handle_id not found"))?;

    let device = h.handle.device();
    let desc = device.device_descriptor()
        .map_err(|e| HapError::internal(format!("get descriptor: {e}")))?;

    let manufacturer = h.handle.read_manufacturer_string_ascii(&desc).unwrap_or_default();
    let product = h.handle.read_product_string_ascii(&desc).unwrap_or_default();

    Ok(json!({
        "vendor_id": desc.vendor_id(),
        "product_id": desc.product_id(),
        "interface_number": h.interface_number,
        "manufacturer": manufacturer,
        "product": product,
        "class": desc.class_code(),
        "bus": device.bus_number(),
        "address": device.address(),
    }))
});

hap_fn!(hap_usb_reset_device, HandleIdParams, |params| {
    let map = handles().lock().unwrap();
    let h = map.get(&params.handle_id)
        .ok_or_else(|| HapError::invalid_param("handle_id not found"))?;

    h.handle.reset()
        .map_err(|e| HapError::internal(format!("reset device: {e}")))?;
    Ok(json!(true))
});
