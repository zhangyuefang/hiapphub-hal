use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::sync::{Arc, Mutex, OnceLock};
use std::collections::VecDeque;

#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};

#[cfg(unix)]
type StreamType = UnixStream;
#[cfg(windows)]
type StreamType = std::net::TcpStream;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct ConnectedApp {
    pub app_id: String,
    pub authenticated: bool,
    pub status: String,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct PendingRequest {
    pub request_id: String,
    pub app_id: String,
    pub method: String,
    pub params: serde_json::Value,
    pub rpc_id: Option<serde_json::Value>,
}

struct ServerState {
    socket_path: String,
    running: bool,
    tokens: HashMap<String, String>,
    connections: HashMap<String, Arc<Mutex<StreamType>>>,
    apps: HashMap<String, ConnectedApp>,
    pending_requests: VecDeque<PendingRequest>,
    pending_responses: HashMap<String, std::sync::mpsc::Sender<JsonRpcResponse>>,
    next_req_id: u64,
}

const MAX_PENDING: usize = 1000;

static SERVER: OnceLock<Arc<Mutex<ServerState>>> = OnceLock::new();

fn get_state() -> Arc<Mutex<ServerState>> {
    SERVER.get_or_init(|| {
        #[cfg(unix)]
        let socket_path = std::env::temp_dir().join("hiapphub-shell.sock").to_string_lossy().to_string();
        #[cfg(windows)]
        let socket_path = std::env::temp_dir().join("hiapphub-shell.pipe").to_string_lossy().to_string();

        Arc::new(Mutex::new(ServerState {
            socket_path,
            running: false,
            tokens: HashMap::new(),
            connections: HashMap::new(),
            apps: HashMap::new(),
            pending_requests: VecDeque::new(),
            pending_responses: HashMap::new(),
            next_req_id: 1,
        }))
    }).clone()
}

pub fn start_server() -> Result<String, String> {
    let state = get_state();
    let socket_path;
    {
        let mut s = state.lock().unwrap();
        if s.running {
            return Ok(s.socket_path.clone());
        }
        s.running = true;
        socket_path = s.socket_path.clone();
    }

    #[cfg(unix)]
    {
        let _ = std::fs::remove_file(&socket_path);
        let listener = UnixListener::bind(&socket_path)
            .map_err(|e| format!("bind failed: {e}"))?;
        let state_clone = state.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let sc = state_clone.clone();
                std::thread::spawn(move || handle_connection(stream, sc));
            }
        });
    }

    #[cfg(windows)]
    {
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .map_err(|e| format!("tcp bind failed: {e}"))?;
        let addr = listener.local_addr().map_err(|e| format!("{e}"))?;
        std::fs::write(&socket_path, format!("{}", addr.port()))
            .map_err(|e| format!("write pipe info: {e}"))?;
        let state_clone = state.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                if let Ok(stream) = stream {
                    let sc = state_clone.clone();
                    std::thread::spawn(move || handle_connection(stream, sc));
                }
            }
        });
    }

    Ok(socket_path)
}

pub fn stop_server() -> bool {
    let state = get_state();
    let mut s = state.lock().unwrap();
    if !s.running { return false; }
    s.running = false;
    s.connections.clear();
    s.apps.clear();
    s.tokens.clear();
    s.pending_requests.clear();
    s.pending_responses.clear();
    let _ = std::fs::remove_file(&s.socket_path);
    true
}

pub fn generate_token(app_id: &str) -> String {
    let token = format!("{:016x}{:016x}", rand_u64(), rand_u64());
    let state = get_state();
    state.lock().unwrap().tokens.insert(token.clone(), app_id.to_string());
    token
}

pub fn get_socket_path() -> String {
    get_state().lock().unwrap().socket_path.clone()
}

pub fn is_running() -> bool {
    get_state().lock().unwrap().running
}

pub fn list_connected_apps() -> Vec<ConnectedApp> {
    get_state().lock().unwrap().apps.values().cloned().collect()
}

pub fn is_app_connected(app_id: &str) -> bool {
    get_state().lock().unwrap().connections.contains_key(app_id)
}

pub fn poll_requests(limit: usize) -> Vec<PendingRequest> {
    let state = get_state();
    let mut s = state.lock().unwrap();
    let n = limit.min(s.pending_requests.len());
    s.pending_requests.drain(..n).collect()
}

pub fn respond_request(request_id: &str, result: Option<serde_json::Value>, error: Option<JsonRpcError>) -> bool {
    let state = get_state();
    let tx = {
        let mut s = state.lock().unwrap();
        s.pending_responses.remove(request_id)
    };
    if let Some(tx) = tx {
        let resp = JsonRpcResponse {
            jsonrpc: "2.0".into(),
            result,
            error,
            id: None,
        };
        tx.send(resp).is_ok()
    } else {
        false
    }
}

