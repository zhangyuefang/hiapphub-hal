use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, atomic::{AtomicU64, Ordering}};
use std::io::{Read as _, Write as _};
use std::time::Duration;

struct PortEntry {
    port: Box<dyn serialport::SerialPort>,
    path: String,
    baud: u32,
}

unsafe impl Send for PortEntry {}

static PORTS: LazyLock<Mutex<HashMap<String, PortEntry>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static PORT_COUNTER: AtomicU64 = AtomicU64::new(1);
fn next_id() -> String { format!("serial_{}", PORT_COUNTER.fetch_add(1, Ordering::Relaxed)) }

// ---------- list_ports ----------
hap_fn!(hap_serial_list_ports, Value, |_p| {
    let ports = serialport::available_ports().map_err(|e| HapError::internal(e.to_string()))?;
    let list: Vec<Value> = ports.iter().map(|p| {
        let mut info = json!({"path": p.port_name, "type": "unknown"});
        if let serialport::SerialPortType::UsbPort(usb) = &p.port_type {
            info["type"] = json!("usb");
            info["vid"] = json!(usb.vid);
            info["pid"] = json!(usb.pid);
            if let Some(ref m) = usb.manufacturer { info["manufacturer"] = json!(m); }
            if let Some(ref pr) = usb.product { info["product"] = json!(pr); }
            if let Some(ref sn) = usb.serial_number { info["serial_number"] = json!(sn); }
        }
        info
    }).collect();
    Ok(json!(list))
});

// ---------- open ----------
#[derive(Deserialize)] pub struct OpenParams { pub path: String, pub baud_rate: Option<i32>, pub data_bits: Option<i32>, pub stop_bits: Option<i32>, pub parity: Option<String>, pub timeout_ms: Option<i32> }
hap_fn!(hap_serial_open, OpenParams, |p| {
    let baud = p.baud_rate.unwrap_or(9600) as u32;
    let data_bits = match p.data_bits.unwrap_or(8) {
        5 => serialport::DataBits::Five,
        6 => serialport::DataBits::Six,
        7 => serialport::DataBits::Seven,
        _ => serialport::DataBits::Eight,
    };
    let stop_bits = match p.stop_bits.unwrap_or(1) {
        2 => serialport::StopBits::Two,
        _ => serialport::StopBits::One,
    };
    let parity = match p.parity.as_deref().unwrap_or("none") {
        "odd" => serialport::Parity::Odd,
        "even" => serialport::Parity::Even,
        _ => serialport::Parity::None,
    };
    let timeout = Duration::from_millis(p.timeout_ms.unwrap_or(1000) as u64);
    let port = serialport::new(&p.path, baud)
        .data_bits(data_bits).stop_bits(stop_bits).parity(parity).timeout(timeout)
        .open().map_err(|e| HapError::internal(e.to_string()))?;
    let id = next_id();
    PORTS.lock().unwrap().insert(id.clone(), PortEntry { port, path: p.path.clone(), baud });
    Ok(json!({"port_id": id, "path": p.path, "baud_rate": baud}))
});

// ---------- close ----------
#[derive(Deserialize)] pub struct PortIdParams { pub port_id: String }
hap_fn!(hap_serial_close, PortIdParams, |p| {
    let removed = PORTS.lock().unwrap().remove(&p.port_id).is_some();
    Ok(json!(removed))
});

// ---------- write ----------
#[derive(Deserialize)] pub struct WriteParams { pub port_id: String, pub data: String, pub encoding: Option<String> }
hap_fn!(hap_serial_write, WriteParams, |p| {
    let mut map = PORTS.lock().unwrap();
    let entry = map.get_mut(&p.port_id).ok_or_else(|| HapError::invalid_param("invalid port_id"))?;
    let bytes = match p.encoding.as_deref() {
        Some("hex") => hex_to_bytes(&p.data)?,
        _ => p.data.as_bytes().to_vec(),
    };
    let n = entry.port.write(&bytes).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(n))
});

