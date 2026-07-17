use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::net::TcpStream;
use std::sync::{Arc, LazyLock, Mutex, atomic::{AtomicBool, AtomicU64, Ordering}};
use tungstenite::{Message, WebSocket, stream::MaybeTlsStream};

static CONN_MAP: LazyLock<Mutex<HashMap<String, WsConn>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static CONN_COUNTER: AtomicU64 = AtomicU64::new(1);

struct WsConn {
    ws: Arc<Mutex<WebSocket<MaybeTlsStream<TcpStream>>>>,
    url: String,
    state: Arc<Mutex<String>>,
    _reader_running: Arc<AtomicBool>,
    #[allow(dead_code)]
    callback_id: String,
    protocol: Option<String>,
    auto_reconnect: Arc<AtomicBool>,
    reconnect_interval_ms: Arc<AtomicU64>,
}

// ---------- connect ----------
#[derive(Deserialize)]
pub struct ConnectParams {
    pub url: String,
    pub headers: Option<Map<String, Value>>,
    pub protocols: Option<Vec<String>>,
    pub callback_id: String,
}
hap_fn!(hap_ws_connect, ConnectParams, |p| {
    let _url_parsed = url::Url::parse(&p.url).map_err(|e| HapError::invalid_param(format!("URL: {e}")))?;

    let mut request = tungstenite::http::Request::builder()
        .uri(p.url.as_str())
        .body(())
        .map_err(|e| HapError::internal(e.to_string()))?;

    if let Some(ref hdrs) = p.headers {
        for (k, v) in hdrs {
            if let Ok(name) = tungstenite::http::header::HeaderName::from_bytes(k.as_bytes()) {
                if let Some(val) = v.as_str() {
                    if let Ok(hv) = tungstenite::http::header::HeaderValue::from_str(val) {
                        request.headers_mut().insert(name, hv);
                    }
                }
            }
        }
    }
    if let Some(ref protos) = p.protocols {
        if !protos.is_empty() {
            let proto_str = protos.join(", ");
            if let Ok(hv) = tungstenite::http::header::HeaderValue::from_str(&proto_str) {
                request.headers_mut().insert("Sec-WebSocket-Protocol", hv);
            }
        }
    }

    let (ws, response) = tungstenite::connect(request)
        .map_err(|e| HapError::internal(format!("connection failed: {e}")))?;

    let protocol = response.headers().get("Sec-WebSocket-Protocol")
        .and_then(|v| v.to_str().ok()).map(|s| s.to_string());

    let conn_id = format!("ws_{}", CONN_COUNTER.fetch_add(1, Ordering::Relaxed));
    let state = Arc::new(Mutex::new("open".to_string()));
    let running = Arc::new(AtomicBool::new(true));
    let ws_arc = Arc::new(Mutex::new(ws));

    let ws_reader = ws_arc.clone();
    let state_reader = state.clone();
    let running_reader = running.clone();
    let cb_id = p.callback_id.clone();
    let cid = conn_id.clone();

    std::thread::spawn(move || {
        hap_common::context::emit_callback(&cb_id,
            &json!({"type": "onOpen", "conn_id": cid}).to_string());

        loop {
            if !running_reader.load(Ordering::Relaxed) { break; }
            let msg = {
                let mut guard = ws_reader.lock().unwrap();
                guard.read()
            };
            match msg {
                Ok(Message::Text(text)) => {
                    hap_common::context::emit_callback(&cb_id,
                        &json!({"type": "onMessage", "conn_id": cid, "data": text, "binary": false}).to_string());
                }
                Ok(Message::Binary(data)) => {
                    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
                    hap_common::context::emit_callback(&cb_id,
                        &json!({"type": "onMessage", "conn_id": cid, "data": b64, "binary": true}).to_string());
                }
                Ok(Message::Close(frame)) => {
                    let (code, reason) = frame.map(|f| (f.code.into(), f.reason.to_string())).unwrap_or((1000u16, String::new()));
                    *state_reader.lock().unwrap() = "closed".to_string();
                    hap_common::context::emit_callback(&cb_id,
                        &json!({"type": "onClose", "conn_id": cid, "code": code, "reason": reason}).to_string());
                    break;
                }
                Ok(Message::Ping(_)) | Ok(Message::Pong(_)) | Ok(Message::Frame(_)) => {}
                Err(e) => {
                    *state_reader.lock().unwrap() = "closed".to_string();
                    hap_common::context::emit_callback(&cb_id,
                        &json!({"type": "onError", "conn_id": cid, "error": e.to_string()}).to_string());
                    break;
                }
            }
        }
        running_reader.store(false, Ordering::Relaxed);
    });

    let conn = WsConn {
        ws: ws_arc, url: p.url.clone(), state, _reader_running: running,
        callback_id: p.callback_id.clone(), protocol,
        auto_reconnect: Arc::new(AtomicBool::new(false)),
        reconnect_interval_ms: Arc::new(AtomicU64::new(3000)),
    };
    CONN_MAP.lock().unwrap().insert(conn_id.clone(), conn);
    Ok(json!({"conn_id": conn_id}))
});

