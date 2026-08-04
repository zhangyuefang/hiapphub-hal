use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, LazyLock, Mutex, atomic::{AtomicBool, AtomicU64, Ordering}};
use tungstenite::{Message, WebSocket, accept};

static SERVER_MAP: LazyLock<Mutex<HashMap<String, WsServer>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static SERVER_COUNTER: AtomicU64 = AtomicU64::new(1);
static CLIENT_COUNTER: AtomicU64 = AtomicU64::new(1);
static MSG_BUFFER: LazyLock<Mutex<HashMap<String, Vec<BufferedMessage>>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

struct BufferedMessage {
    client_id: String,
    message: String,
}

type WsClientMap = HashMap<String, Arc<Mutex<WebSocket<TcpStream>>>>;

struct WsServer {
    running: Arc<AtomicBool>,
    clients: Arc<Mutex<WsClientMap>>,
    #[allow(dead_code)]
    addr: String,
}

#[derive(Deserialize)]
pub struct ListenParams {
    pub host: Option<String>,
    pub port: f64,
    pub path: Option<String>,
}

hap_fn!(hap_ws_server_listen, ListenParams, |p| {
    let host = p.host.unwrap_or_else(|| "0.0.0.0".to_string());
    let port = p.port as u16;
    let addr = format!("{host}:{port}");

    let listener = TcpListener::bind(&addr)
        .map_err(|e| HapError::internal(format!("bind failed: {e}")))?;
    listener.set_nonblocking(false)
        .map_err(|e| HapError::internal(format!("set blocking failed: {e}")))?;

    let server_id = format!("wss_{}", SERVER_COUNTER.fetch_add(1, Ordering::Relaxed));
    let running = Arc::new(AtomicBool::new(true));
    let clients: Arc<Mutex<WsClientMap>> = Arc::new(Mutex::new(HashMap::new()));

    let run_flag = running.clone();
    let clients_ref = clients.clone();
    let _sid = server_id.clone();

    let sid_for_buf = server_id.clone();
    std::thread::spawn(move || {
        listener.set_nonblocking(true).ok();
        while run_flag.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((stream, _addr)) => {
                    stream.set_nonblocking(false).ok();
                    stream.set_read_timeout(Some(std::time::Duration::from_millis(100))).ok();
                    let ws = match accept(stream) {
                        Ok(ws) => ws,
                        Err(_) => continue,
                    };
                    let client_id = format!("wsc_{}", CLIENT_COUNTER.fetch_add(1, Ordering::Relaxed));
                    let ws_arc = Arc::new(Mutex::new(ws));
                    clients_ref.lock().unwrap().insert(client_id.clone(), ws_arc.clone());

                    let clients_inner = clients_ref.clone();
                    let cid = client_id.clone();
                    let run_inner = run_flag.clone();
                    let buf_sid = sid_for_buf.clone();
                    std::thread::spawn(move || {
                        loop {
                            if !run_inner.load(Ordering::Relaxed) { break; }
                            let msg = { ws_arc.lock().unwrap().read() };
                            match msg {
                                Ok(Message::Text(text)) => {
                                    let mut buf = MSG_BUFFER.lock().unwrap();
                                    let queue = buf.entry(buf_sid.clone()).or_default();
                                    if queue.len() < 1000 {
                                        queue.push(BufferedMessage { client_id: cid.clone(), message: text });
                                    }
                                }
                                Ok(Message::Close(_)) => {
                                    clients_inner.lock().unwrap().remove(&cid);
                                    break;
                                }
                                Err(tungstenite::Error::Io(ref e))
                                    if e.kind() == std::io::ErrorKind::WouldBlock
                                        || e.kind() == std::io::ErrorKind::TimedOut => {
                                    continue;
                                }
                                Err(_) => {
                                    clients_inner.lock().unwrap().remove(&cid);
                                    break;
                                }
                                _ => {}
                            }
                        }
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                Err(_) => break,
            }
        }
        drop(listener);
    });

    let server = WsServer { running, clients, addr: addr.clone() };
    SERVER_MAP.lock().unwrap().insert(server_id.clone(), server);
    Ok(json!(server_id))
});

#[derive(Deserialize)]
pub struct ServerCloseParams { pub server_id: String }

hap_fn!(hap_ws_server_close, ServerCloseParams, |p| {
    let mut map = SERVER_MAP.lock().unwrap();
    if let Some(server) = map.remove(&p.server_id) {
        server.running.store(false, Ordering::Relaxed);
        let clients = server.clients.lock().unwrap();
        for (_, ws) in clients.iter() {
            let _ = ws.lock().unwrap().close(None);
        }
    }
    Ok(json!(true))
});

#[derive(Deserialize)]
pub struct ServerSendParams { pub server_id: String, pub client_id: String, #[serde(alias = "data")] pub message: String }

hap_fn!(hap_ws_server_send, ServerSendParams, |p| {
    let map = SERVER_MAP.lock().unwrap();
    let server = map.get(&p.server_id).ok_or_else(|| HapError::invalid_param("invalid server_id"))?;
    let clients = server.clients.lock().unwrap();
    let ws = clients.get(&p.client_id).ok_or_else(|| HapError::invalid_param("invalid client_id"))?;
    let mut guard = ws.lock().unwrap();
    guard.send(Message::Text(p.message.clone())).map_err(|e| HapError::internal(e.to_string()))?;
    guard.flush().map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

#[derive(Deserialize)]
pub struct ServerBroadcastParams { pub server_id: String, #[serde(alias = "data")] pub message: String }

hap_fn!(hap_ws_server_broadcast, ServerBroadcastParams, |p| {
    let map = SERVER_MAP.lock().unwrap();
    let server = map.get(&p.server_id).ok_or_else(|| HapError::invalid_param("invalid server_id"))?;
    let clients = server.clients.lock().unwrap();
    let mut sent = 0i64;
    for (_, ws) in clients.iter() {
        let mut guard = ws.lock().unwrap();
        if guard.send(Message::Text(p.message.clone())).is_ok() {
            let _ = guard.flush();
            sent += 1;
        }
    }
    Ok(json!(sent))
});

#[derive(Deserialize)]
pub struct ServerClientsParams { pub server_id: String }

hap_fn!(hap_ws_server_clients, ServerClientsParams, |p| {
    let map = SERVER_MAP.lock().unwrap();
    let server = map.get(&p.server_id).ok_or_else(|| HapError::invalid_param("invalid server_id"))?;
    let clients = server.clients.lock().unwrap();
    let list: Vec<Value> = clients.keys().map(|id| json!({"client_id": id})).collect();
    Ok(json!(list))
});

#[derive(Deserialize)]
pub struct ServerDisconnectParams { pub server_id: String, pub client_id: String }

hap_fn!(hap_ws_server_disconnect, ServerDisconnectParams, |p| {
    let map = SERVER_MAP.lock().unwrap();
    let server = map.get(&p.server_id).ok_or_else(|| HapError::invalid_param("invalid server_id"))?;
    let mut clients = server.clients.lock().unwrap();
    if let Some(ws) = clients.remove(&p.client_id) {
        let _ = ws.lock().unwrap().close(None);
    }
    Ok(json!(true))
});

#[derive(Deserialize)]
pub struct ServerRecvParams { pub server_id: String, pub limit: Option<usize> }

hap_fn!(hap_ws_server_recv, ServerRecvParams, |p| {
    let limit = p.limit.unwrap_or(50);
    let mut buf = MSG_BUFFER.lock().unwrap();
    let queue = buf.entry(p.server_id.clone()).or_default();
    let count = queue.len().min(limit);
    let messages: Vec<Value> = queue.drain(..count)
        .map(|m| json!({"client_id": m.client_id, "message": m.message}))
        .collect();
    Ok(json!(messages))
});