fn hex_to_bytes(s: &str) -> Result<Vec<u8>, HapError> {
    let clean: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if !clean.len().is_multiple_of(2) { return Err(HapError::invalid_param("hex length must be even")); }
    (0..clean.len()).step_by(2)
        .map(|i| u8::from_str_radix(&clean[i..i+2], 16).map_err(|e| HapError::invalid_param(e.to_string())))
        .collect()
}

// ---------- read ----------
#[derive(Deserialize)] pub struct ReadParams { pub port_id: String, pub size: Option<i32>, pub timeout_ms: Option<i32> }
hap_fn!(hap_serial_read, ReadParams, |p| {
    let mut map = PORTS.lock().unwrap();
    let entry = map.get_mut(&p.port_id).ok_or_else(|| HapError::invalid_param("invalid port_id"))?;
    if let Some(t) = p.timeout_ms {
        entry.port.set_timeout(Duration::from_millis(t as u64)).ok();
    }
    let size = p.size.unwrap_or(1024) as usize;
    let mut buf = vec![0u8; size];
    match entry.port.read(&mut buf) {
        Ok(n) => {
            buf.truncate(n);
            Ok(json!(String::from_utf8_lossy(&buf)))
        },
        Err(e) if e.kind() == std::io::ErrorKind::TimedOut => Ok(json!("")),
        Err(e) => Err(HapError::internal(e.to_string())),
    }
});

// ---------- read_line ----------
#[derive(Deserialize)] pub struct ReadLineParams { pub port_id: String, pub timeout_ms: Option<i32> }
hap_fn!(hap_serial_read_line, ReadLineParams, |p| {
    let mut map = PORTS.lock().unwrap();
    let entry = map.get_mut(&p.port_id).ok_or_else(|| HapError::invalid_param("invalid port_id"))?;
    if let Some(t) = p.timeout_ms {
        entry.port.set_timeout(Duration::from_millis(t as u64)).ok();
    }
    let mut result = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        match entry.port.read(&mut byte) {
            Ok(1) => {
                if byte[0] == b'\n' { break; }
                result.push(byte[0]);
            },
            Ok(_) => break,
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => break,
            Err(e) => return Err(HapError::internal(e.to_string())),
        }
    }
    Ok(json!(String::from_utf8_lossy(&result)))
});

// ---------- read_until ----------
#[derive(Deserialize)] pub struct ReadUntilParams { pub port_id: String, pub delimiter: String }
hap_fn!(hap_serial_read_until, ReadUntilParams, |p| {
    let mut map = PORTS.lock().unwrap();
    let entry = map.get_mut(&p.port_id).ok_or_else(|| HapError::invalid_param("invalid port_id"))?;
    let delim = p.delimiter.as_bytes();
    if delim.is_empty() { return Err(HapError::invalid_param("delimiter must not be empty")); }
    let mut result = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        match entry.port.read(&mut byte) {
            Ok(1) => {
                result.push(byte[0]);
                if result.ends_with(delim) { break; }
            },
            Ok(_) => break,
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => break,
            Err(e) => return Err(HapError::internal(e.to_string())),
        }
    }
    Ok(json!(String::from_utf8_lossy(&result)))
});

// ---------- available ----------
hap_fn!(hap_serial_available, PortIdParams, |p| {
    let mut map = PORTS.lock().unwrap();
    let entry = map.get_mut(&p.port_id).ok_or_else(|| HapError::invalid_param("invalid port_id"))?;
    let n = entry.port.bytes_to_read().map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(n))
});

// ---------- set_baud_rate ----------
#[derive(Deserialize)] pub struct SetBaudParams { pub port_id: String, pub baud_rate: i32 }
hap_fn!(hap_serial_set_baud_rate, SetBaudParams, |p| {
    let mut map = PORTS.lock().unwrap();
    let entry = map.get_mut(&p.port_id).ok_or_else(|| HapError::invalid_param("invalid port_id"))?;
    entry.port.set_baud_rate(p.baud_rate as u32).map_err(|e| HapError::internal(e.to_string()))?;
    entry.baud = p.baud_rate as u32;
    Ok(json!(true))
});

