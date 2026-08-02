use crate::automation_routes;
use crate::callback_routes::{self, CallbackRequest};
use crate::ws_manager::{WsClient, WsManager};
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::extract::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tower_http::cors::CorsLayer;

pub struct AppState {
    pub ws: Arc<WsManager>,
    pub token: String,
    pub pending_callbacks: Mutex<Vec<CallbackRequest>>,
    pub internal_requests: Mutex<Vec<InternalRequest>>,
    pub internal_routes: RwLock<Vec<(String, String)>>,
    pub eval_queue: Mutex<Vec<EvalEntry>>,
}

pub struct EvalEntry {
    pub id: String,
    pub app_id: String,
    pub script: String,
    pub result_tx: Option<tokio::sync::oneshot::Sender<String>>,
}

pub struct InternalRequest {
    pub request_id: String,
    pub method: String,
    pub uri: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub respond_tx: Option<tokio::sync::oneshot::Sender<InternalResponse>>,
}

pub struct InternalResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

struct ServerHandles {
    http_handle: tokio::task::JoinHandle<()>,
    ws_handle: tokio::task::JoinHandle<()>,
    internal_handle: tokio::task::JoinHandle<()>,
    state: Arc<AppState>,
    http_port: u16,
    ws_port: u16,
    internal_port: u16,
}

static SERVER: std::sync::OnceLock<Mutex<Option<ServerHandles>>> = std::sync::OnceLock::new();

fn get_server_lock() -> &'static Mutex<Option<ServerHandles>> {
    SERVER.get_or_init(|| Mutex::new(None))
}

fn generate_token() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".chars().collect();
    (0..32).map(|_| chars[rng.gen_range(0..chars.len())]).collect()
}

pub fn runtime() -> &'static tokio::runtime::Runtime {
    static RT: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .build()
            .expect("tokio runtime")
    })
}

pub fn start(http_port: u16, ws_port: u16, internal_port: u16) -> Result<Value, String> {
    let rt = runtime();
    rt.block_on(async {
        let mut lock = get_server_lock().lock().await;
        if lock.is_some() {
            return Err("server already running".into());
        }

        let token = generate_token();
        let ws_mgr = WsManager::new();
        let state = Arc::new(AppState {
            ws: ws_mgr.clone(),
            token: token.clone(),
            pending_callbacks: Mutex::new(Vec::new()),
            internal_requests: Mutex::new(Vec::new()),
            internal_routes: RwLock::new(Vec::new()),
            eval_queue: Mutex::new(Vec::new()),
        });

        let http_state = state.clone();
        let http_handle = tokio::spawn(async move {
            run_http_server(http_state, http_port).await;
        });

        let ws_state = state.clone();
        let ws_handle = tokio::spawn(async move {
            run_ws_server(ws_state, ws_port).await;
        });

        let int_state = state.clone();
        let int_handle = tokio::spawn(async move {
            run_internal_server(int_state, internal_port).await;
        });

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        write_port_file(http_port, &token);

        *lock = Some(ServerHandles {
            http_handle,
            ws_handle,
            internal_handle: int_handle,
            state: state.clone(),
            http_port,
            ws_port,
            internal_port,
        });

        Ok(json!({
            "token": token,
            "http_port": http_port,
            "ws_port": ws_port,
            "internal_port": internal_port,
        }))
    })
}

pub fn stop() -> Result<Value, String> {
    let rt = runtime();
    rt.block_on(async {
        let mut lock = get_server_lock().lock().await;
        if let Some(handles) = lock.take() {
            handles.http_handle.abort();
            handles.ws_handle.abort();
            handles.internal_handle.abort();
            let ws = &handles.state.ws;
            ws.send_to_role("runner", &json!({ "type": "devtools:shutdown" })).await;
        }
        Ok(json!(true))
    })
}

pub fn status() -> Result<Value, String> {
    let rt = runtime();
    rt.block_on(async {
        let lock = get_server_lock().lock().await;
        match lock.as_ref() {
            Some(h) => Ok(json!({
                "running": true,
                "http_port": h.http_port,
                "ws_port": h.ws_port,
                "internal_port": h.internal_port,
                "token": h.state.token,
            })),
            None => Ok(json!({ "running": false })),
        }
    })
}

