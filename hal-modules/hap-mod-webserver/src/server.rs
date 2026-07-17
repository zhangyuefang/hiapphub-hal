use axum::{
    Router,
    body::Body,
    extract::Request,
    http::StatusCode,
    response::Response,
};
use hap_common::HapError;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, OnceLock};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::runtime::Runtime;
use tokio::sync::oneshot;
use tower_http::cors::CorsLayer;

struct PendingRequest {
    tx: oneshot::Sender<PendingResponse>,
    method: String,
    uri: String,
    headers: HashMap<String, String>,
    body: String,
    created_at: std::time::Instant,
}

struct PendingResponse {
    status: u16,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

struct ServerState {
    server_id: String,
    host: String,
    port: u16,
    static_dir: Option<String>,
    shutdown_tx: Option<tokio::sync::watch::Sender<bool>>,
    pending: Arc<Mutex<HashMap<String, PendingRequest>>>,
    timeout_ms: Arc<AtomicU64>,
    request_count: Arc<AtomicU64>,
}

static RUNTIME: OnceLock<Runtime> = OnceLock::new();
static SERVERS: OnceLock<Mutex<HashMap<String, ServerState>>> = OnceLock::new();

fn rt() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime")
    })
}

fn servers() -> &'static Mutex<HashMap<String, ServerState>> {
    SERVERS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn listen(port: Option<u16>, host: Option<String>, cors: Option<bool>) -> Result<Value, HapError> {
    let port = port.unwrap_or(3000);
    let host = host.unwrap_or_else(|| "127.0.0.1".to_string());
    let enable_cors = cors.unwrap_or(false);
    let server_id = uuid::Uuid::new_v4().to_string();

    let pending: Arc<Mutex<HashMap<String, PendingRequest>>> = Arc::new(Mutex::new(HashMap::new()));
    let pending_clone = pending.clone();
    let timeout_ms = Arc::new(AtomicU64::new(30000));
    let timeout_clone = timeout_ms.clone();
    let request_count = Arc::new(AtomicU64::new(0));
    let req_count_clone = request_count.clone();

    let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);

    let addr_str = format!("{}:{}", host, port);
    let addr: SocketAddr = addr_str.parse()
        .map_err(|e| HapError::internal(format!("Invalid address: {}", e)))?;

    let app = {
        let pending_for_handler = pending_clone;
        let handler = move |req: Request<Body>| {
            let pending_inner = pending_for_handler.clone();
            let timeout_val = timeout_clone.clone();
            let counter = req_count_clone.clone();
            async move {
                counter.fetch_add(1, Ordering::Relaxed);
                let req_id = uuid::Uuid::new_v4().to_string();
                let method = req.method().to_string();
                let uri = req.uri().to_string();
                let headers: HashMap<String, String> = req.headers().iter()
                    .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                    .collect();
                let body_bytes = axum::body::to_bytes(req.into_body(), 10 * 1024 * 1024)
                    .await
                    .unwrap_or_default();
                let body_str = String::from_utf8_lossy(&body_bytes).to_string();

                let (tx, rx) = oneshot::channel();
                {
                    let mut map = pending_inner.lock().unwrap();
                    map.insert(req_id.clone(), PendingRequest {
                        tx,
                        method,
                        uri,
                        headers,
                        body: body_str,
                        created_at: std::time::Instant::now(),
                    });
                }

                let timeout_dur = std::time::Duration::from_millis(timeout_val.load(Ordering::Relaxed));
                match tokio::time::timeout(timeout_dur, rx).await {
                    Ok(Ok(resp)) => {
                        let status = StatusCode::from_u16(resp.status).unwrap_or(StatusCode::OK);
                        let mut builder = Response::builder().status(status);
                        for (k, v) in &resp.headers {
                            builder = builder.header(k.as_str(), v.as_str());
                        }
                        builder.body(Body::from(resp.body)).unwrap_or_else(|_| {
                            Response::builder().status(500).body(Body::from("Internal error")).unwrap()
                        })
                    }
                    _ => {
                        pending_inner.lock().unwrap().remove(&req_id);
                        Response::builder()
                            .status(504)
                            .body(Body::from("Gateway Timeout"))
                            .unwrap()
                    }
                }
            }
        };

        let mut router = Router::new().fallback(handler);
        if enable_cors {
            router = router.layer(CorsLayer::permissive());
        }
        router
    };

    let sid = server_id.clone();
    let host_clone = host.clone();

    let actual_port = rt().block_on(async {
        let listener = tokio::net::TcpListener::bind(addr).await
            .map_err(|e| HapError::internal(format!("Bind failed: {}", e)))?;
        let ap = listener.local_addr()
            .map(|a| a.port())
            .unwrap_or(port);

        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    shutdown_rx.changed().await.ok();
                })
                .await
                .ok();
        });

        Ok::<u16, HapError>(ap)
    })?;

    let state = ServerState {
        server_id: sid.clone(),
        host: host_clone,
        port: actual_port,
        static_dir: None,
        shutdown_tx: Some(shutdown_tx),
        pending,
        timeout_ms,
        request_count,
    };
    servers().lock().unwrap().insert(sid.clone(), state);

    Ok(json!({"server_id": sid, "port": actual_port}))
}

