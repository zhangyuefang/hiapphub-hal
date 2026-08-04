use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpStream, TcpListener, SocketAddr};
use std::sync::{LazyLock, Mutex, atomic::{AtomicU64, Ordering}};
use std::time::Duration;

static TCP_CONNS: LazyLock<Mutex<HashMap<String, Mutex<TcpStream>>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static TCP_SERVERS: LazyLock<Mutex<HashMap<String, TcpListener>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static COUNTER: AtomicU64 = AtomicU64::new(1);

fn next_id(prefix: &str) -> String {
    format!("{}_{}", prefix, COUNTER.fetch_add(1, Ordering::Relaxed))
}

fn encode_data(data: &[u8], encoding: &str) -> String {
    match encoding {
        "hex" => hex_encode(data),
        "base64" => base64::Engine::encode(&base64::engine::general_purpose::STANDARD, data),
        _ => String::from_utf8_lossy(data).to_string(),
    }
}

fn decode_data(data: &str, encoding: &str) -> Result<Vec<u8>, HapError> {
    match encoding {
        "hex" => hex_decode(data).map_err(HapError::invalid_param),
        "base64" => base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data)
            .map_err(|e| HapError::invalid_param(e.to_string())),
        _ => Ok(data.as_bytes().to_vec()),
    }
}

fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn hex_encode_pub(data: &[u8]) -> String {
    hex_encode(data)
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    (0..s.len()).step_by(2).map(|i| {
        u8::from_str_radix(&s[i..i+2], 16).map_err(|e| e.to_string())
    }).collect()
}

// ---------- tcp_connect ----------
#[derive(Deserialize)]
pub struct TcpConnectParams {
    pub host: String, pub port: i32, pub timeout_ms: Option<u32>,
    #[allow(dead_code)] pub tls: Option<bool>,
    #[allow(dead_code)] pub callback_id: Option<String>,
}
hap_fn!(hap_net_tcp_connect, TcpConnectParams, |p| {
    let addr = format!("{}:{}", p.host, p.port);
    let timeout = Duration::from_millis(p.timeout_ms.unwrap_or(5000) as u64);
    let sock_addr: SocketAddr = addr.parse().map_err(|e: std::net::AddrParseError| {
        let addrs: Vec<SocketAddr> = dns_lookup::lookup_host(&p.host)
            .unwrap_or_default().into_iter()
            .map(|ip| SocketAddr::new(ip, p.port as u16)).collect();
        if addrs.is_empty() { return HapError::invalid_param(format!("cannot resolve: {e}")); }
        HapError::invalid_param("".to_string())// won't reach
    }).or_else(|_| -> Result<SocketAddr, HapError> {
        let addrs: Vec<SocketAddr> = dns_lookup::lookup_host(&p.host)
            .map_err(|e| HapError::internal(e.to_string()))?
            .into_iter().map(|ip| SocketAddr::new(ip, p.port as u16)).collect();
        addrs.into_iter().next().ok_or_else(|| HapError::internal("cannot resolve host"))
    })?;

    let stream = TcpStream::connect_timeout(&sock_addr, timeout)
        .map_err(|e| HapError::internal(format!("connection failed: {e}")))?;
    let local_addr = stream.local_addr().map(|a| a.to_string()).unwrap_or_default();
    let id = next_id("tcp");
    TCP_CONNS.lock().unwrap().insert(id.clone(), Mutex::new(stream));
    Ok(json!({"conn_id": id, "local_addr": local_addr}))
});

// ---------- tcp_send ----------
#[derive(Deserialize)]
pub struct TcpSendParams { pub conn_id: String, pub data: String, pub encoding: Option<String> }
hap_fn!(hap_net_tcp_send, TcpSendParams, |p| {
    let enc = p.encoding.as_deref().unwrap_or("utf8");
    let bytes = decode_data(&p.data, enc)?;
    let map = TCP_CONNS.lock().unwrap();
    let mtx = map.get(&p.conn_id).ok_or_else(|| HapError::invalid_param("invalid conn_id"))?;
    let mut stream = mtx.lock().unwrap();
    let n = stream.write(&bytes).map_err(|e| HapError::internal(e.to_string()))?;
    stream.flush().map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(n as i32))
});

// ---------- tcp_recv ----------
#[derive(Deserialize)]
pub struct TcpRecvParams { pub conn_id: String, pub size: i32, pub timeout_ms: Option<u32>, pub encoding: Option<String> }
hap_fn!(hap_net_tcp_recv, TcpRecvParams, |p| {
    let enc = p.encoding.as_deref().unwrap_or("utf8");
    let map = TCP_CONNS.lock().unwrap();
    let mtx = map.get(&p.conn_id).ok_or_else(|| HapError::invalid_param("invalid conn_id"))?;
    let mut stream = mtx.lock().unwrap();
    if let Some(t) = p.timeout_ms {
        stream.set_read_timeout(Some(Duration::from_millis(t as u64))).ok();
    }
    let mut buf = vec![0u8; p.size as usize];
    let n = stream.read(&mut buf).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(encode_data(&buf[..n], enc)))
});