pub fn get_state() -> Result<Arc<AppState>, String> {
    let rt = runtime();
    rt.block_on(async {
        let lock = get_server_lock().lock().await;
        lock.as_ref().map(|h| h.state.clone()).ok_or_else(|| "server not running".into())
    })
}

async fn auth_middleware(
    token: &str,
    headers: &axum::http::HeaderMap,
) -> Option<axum::response::Response<axum::body::Body>> {
    let auth = headers.get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let bearer = auth.strip_prefix("Bearer ").unwrap_or(auth);
    if bearer != token {
        return Some((
            StatusCode::UNAUTHORIZED,
            axum::Json(json!({ "error": { "code": "UNAUTHORIZED", "message": "Invalid or missing token", "details": null } })),
        ).into_response());
    }
    None
}

async fn run_http_server(state: Arc<AppState>, port: u16) {
    let token = state.token.clone();
    let app = Router::new()
        .route("/api/v1/apps", get(automation_routes::list_apps))
        .route("/api/v1/apps/{appId}", get(automation_routes::get_app))
        .route("/api/v1/apps/{appId}/windows", get(automation_routes::get_windows))
        .route("/api/v1/apps/{appId}/windows/{label}/bounds", get(automation_routes::get_bounds))
        .route("/api/v1/apps/{appId}/windows/{label}/screenshot", get(automation_routes::screenshot))
        .route("/api/v1/apps/{appId}/windows/{label}/dom", get(automation_routes::dom_tree))
        .route("/api/v1/apps/{appId}/windows/{label}/dom/query", get(automation_routes::dom_query))
        .route("/api/v1/apps/{appId}/windows/{label}/dom/query-all", get(automation_routes::dom_query_all))
        .route("/api/v1/apps/{appId}/windows/{label}/dom/snapshot", post(automation_routes::dom_snapshot))
        .route("/api/v1/apps/{appId}/windows/{label}/dom/diff", post(automation_routes::dom_diff))
        .route("/api/v1/apps/{appId}/windows/{label}/dom/observe", post(automation_routes::dom_observe))
        .route("/api/v1/apps/{appId}/windows/{label}/dom/mutations", get(automation_routes::dom_mutations))
        .route("/api/v1/apps/{appId}/windows/{label}/dom/observe/stop", post(automation_routes::dom_observe_stop))
        .route("/api/v1/apps/{appId}/windows/{label}/eval", post(automation_routes::eval))
        .route("/api/v1/apps/{appId}/windows/{label}/resize", post(automation_routes::resize))
        .route("/api/v1/apps/{appId}/windows/{label}/move", post(automation_routes::move_window))
        .route("/api/v1/apps/{appId}/windows/{label}/click", post(automation_routes::click))
        .route("/api/v1/apps/{appId}/windows/{label}/type", post(automation_routes::type_text))
        .route("/api/v1/apps/{appId}/windows/{label}/scroll", post(automation_routes::scroll))
        .route("/api/v1/apps/{appId}/windows/{label}/wait-for-selector", post(automation_routes::wait_for_selector))
        .route("/api/v1/apps/{appId}/windows/{label}/wait-for-navigation", post(automation_routes::wait_for_navigation))
        .route("/api/v1/apps/{appId}/windows/{label}/wait-for-idle", post(automation_routes::wait_for_idle))
        .route("/api/v1/apps/{appId}/windows/{label}/console/start", post(automation_routes::console_start))
        .route("/api/v1/apps/{appId}/windows/{label}/console/logs", get(automation_routes::console_logs))
        .route("/api/v1/apps/{appId}/windows/{label}/accessibility", get(automation_routes::accessibility))
        .route("/api/v1/apps/{appId}/windows/{label}/performance", get(automation_routes::perf))
        .route("/api/v1/apps/{appId}/windows/{label}/network/start", post(automation_routes::network_start))
        .route("/api/v1/apps/{appId}/windows/{label}/network/requests", get(automation_routes::network_requests))
        .route("/api/v1/apps/{appId}/windows/{label}/network/stop", post(automation_routes::network_stop))
        .route("/api/v1/apps/{appId}/windows/{label}/storage", get(automation_routes::storage_get).post(automation_routes::storage_set))
        .route("/api/v1/apps/{appId}/windows/{label}/mock/set", post(automation_routes::mock_set))
        .route("/api/v1/apps/{appId}/windows/{label}/mock/clear", post(automation_routes::mock_clear))
        .route("/api/v1/apps/{appId}/windows/{label}/mock/list", get(automation_routes::mock_list))
        .route("/api/v1/apps/{appId}/windows/{label}/batch", post(automation_routes::batch))
        .route("/api/v1/devtools/state", get(callback_routes::devtools_state))
        .route("/api/v1/devtools/projects", get(callback_routes::devtools_projects))
        .route("/api/v1/devtools/workspace/open", post(callback_routes::workspace_open))
        .route("/api/v1/devtools/workspace/create", post(callback_routes::workspace_create))
        .route("/api/v1/devtools/workspace/close", post(callback_routes::workspace_close))
        .route("/api/v1/devtools/project/add", post(callback_routes::project_add))
        .route("/api/v1/devtools/project/open", post(callback_routes::project_open))
        .route("/api/v1/devtools/project/close", post(callback_routes::project_close))
        .route("/api/v1/devtools/project/start", post(callback_routes::project_start))
        .route("/api/v1/devtools/project/stop", post(callback_routes::project_stop))
        .route("/api/v1/projects/start", post(callback_routes::projects_start))
        .route("/api/v1/ohos/eval/pending", get(ohos_eval_pending))
        .route("/api/v1/ohos/eval/result", post(ohos_eval_result))
        .route("/api/v1/ohos/logs", post(ohos_log_broadcast))
        .route("/api/v1/ohos/eval", post(ohos_eval_submit))
        .layer(axum::middleware::from_fn(move |req: axum::extract::Request, next: axum::middleware::Next| {
            let t = token.clone();
            async move {
                let path = req.uri().path().to_string();
                if path.starts_with("/api/v1/ohos/eval/pending") || path.starts_with("/api/v1/ohos/eval/result") || path.starts_with("/api/v1/ohos/logs") {
                    return next.run(req).await;
                }
                if let Some(resp) = auth_middleware(&t, req.headers()).await {
                    return resp;
                }
                next.run(req).await
            }
        }))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.ok();
}