pub fn stop(server_id: &str) -> Result<Value, HapError> {
    let mut map = servers().lock().unwrap();
    if let Some(mut state) = map.remove(server_id) {
        if let Some(tx) = state.shutdown_tx.take() {
            let _ = tx.send(true);
        }
        Ok(json!(true))
    } else {
        Err(HapError::internal("Server not found"))
    }
}

pub fn respond(
    server_id: &str,
    request_id: &str,
    status: Option<u16>,
    headers: Option<HashMap<String, String>>,
    body: Option<String>,
) -> Result<Value, HapError> {
    let map = servers().lock().unwrap();
    if let Some(state) = map.get(server_id) {
        let mut pending = state.pending.lock().unwrap();
        if let Some(req) = pending.remove(request_id) {
            let resp = PendingResponse {
                status: status.unwrap_or(200),
                headers: headers.unwrap_or_default(),
                body: body.unwrap_or_default().into_bytes(),
            };
            let _ = req.tx.send(resp);
            Ok(json!(true))
        } else {
            Err(HapError::internal("Request not found or already responded"))
        }
    } else {
        Err(HapError::internal("Server not found"))
    }
}

pub fn respond_json(
    server_id: &str,
    request_id: &str,
    status: Option<u16>,
    data: Value,
) -> Result<Value, HapError> {
    let mut headers = HashMap::new();
    headers.insert("content-type".to_string(), "application/json".to_string());
    let body = serde_json::to_string(&data).unwrap_or_default();
    
    let map = servers().lock().unwrap();
    if let Some(state) = map.get(server_id) {
        let mut pending = state.pending.lock().unwrap();
        if let Some(req) = pending.remove(request_id) {
            let resp = PendingResponse {
                status: status.unwrap_or(200),
                headers,
                body: body.into_bytes(),
            };
            let _ = req.tx.send(resp);
            Ok(json!(true))
        } else {
            Err(HapError::internal("Request not found or already responded"))
        }
    } else {
        Err(HapError::internal("Server not found"))
    }
}

pub fn respond_file(
    server_id: &str,
    request_id: &str,
    file_path: &str,
    content_type: Option<String>,
) -> Result<Value, HapError> {
    let data = std::fs::read(file_path)
        .map_err(|e| HapError::internal(format!("Read file failed: {}", e)))?;
    
    let ct = content_type.unwrap_or_else(|| guess_content_type(file_path));
    let mut headers = HashMap::new();
    headers.insert("content-type".to_string(), ct);

    let map = servers().lock().unwrap();
    if let Some(state) = map.get(server_id) {
        let mut pending = state.pending.lock().unwrap();
        if let Some(req) = pending.remove(request_id) {
            let resp = PendingResponse {
                status: 200,
                headers,
                body: data,
            };
            let _ = req.tx.send(resp);
            Ok(json!(true))
        } else {
            Err(HapError::internal("Request not found or already responded"))
        }
    } else {
        Err(HapError::internal("Server not found"))
    }
}

