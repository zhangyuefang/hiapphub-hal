use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::UdpSocket;
use std::sync::{LazyLock, Mutex, atomic::{AtomicU64, Ordering}};
use std::time::Duration;

static UDP_SOCKETS: LazyLock<Mutex<HashMap<String, UdpSocket>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static UDP_COUNTER: AtomicU64 = AtomicU64::new(1);

fn encode_data(data: &[u8], encoding: &str) -> String {
    match encoding {
        "hex" => data.iter().map(|b| format!("{:02x}", b)).collect(),
        "base64" => base64::Engine::encode(&base64::engine::general_purpose::STANDARD, data),
        _ => String::from_utf8_lossy(data).to_string(),
    }
}

fn decode_data(data: &str, encoding: &str) -> Result<Vec<u8>, HapError> {
    match encoding {
        "hex" => (0..data.len()).step_by(2).map(|i|
            u8::from_str_radix(&data[i..i+2], 16).map_err(|e| HapError::invalid_param(e.to_string()))
        ).collect(),
        "base64" => base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data)
            .map_err(|e| HapError::invalid_param(e.to_string())),
        _ => Ok(data.as_bytes().to_vec()),
    }
}

// ---------- udp_bind ----------
#[derive(Deserialize)]
pub struct UdpBindParams {
    pub host: String, pub port: i32,
    #[allow(dead_code)] pub callback_id: Option<String>,
}
hap_fn!(hap_net_udp_bind, UdpBindParams, |p| {
    let addr = format!("{}:{}", p.host, p.port);
    let socket = UdpSocket::bind(&addr).map_err(|e| HapError::internal(format!("bind failed: {e}")))?;
    let local_addr = socket.local_addr().map(|a| a.to_string()).unwrap_or_default();
    let id = format!("udp_{}", UDP_COUNTER.fetch_add(1, Ordering::Relaxed));
    UDP_SOCKETS.lock().unwrap().insert(id.clone(), socket);
    Ok(json!({"socket_id": id, "local_addr": local_addr}))
});

// ---------- udp_send ----------
#[derive(Deserialize)]
pub struct UdpSendParams { pub socket_id: String, pub host: String, pub port: i32, pub data: String, pub encoding: Option<String> }
hap_fn!(hap_net_udp_send, UdpSendParams, |p| {
    let enc = p.encoding.as_deref().unwrap_or("utf8");
    let bytes = decode_data(&p.data, enc)?;
    let target = format!("{}:{}", p.host, p.port);
    let map = UDP_SOCKETS.lock().unwrap();
    let socket = map.get(&p.socket_id).ok_or_else(|| HapError::invalid_param("invalid socket_id"))?;
    let n = socket.send_to(&bytes, &target).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(n as i32))
});

// ---------- udp_recv ----------
#[derive(Deserialize)]
pub struct UdpRecvParams { pub socket_id: String, pub size: i32, pub timeout_ms: Option<u32>, pub encoding: Option<String> }
hap_fn!(hap_net_udp_recv, UdpRecvParams, |p| {
    let enc = p.encoding.as_deref().unwrap_or("utf8");
    let map = UDP_SOCKETS.lock().unwrap();
    let socket = map.get(&p.socket_id).ok_or_else(|| HapError::invalid_param("invalid socket_id"))?;
    if let Some(t) = p.timeout_ms {
        socket.set_read_timeout(Some(Duration::from_millis(t as u64))).ok();
    }
    let mut buf = vec![0u8; p.size as usize];
    let (n, addr) = socket.recv_from(&mut buf).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!({
        "data": encode_data(&buf[..n], enc),
        "remote_host": addr.ip().to_string(),
        "remote_port": addr.port(),
    }))
});

// ---------- udp_close ----------
#[derive(Deserialize)]
pub struct UdpCloseParams { pub socket_id: String }
hap_fn!(hap_net_udp_close, UdpCloseParams, |p| {
    UDP_SOCKETS.lock().unwrap().remove(&p.socket_id);
    Ok(json!(true))
});

// ---------- udp_set_broadcast ----------
#[derive(Deserialize)]
pub struct UdpBroadcastParams { pub socket_id: String, pub enabled: bool }
hap_fn!(hap_net_udp_set_broadcast, UdpBroadcastParams, |p| {
    let map = UDP_SOCKETS.lock().unwrap();
    let socket = map.get(&p.socket_id).ok_or_else(|| HapError::invalid_param("invalid socket_id"))?;
    socket.set_broadcast(p.enabled).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

// ---------- list_udp_sockets ----------
hap_fn!(hap_net_list_udp_sockets, Value, |_p| {
    let map = UDP_SOCKETS.lock().unwrap();
    let list: Vec<Value> = map.iter().map(|(id, s)| {
        json!({
            "socket_id": id,
            "local_addr": s.local_addr().map(|a| a.to_string()).unwrap_or_default(),
        })
    }).collect();
    Ok(json!(list))
});