async fn ohos_eval_pending(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let queue = state.eval_queue.lock().await;
    let pending: Vec<Value> = queue.iter().map(|e| json!({"id": e.id, "appId": e.app_id, "script": e.script})).collect();
    Json(json!({"pending": pending}))
}

async fn ohos_eval_result(State(state): State<Arc<AppState>>, Json(body): Json<Value>) -> impl IntoResponse {
    let id = body.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let result = body.get("result").and_then(|v| v.as_str()).unwrap_or("null");
    let mut queue = state.eval_queue.lock().await;
    if let Some(pos) = queue.iter().position(|e| e.id == id) {
        let entry = queue.remove(pos);
        if let Some(tx) = entry.result_tx {
            let _ = tx.send(result.to_string());
        }
    }
    // Also broadcast to WS clients
    let _ = state.ws.broadcast(&json!({"type": "custom", "event": "hap:eval:response", "data": {"id": id, "result": result}})).await;
    Json(json!({"ok": true}))
}

async fn ohos_log_broadcast(State(state): State<Arc<AppState>>, Json(body): Json<Value>) -> impl IntoResponse {
    let _ = state.ws.broadcast(&json!({"type": "custom", "event": "hap:log", "data": body})).await;
    Json(json!({"ok": true}))
}

async fn ohos_eval_submit(State(state): State<Arc<AppState>>, Json(body): Json<Value>) -> impl IntoResponse {
    let app_id = body.get("appId").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let script = body.get("script").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if script.is_empty() {
        return Json(json!({"error": "script required"}));
    }
    let id = format!("eval_{}_{}", millis(), &uuid::Uuid::new_v4().to_string()[..8]);
    let (tx, rx) = tokio::sync::oneshot::channel();
    {
        let mut queue = state.eval_queue.lock().await;
        queue.push(EvalEntry { id: id.clone(), app_id, script, result_tx: Some(tx) });
    }
    match tokio::time::timeout(std::time::Duration::from_secs(10), rx).await {
        Ok(Ok(result)) => Json(json!({"id": id, "result": result})),
        _ => {
            let mut queue = state.eval_queue.lock().await;
            queue.retain(|e| e.id != id);
            Json(json!({"id": id, "error": "timeout"}))
        }
    }
}