pub fn redirect(
    server_id: &str,
    request_id: &str,
    url: &str,
    status: Option<u16>,
) -> Result<Value, HapError> {
    let mut headers = HashMap::new();
    headers.insert("location".to_string(), url.to_string());
    let st = status.unwrap_or(302);

    let map = servers().lock().unwrap();
    if let Some(state) = map.get(server_id) {
        let mut pending = state.pending.lock().unwrap();
        if let Some(req) = pending.remove(request_id) {
            let resp = PendingResponse {
                status: st,
                headers,
                body: Vec::new(),
            };
            let _ = req.tx.send(resp);
            Ok(json!(true))
        } else {
            Err(HapError::internal("Request not found or already responded"))
        }
    } else {
        Err(HapError::internal("Server not found"))
    }
}

pub fn get_requests(server_id: &str) -> Result<Value, HapError> {
    let map = servers().lock().unwrap();
    if let Some(state) = map.get(server_id) {
        let pending = state.pending.lock().unwrap();
        let list: Vec<Value> = pending.iter().map(|(id, req)| {
            json!({
                "request_id": id,
                "method": req.method,
                "uri": req.uri,
                "headers": req.headers,
                "body": req.body,
                "elapsed_ms": req.created_at.elapsed().as_millis() as u64
            })
        }).collect();
        Ok(json!(list))
    } else {
        Err(HapError::internal("Server not found"))
    }
}

pub fn set_timeout(server_id: &str, timeout_ms: u64) -> Result<Value, HapError> {
    let map = servers().lock().unwrap();
    if let Some(state) = map.get(server_id) {
        state.timeout_ms.store(timeout_ms, Ordering::Relaxed);
        Ok(json!(true))
    } else {
        Err(HapError::internal("Server not found"))
    }
}

pub fn list_servers() -> Result<Value, HapError> {
    let map = servers().lock().unwrap();
    let list: Vec<Value> = map.values().map(|s| {
        json!({
            "server_id": s.server_id,
            "host": s.host,
            "port": s.port,
            "static_dir": s.static_dir,
            "pending_requests": s.pending.lock().unwrap().len(),
            "total_requests": s.request_count.load(Ordering::Relaxed)
        })
    }).collect();
    Ok(json!(list))
}

pub fn add_static(server_id: &str, dir: &str) -> Result<Value, HapError> {
    if !std::path::Path::new(dir).is_dir() {
        return Err(HapError::internal("Directory does not exist"));
    }
    let mut map = servers().lock().unwrap();
    if let Some(state) = map.get_mut(server_id) {
        state.static_dir = Some(dir.to_string());
        Ok(json!(true))
    } else {
        Err(HapError::internal("Server not found"))
    }
}

pub fn server_info(server_id: &str) -> Result<Value, HapError> {
    let map = servers().lock().unwrap();
    if let Some(state) = map.get(server_id) {
        Ok(json!({
            "server_id": state.server_id,
            "host": state.host,
            "port": state.port,
            "static_dir": state.static_dir,
            "pending_requests": state.pending.lock().unwrap().len(),
            "total_requests": state.request_count.load(Ordering::Relaxed),
            "timeout_ms": state.timeout_ms.load(Ordering::Relaxed)
        }))
    } else {
        Err(HapError::internal("Server not found"))
    }
}

pub fn pending_count(server_id: &str) -> Result<Value, HapError> {
    let map = servers().lock().unwrap();
    if let Some(state) = map.get(server_id) {
        let count = state.pending.lock().unwrap().len();
        Ok(json!(count))
    } else {
        Err(HapError::internal("Server not found"))
    }
}

fn guess_content_type(path: &str) -> String {
    let ext = path.rsplit('.').next().unwrap_or("");
    match ext.to_lowercase().as_str() {
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" | "mjs" => "application/javascript",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "pdf" => "application/pdf",
        "xml" => "application/xml",
        "txt" => "text/plain",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }.to_string()
}