pub fn send_to_app(app_id: &str, method: &str, params: serde_json::Value) -> Result<(), String> {
    let state = get_state();
    let conn = {
        let s = state.lock().unwrap();
        s.connections.get(app_id).cloned()
    };
    let stream = conn.ok_or_else(|| format!("app '{app_id}' not connected"))?;
    let notification = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        method: method.into(),
        params: Some(params),
        id: None,
    };
    let mut msg = serde_json::to_string(&notification).unwrap();
    msg.push('\n');
    let mut writer = stream.lock().unwrap();
    writer.write_all(msg.as_bytes()).map_err(|e| format!("send failed: {e}"))
}

fn handle_connection(stream: StreamType, state: Arc<Mutex<ServerState>>) {
    let writer = Arc::new(Mutex::new(
        stream.try_clone().expect("clone stream"),
    ));
    let reader = BufReader::new(stream);
    let mut authenticated_app_id: Option<String> = None;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.is_empty() { continue; }
        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(_) => continue,
        };

        if authenticated_app_id.is_none() && req.method != "auth.verify" {
            let resp = JsonRpcResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(JsonRpcError { code: -32600, message: "not authenticated".into() }),
                id: req.id.clone(),
            };
            if req.id.is_some() {
                let mut msg = serde_json::to_string(&resp).unwrap();
                msg.push('\n');
                let _ = writer.lock().unwrap().write_all(msg.as_bytes());
            }
            continue;
        }

        let response = if req.method == "auth.verify" {
            let token = req.params.as_ref()
                .and_then(|p| p["token"].as_str())
                .unwrap_or("");
            let mut s = state.lock().unwrap();
            if let Some(app_id) = s.tokens.get(token).cloned() {
                authenticated_app_id = Some(app_id.clone());
                s.connections.insert(app_id.clone(), writer.clone());
                s.apps.insert(app_id.clone(), ConnectedApp {
                    app_id: app_id.clone(),
                    authenticated: true,
                    status: "running".into(),
                });
                JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    result: Some(serde_json::json!({ "authenticated": true, "app_id": app_id })),
                    error: None,
                    id: req.id.clone(),
                }
            } else {
                JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    result: None,
                    error: Some(JsonRpcError { code: -32001, message: "invalid token".into() }),
                    id: req.id.clone(),
                }
            }
        } else if req.method == "app.reportStatus" {
            let status = req.params.as_ref()
                .and_then(|p| p["status"].as_str())
                .unwrap_or("unknown");
            if let Some(ref app_id) = authenticated_app_id {
                let mut s = state.lock().unwrap();
                if let Some(app) = s.apps.get_mut(app_id) {
                    app.status = status.to_string();
                }
            }
            JsonRpcResponse {
                jsonrpc: "2.0".into(),
                result: Some(serde_json::json!({ "acknowledged": true })),
                error: None,
                id: req.id.clone(),
            }
        } else {
            let app_id = authenticated_app_id.clone().unwrap_or_default();
            let (tx, rx) = std::sync::mpsc::channel();
            let req_id;
            {
                let mut s = state.lock().unwrap();
                req_id = format!("ipc-{}", s.next_req_id);
                s.next_req_id += 1;
                while s.pending_requests.len() >= MAX_PENDING {
                    if let Some(old) = s.pending_requests.pop_front() {
                        s.pending_responses.remove(&old.request_id);
                    }
                }
                s.pending_requests.push_back(PendingRequest {
                    request_id: req_id.clone(),
                    app_id,
                    method: req.method.clone(),
                    params: req.params.clone().unwrap_or(serde_json::Value::Null),
                    rpc_id: req.id.clone(),
                });
                s.pending_responses.insert(req_id.clone(), tx);
            }

            match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                Ok(mut resp) => {
                    resp.id = req.id.clone();
                    resp
                }
                Err(_) => {
                    state.lock().unwrap().pending_responses.remove(&req_id);
                    JsonRpcResponse {
                        jsonrpc: "2.0".into(),
                        result: None,
                        error: Some(JsonRpcError { code: -32000, message: "request timeout".into() }),
                        id: req.id.clone(),
                    }
                }
            }
        };

        if req.id.is_some() {
            let mut msg = serde_json::to_string(&response).unwrap();
            msg.push('\n');
            let _ = writer.lock().unwrap().write_all(msg.as_bytes());
        }
    }

    if let Some(ref app_id) = authenticated_app_id {
        let mut s = state.lock().unwrap();
        s.connections.remove(app_id);
        s.apps.remove(app_id);
    }
}

fn rand_u64() -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    std::time::SystemTime::now().hash(&mut hasher);
    std::thread::current().id().hash(&mut hasher);
    hasher.finish()
}