async fn ws_upgrade_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws_connection(socket, state))
}

async fn handle_ws_connection(socket: WebSocket, state: Arc<AppState>) {
    let client_id = uuid::Uuid::new_v4().to_string();
    let (ws_tx, mut ws_rx) = socket.split();
    let (msg_tx, mut msg_rx) = mpsc::unbounded_channel::<String>();

    let client = WsClient {
        client_id: client_id.clone(),
        role: None,
        window_label: None,
        app_id: None,
        manifest_path: None,
        tx: msg_tx,
    };
    state.ws.add_client(client).await;

    let mut ws_tx = ws_tx;
    let send_task = tokio::spawn(async move {
        while let Some(msg) = msg_rx.recv().await {
            if ws_tx.send(Message::Text(msg.into())).await.is_err() { break; }
        }
    });

    while let Some(result) = ws_rx.next().await {
        match result {
            Ok(Message::Text(text)) => {
                if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&text) {
                    if msg.get("type").and_then(|t| t.as_str()) == Some("get_token") {
                        let resp = serde_json::json!({"type":"token","token":&state.token});
                        let _ = state.ws.send_to_client(&client_id, &resp).await;
                        continue;
                    }
                }
                state.ws.handle_message(&client_id, &text).await;
            }
            Ok(Message::Close(_)) | Err(_) => break,
            _ => {}
        }
    }

    state.ws.remove_client(&client_id).await;
    send_task.abort();
}

async fn run_ws_server(state: Arc<AppState>, port: u16) {
    let app = Router::new()
        .route("/", get(ws_upgrade_handler))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.ok();
}

async fn run_internal_server(state: Arc<AppState>, port: u16) {
    let app = Router::new()
        .fallback(move |req: axum::extract::Request| {
            let st = state.clone();
            async move {
                let method = req.method().to_string();
                let uri = req.uri().to_string();
                let hdrs: HashMap<String, String> = req.headers().iter()
                    .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                    .collect();
                let body_bytes = axum::body::to_bytes(req.into_body(), 1024 * 1024).await.unwrap_or_default();
                let body = String::from_utf8_lossy(&body_bytes).to_string();

                let request_id = format!("int_{}_{}", millis(), &uuid::Uuid::new_v4().to_string()[..8]);
                let (tx, rx) = tokio::sync::oneshot::channel();

                {
                    let mut reqs = st.internal_requests.lock().await;
                    reqs.push(InternalRequest {
                        request_id: request_id.clone(),
                        method,
                        uri,
                        headers: hdrs,
                        body,
                        respond_tx: Some(tx),
                    });
                }

                match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
                    Ok(Ok(resp)) => {
                        let status = StatusCode::from_u16(resp.status).unwrap_or(StatusCode::OK);
                        let mut builder = axum::response::Response::builder().status(status);
                        for (k, v) in &resp.headers {
                            builder = builder.header(k.as_str(), v.as_str());
                        }
                        if !resp.headers.contains_key("content-type") {
                            builder = builder.header("content-type", "application/json");
                        }
                        builder.body(axum::body::Body::from(resp.body)).unwrap_or_default()
                    }
                    _ => {
                        let mut reqs = st.internal_requests.lock().await;
                        reqs.retain(|r| r.request_id != request_id);
                        (StatusCode::GATEWAY_TIMEOUT, "timeout").into_response()
                    }
                }
            }
        })
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.ok();
}

fn write_port_file(port: u16, token: &str) {
    let content = serde_json::to_string(&json!({ "port": port, "token": token })).unwrap_or_default();
    let candidates: Vec<Option<String>> = vec![
        std::env::var("HOME").ok().filter(|h| h != "/").map(|h| format!("{}/.hiapphub", h)),
        std::env::current_dir().ok().map(|d| format!("{}/.hiapphub", d.display())),
        Some("/data/storage/el2/base/haps/entry/files/.hiapphub".to_string()),
        Some("/data/storage/el2/base/files/.hiapphub".to_string()),
    ];
    for dir_opt in &candidates {
        if let Some(dir) = dir_opt {
            let _ = std::fs::create_dir_all(dir);
            let path = format!("{}/devtools.port", dir);
            if std::fs::write(&path, &content).is_ok() {
                return;
            }
        }
    }
}

fn millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