// ---------- tcp_close ----------
#[derive(Deserialize)]
pub struct TcpCloseParams { pub conn_id: String }
hap_fn!(hap_net_tcp_close, TcpCloseParams, |p| {
    TCP_CONNS.lock().unwrap().remove(&p.conn_id);
    Ok(json!(true))
});

// ---------- tcp_is_connected ----------
hap_fn!(hap_net_tcp_is_connected, TcpCloseParams, |p| {
    let map = TCP_CONNS.lock().unwrap();
    if let Some(mtx) = map.get(&p.conn_id) {
        let stream = mtx.lock().unwrap();
        let mut buf = [0u8; 0];
        stream.set_nonblocking(true).ok();
        let connected = match stream.peek(&mut buf) {
            Ok(_) => true,
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => true,
            _ => false,
        };
        stream.set_nonblocking(false).ok();
        Ok(json!(connected))
    } else {
        Ok(json!(false))
    }
});

// ---------- tcp_set_keepalive ----------
#[derive(Deserialize)]
pub struct TcpKeepaliveParams { pub conn_id: String, pub enabled: bool, #[allow(dead_code)] pub interval_ms: Option<u32> }
hap_fn!(hap_net_tcp_set_keepalive, TcpKeepaliveParams, |_p| {
    Ok(json!(true))
});

// ---------- tcp_set_nodelay ----------
#[derive(Deserialize)]
pub struct TcpNodelayParams { pub conn_id: String, pub enabled: bool }
hap_fn!(hap_net_tcp_set_nodelay, TcpNodelayParams, |p| {
    let map = TCP_CONNS.lock().unwrap();
    let mtx = map.get(&p.conn_id).ok_or_else(|| HapError::invalid_param("invalid conn_id"))?;
    let stream = mtx.lock().unwrap();
    stream.set_nodelay(p.enabled).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

// ---------- tcp_listen ----------
#[derive(Deserialize)]
pub struct TcpListenParams {
    pub host: String, pub port: i32,
    #[allow(dead_code)] pub backlog: Option<i32>,
    #[allow(dead_code)] pub callback_id: Option<String>,
}
hap_fn!(hap_net_tcp_listen, TcpListenParams, |p| {
    let addr = format!("{}:{}", p.host, p.port);
    let listener = TcpListener::bind(&addr).map_err(|e| HapError::internal(format!("listen failed: {e}")))?;
    let local_addr = listener.local_addr().map(|a| a.to_string()).unwrap_or_default();
    let id = next_id("srv");
    TCP_SERVERS.lock().unwrap().insert(id.clone(), listener);
    Ok(json!({"server_id": id, "local_addr": local_addr}))
});

// ---------- tcp_accept ----------
#[derive(Deserialize)]
pub struct TcpAcceptParams { pub server_id: String, pub timeout_ms: Option<u32> }
hap_fn!(hap_net_tcp_accept, TcpAcceptParams, |p| {
    let servers = TCP_SERVERS.lock().unwrap();
    let listener = servers.get(&p.server_id).ok_or_else(|| HapError::invalid_param("invalid server_id"))?;
    if let Some(_t) = p.timeout_ms {
        listener.set_nonblocking(false).ok();
    }
    let (stream, addr) = listener.accept().map_err(|e| HapError::internal(e.to_string()))?;
    let conn_id = next_id("tcp");
    drop(servers);
    TCP_CONNS.lock().unwrap().insert(conn_id.clone(), Mutex::new(stream));
    Ok(json!({"conn_id": conn_id, "remote_addr": addr.to_string()}))
});

// ---------- tcp_stop ----------
#[derive(Deserialize)]
pub struct TcpStopParams { pub server_id: String }
hap_fn!(hap_net_tcp_stop, TcpStopParams, |p| {
    TCP_SERVERS.lock().unwrap().remove(&p.server_id);
    Ok(json!(true))
});

// ---------- list_tcp_connections ----------
hap_fn!(hap_net_list_tcp_connections, Value, |_p| {
    let map = TCP_CONNS.lock().unwrap();
    let list: Vec<Value> = map.iter().map(|(id, mtx)| {
        let stream = mtx.lock().unwrap();
        json!({
            "conn_id": id,
            "remote_addr": stream.peer_addr().map(|a| a.to_string()).unwrap_or_default(),
            "local_addr": stream.local_addr().map(|a| a.to_string()).unwrap_or_default(),
        })
    }).collect();
    Ok(json!(list))
});

// ---------- list_tcp_servers ----------
hap_fn!(hap_net_list_tcp_servers, Value, |_p| {
    let map = TCP_SERVERS.lock().unwrap();
    let list: Vec<Value> = map.iter().map(|(id, l)| {
        json!({
            "server_id": id,
            "local_addr": l.local_addr().map(|a| a.to_string()).unwrap_or_default(),
        })
    }).collect();
    Ok(json!(list))
});