// ---------- send ----------
#[derive(Deserialize)]
pub struct SendParams { pub conn_id: String, pub data: String }
hap_fn!(hap_ws_send, SendParams, |p| {
    let map = CONN_MAP.lock().unwrap();
    let conn = map.get(&p.conn_id).ok_or_else(|| HapError::invalid_param("invalid conn_id"))?;
    let mut ws = conn.ws.lock().unwrap();
    ws.send(Message::Text(p.data.clone())).map_err(|e| HapError::internal(e.to_string()))?;
    ws.flush().map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

// ---------- send_binary ----------
#[derive(Deserialize)]
pub struct SendBinaryParams { pub conn_id: String, pub data: String }
hap_fn!(hap_ws_send_binary, SendBinaryParams, |p| {
    let map = CONN_MAP.lock().unwrap();
    let conn = map.get(&p.conn_id).ok_or_else(|| HapError::invalid_param("invalid conn_id"))?;
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &p.data)
        .map_err(|e| HapError::invalid_param(format!("base64: {e}")))?;
    let mut ws = conn.ws.lock().unwrap();
    ws.send(Message::Binary(bytes)).map_err(|e| HapError::internal(e.to_string()))?;
    ws.flush().map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

// ---------- close ----------
#[derive(Deserialize)]
pub struct CloseParams { pub conn_id: String, pub code: Option<u16>, pub reason: Option<String> }
hap_fn!(hap_ws_close, CloseParams, |p| {
    let mut map = CONN_MAP.lock().unwrap();
    if let Some(conn) = map.get(&p.conn_id) {
        *conn.state.lock().unwrap() = "closing".to_string();
        conn._reader_running.store(false, Ordering::Relaxed);
        let frame = tungstenite::protocol::CloseFrame {
            code: tungstenite::protocol::frame::coding::CloseCode::from(p.code.unwrap_or(1000)),
            reason: p.reason.as_deref().unwrap_or("").into(),
        };
        let _ = conn.ws.lock().unwrap().close(Some(frame));
        *conn.state.lock().unwrap() = "closed".to_string();
    }
    map.remove(&p.conn_id);
    Ok(json!(true))
});

// ---------- state ----------
#[derive(Deserialize)]
pub struct StateParams { pub conn_id: String }
hap_fn!(hap_ws_state, StateParams, |p| {
    let map = CONN_MAP.lock().unwrap();
    let conn = map.get(&p.conn_id).ok_or_else(|| HapError::invalid_param("invalid conn_id"))?;
    let s = conn.state.lock().unwrap().clone();
    Ok(json!(s))
});

// ---------- set_auto_reconnect ----------
#[derive(Deserialize)]
pub struct AutoReconnectParams {
    pub conn_id: String,
    pub enabled: bool,
    pub interval_ms: Option<u32>,
    #[allow(dead_code)] pub max_retries: Option<i32>,
}
hap_fn!(hap_ws_set_auto_reconnect, AutoReconnectParams, |p| {
    let map = CONN_MAP.lock().unwrap();
    let conn = map.get(&p.conn_id).ok_or_else(|| HapError::invalid_param("invalid conn_id"))?;
    conn.auto_reconnect.store(p.enabled, Ordering::Relaxed);
    if let Some(ms) = p.interval_ms {
        conn.reconnect_interval_ms.store(ms as u64, Ordering::Relaxed);
    }
    Ok(json!(true))
});

// ---------- ping ----------
#[derive(Deserialize)]
pub struct PingParams { pub conn_id: String }
hap_fn!(hap_ws_ping, PingParams, |p| {
    let map = CONN_MAP.lock().unwrap();
    let conn = map.get(&p.conn_id).ok_or_else(|| HapError::invalid_param("invalid conn_id"))?;
    let mut ws = conn.ws.lock().unwrap();
    ws.send(Message::Ping(vec![])).map_err(|e| HapError::internal(e.to_string()))?;
    ws.flush().map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

// ---------- buffered_amount (approximate) ----------
#[derive(Deserialize)]
pub struct BufferedAmountParams { pub conn_id: String }
hap_fn!(hap_ws_buffered_amount, BufferedAmountParams, |p| {
    let map = CONN_MAP.lock().unwrap();
    let _conn = map.get(&p.conn_id).ok_or_else(|| HapError::invalid_param("invalid conn_id"))?;
    Ok(json!(0i64))
});

// ---------- list_connections ----------
hap_fn!(hap_ws_list_connections, Value, |_p| {
    let map = CONN_MAP.lock().unwrap();
    let list: Vec<Value> = map.iter().map(|(id, c)| {
        json!({
            "conn_id": id,
            "url": c.url,
            "state": *c.state.lock().unwrap(),
            "protocol": c.protocol,
        })
    }).collect();
    Ok(json!(list))
});