// ---------- set_dtr / set_rts ----------
#[derive(Deserialize)] pub struct SetSignalParams { pub port_id: String, pub state: bool }
hap_fn!(hap_serial_set_dtr, SetSignalParams, |p| {
    let mut map = PORTS.lock().unwrap();
    let entry = map.get_mut(&p.port_id).ok_or_else(|| HapError::invalid_param("invalid port_id"))?;
    entry.port.write_data_terminal_ready(p.state).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

hap_fn!(hap_serial_set_rts, SetSignalParams, |p| {
    let mut map = PORTS.lock().unwrap();
    let entry = map.get_mut(&p.port_id).ok_or_else(|| HapError::invalid_param("invalid port_id"))?;
    entry.port.write_request_to_send(p.state).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

// ---------- flush ----------
hap_fn!(hap_serial_flush, PortIdParams, |p| {
    let mut map = PORTS.lock().unwrap();
    let entry = map.get_mut(&p.port_id).ok_or_else(|| HapError::invalid_param("invalid port_id"))?;
    entry.port.flush().map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

// ---------- clear_buffer ----------
#[derive(Deserialize)] pub struct ClearBufParams { pub port_id: String, pub buffer_type: Option<String> }
hap_fn!(hap_serial_clear_buffer, ClearBufParams, |p| {
    let mut map = PORTS.lock().unwrap();
    let entry = map.get_mut(&p.port_id).ok_or_else(|| HapError::invalid_param("invalid port_id"))?;
    let ct = match p.buffer_type.as_deref() {
        Some("input") => serialport::ClearBuffer::Input,
        Some("output") => serialport::ClearBuffer::Output,
        _ => serialport::ClearBuffer::All,
    };
    entry.port.clear(ct).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

// ---------- on_port_change / off_port_change ----------
use std::sync::atomic::{AtomicBool, Ordering as AtomOrd};
use std::sync::Arc;

struct PortWatcher {
    stop_flag: Arc<AtomicBool>,
    _handle: std::thread::JoinHandle<()>,
}

static PORT_WATCHERS: LazyLock<Mutex<std::collections::HashMap<String, PortWatcher>>> = LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));
static WATCHER_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

#[derive(Deserialize)] pub struct CallbackParams { pub callback_id: String }
hap_fn!(hap_serial_on_port_change, CallbackParams, |_p| {
    let wid = format!("pw_{}", WATCHER_COUNTER.fetch_add(1, AtomOrd::Relaxed));
    let stop = Arc::new(AtomicBool::new(false));
    let stop_ref = stop.clone();
    let handle = std::thread::spawn(move || {
        let mut prev: Vec<String> = serialport::available_ports()
            .unwrap_or_default().iter().map(|p| p.port_name.clone()).collect();
        prev.sort();
        while !stop_ref.load(AtomOrd::Relaxed) {
            std::thread::sleep(std::time::Duration::from_secs(2));
            let mut curr: Vec<String> = serialport::available_ports()
                .unwrap_or_default().iter().map(|p| p.port_name.clone()).collect();
            curr.sort();
            if curr != prev {
                prev = curr;
            }
        }
    });
    PORT_WATCHERS.lock().unwrap().insert(wid.clone(), PortWatcher { stop_flag: stop, _handle: handle });
    Ok(json!({"watcher_id": wid}))
});

#[derive(Deserialize)] pub struct WatcherParams { pub watcher_id: String }
hap_fn!(hap_serial_off_port_change, WatcherParams, |p| {
    if let Some(w) = PORT_WATCHERS.lock().unwrap().remove(&p.watcher_id) {
        w.stop_flag.store(true, AtomOrd::Relaxed);
        Ok(json!(true))
    } else {
        Ok(json!(false))
    }
});

// ---------- is_open ----------
hap_fn!(hap_serial_is_open, PortIdParams, |p| {
    let map = PORTS.lock().unwrap();
    Ok(json!(map.contains_key(&p.port_id)))
});

// ---------- list_open ----------
hap_fn!(hap_serial_list_open, Value, |_p| {
    let map = PORTS.lock().unwrap();
    let list: Vec<Value> = map.iter().map(|(id, e)| json!({"port_id": id, "path": e.path, "baud_rate": e.baud})).collect();
    Ok(json!(list))
});
