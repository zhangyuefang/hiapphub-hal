use axum::body::Body;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::oneshot;

use crate::server::AppState;

pub struct CallbackRequest {
    pub request_id: String,
    pub method: String,
    pub path: String,
    pub body: Value,
    pub respond_tx: oneshot::Sender<CallbackResponse>,
}

pub struct CallbackResponse {
    pub status: u16,
    pub body: String,
}

fn error_json(code: &str, message: &str) -> Value {
    json!({ "error": { "code": code, "message": message, "details": null } })
}

async fn forward_to_webview(
    state: &Arc<AppState>,
    method: &str,
    path: &str,
    body: Value,
    timeout_ms: u64,
) -> Response<Body> {
    let request_id = format!("cb_{}_{}", millis(), &uuid::Uuid::new_v4().to_string()[..8]);
    let (tx, rx) = oneshot::channel();

    let cb_req = CallbackRequest {
        request_id: request_id.clone(),
        method: method.to_string(),
        path: path.to_string(),
        body,
        respond_tx: tx,
    };

    {
        let mut pending = state.pending_callbacks.lock().await;
        pending.push(cb_req);
    }

    match tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), rx).await {
        Ok(Ok(resp)) => {
            let status = StatusCode::from_u16(resp.status).unwrap_or(StatusCode::OK);
            (status, [("content-type", "application/json")], resp.body).into_response()
        }
        Ok(Err(_)) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error_json("INTERNAL", "callback channel closed"))).into_response()
        }
        Err(_) => {
            let mut pending = state.pending_callbacks.lock().await;
            pending.retain(|r| r.request_id != request_id);
            (StatusCode::GATEWAY_TIMEOUT, Json(error_json("TIMEOUT", "webview callback timed out"))).into_response()
        }
    }
}

pub async fn devtools_state(State(state): State<Arc<AppState>>) -> Response<Body> {
    forward_to_webview(&state, "GET", "/api/v1/devtools/state", json!({}), 10000).await
}

pub async fn devtools_projects(State(state): State<Arc<AppState>>) -> Response<Body> {
    forward_to_webview(&state, "GET", "/api/v1/devtools/projects", json!({}), 10000).await
}

pub async fn workspace_open(State(state): State<Arc<AppState>>, Json(body): Json<Value>) -> Response<Body> {
    if body.get("dir").is_none() {
        return (StatusCode::BAD_REQUEST, Json(error_json("MISSING_PARAM", "\"dir\" is required"))).into_response();
    }
    forward_to_webview(&state, "POST", "/api/v1/devtools/workspace/open", body, 10000).await
}

pub async fn workspace_create(State(state): State<Arc<AppState>>, Json(body): Json<Value>) -> Response<Body> {
    if body.get("dir").is_none() || body.get("name").is_none() {
        return (StatusCode::BAD_REQUEST, Json(error_json("MISSING_PARAM", "\"dir\" and \"name\" are required"))).into_response();
    }
    forward_to_webview(&state, "POST", "/api/v1/devtools/workspace/create", body, 10000).await
}

pub async fn workspace_close(State(state): State<Arc<AppState>>) -> Response<Body> {
    forward_to_webview(&state, "POST", "/api/v1/devtools/workspace/close", json!({}), 10000).await
}

pub async fn project_add(State(state): State<Arc<AppState>>, Json(body): Json<Value>) -> Response<Body> {
    if body.get("id").is_none() {
        return (StatusCode::BAD_REQUEST, Json(error_json("MISSING_PARAM", "\"id\" is required"))).into_response();
    }
    forward_to_webview(&state, "POST", "/api/v1/devtools/project/add", body, 10000).await
}

pub async fn project_open(State(state): State<Arc<AppState>>, Json(body): Json<Value>) -> Response<Body> {
    if body.get("id").is_none() {
        return (StatusCode::BAD_REQUEST, Json(error_json("MISSING_PARAM", "\"id\" is required"))).into_response();
    }
    forward_to_webview(&state, "POST", "/api/v1/devtools/project/open", body, 10000).await
}

pub async fn project_close(State(state): State<Arc<AppState>>, Json(body): Json<Value>) -> Response<Body> {
    if body.get("id").is_none() {
        return (StatusCode::BAD_REQUEST, Json(error_json("MISSING_PARAM", "\"id\" is required"))).into_response();
    }
    forward_to_webview(&state, "POST", "/api/v1/devtools/project/close", body, 10000).await
}

pub async fn project_start(State(state): State<Arc<AppState>>, Json(body): Json<Value>) -> Response<Body> {
    forward_to_webview(&state, "POST", "/api/v1/devtools/project/start", body, 30000).await
}

pub async fn project_stop(State(state): State<Arc<AppState>>, Json(body): Json<Value>) -> Response<Body> {
    forward_to_webview(&state, "POST", "/api/v1/devtools/project/stop", body, 10000).await
}

pub async fn projects_start(State(state): State<Arc<AppState>>, Json(body): Json<Value>) -> Response<Body> {
    forward_to_webview(&state, "POST", "/api/v1/projects/start", body, 30000).await
}

fn millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
