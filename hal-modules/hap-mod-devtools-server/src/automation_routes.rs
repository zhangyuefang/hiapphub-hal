use crate::js_codegen;
use crate::ws_manager::WsManager;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::server::AppState;

fn error_json(code: &str, message: &str) -> Value {
    json!({ "error": { "code": code, "message": message, "details": null } })
}

fn err_response(status: StatusCode, code: &str, message: &str) -> Response<Body> {
    (status, Json(error_json(code, message))).into_response()
}

const SELF_APP_ID: &str = "hiapphub-devtools";

#[derive(Deserialize)]
pub struct RunningApp {
    pub app_id: String,
    pub manifest_path: Option<String>,
    pub status: String,
    pub windows: Vec<String>,
}

async fn get_all_apps(ws: &WsManager) -> Vec<Value> {
    let clients = ws.get_clients_info().await;
    let mut apps: Vec<Value> = Vec::new();
    let mut has_self = false;
    for c in &clients {
        if let Some(app_id) = c.get("appId").and_then(|a| a.as_str()) {
            if c.get("role").and_then(|r| r.as_str()) == Some("runner") {
                if app_id == SELF_APP_ID { has_self = true; }
                apps.push(json!({
                    "appId": app_id,
                    "manifestPath": c.get("manifestPath"),
                    "status": "running",
                    "windows": [c.get("label").and_then(|l| l.as_str()).unwrap_or("main")]
                }));
            }
        }
    }
    if !has_self {
        apps.insert(0, json!({
            "appId": SELF_APP_ID,
            "status": "running",
            "windows": ["main"]
        }));
    }
    apps
}

async fn exec_for_app(ws: &WsManager, app_id: &str, action: &str, params: Option<Value>, timeout_ms: u64) -> Result<Value, String> {
    if app_id == SELF_APP_ID {
        return local_exec(action, params).await;
    }
    ws.send_api_request(app_id, action, params, timeout_ms).await
}

async fn local_exec(action: &str, _params: Option<Value>) -> Result<Value, String> {
    match action {
        "eval" => {
            Ok(json!({ "result": "ERROR: eval not supported on server side" }))
        }
        "get_bounds" => Ok(json!({ "x": 0, "y": 0, "width": 1000, "height": 700 })),
        "screenshot" => Ok(json!({ "error": "screenshot not supported for DevTools self" })),
        "resize" | "move" => Ok(json!({ "success": true })),
        _ => Ok(json!({ "error": format!("unsupported self action: {}", action) })),
    }
}

fn find_app<'a>(apps: &'a [Value], app_id: &str) -> Option<&'a Value> {
    apps.iter().find(|a| a.get("appId").and_then(|id| id.as_str()) == Some(app_id))
}

pub async fn list_apps(State(state): State<Arc<AppState>>, Query(query): Query<HashMap<String, String>>) -> impl IntoResponse {
    let apps = get_all_apps(&state.ws).await;
    let mut filtered = apps;
    if let Some(mp) = query.get("manifestPath") {
        filtered.retain(|a| a.get("manifestPath").and_then(|p| p.as_str()) == Some(mp));
    }
    if let Some(aid) = query.get("appId") {
        filtered.retain(|a| a.get("appId").and_then(|p| p.as_str()) == Some(aid.as_str()));
    }
    Json(json!({ "apps": filtered }))
}

pub async fn get_app(State(state): State<Arc<AppState>>, Path(app_id): Path<String>) -> Response<Body> {
    let apps = get_all_apps(&state.ws).await;
    match find_app(&apps, &app_id) {
        Some(app) => Json(app.clone()).into_response(),
        None => err_response(StatusCode::NOT_FOUND, "NOT_FOUND", &format!("App {} is not running", app_id)),
    }
}

pub async fn get_windows(State(state): State<Arc<AppState>>, Path(app_id): Path<String>) -> Response<Body> {
    let apps = get_all_apps(&state.ws).await;
    match find_app(&apps, &app_id) {
        Some(app) => {
            let windows: Vec<Value> = app.get("windows").and_then(|w| w.as_array())
                .map(|ws| ws.iter().map(|w| json!({ "label": w })).collect())
                .unwrap_or_default();
            Json(json!({ "windows": windows })).into_response()
        }
        None => err_response(StatusCode::NOT_FOUND, "NOT_FOUND", &format!("App {} is not running", app_id)),
    }
}

pub async fn get_bounds(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>) -> Response<Body> {
    let apps = get_all_apps(&state.ws).await;
    if find_app(&apps, &app_id).is_none() {
        return err_response(StatusCode::NOT_FOUND, "NOT_FOUND", &format!("App {} is not running", app_id));
    }
    match exec_for_app(&state.ws, &app_id, "get_bounds", Some(json!({ "label": label })), 5000).await {
        Ok(data) => Json(data).into_response(),
        Err(e) => err_response(StatusCode::GATEWAY_TIMEOUT, "TIMEOUT", &e),
    }
}

pub async fn screenshot(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>, Query(query): Query<HashMap<String, String>>) -> Response<Body> {
    let apps = get_all_apps(&state.ws).await;
    if find_app(&apps, &app_id).is_none() {
        return err_response(StatusCode::NOT_FOUND, "NOT_FOUND", &format!("App {} is not running", app_id));
    }
    match exec_for_app(&state.ws, &app_id, "screenshot", Some(json!({ "label": label })), 10000).await {
        Ok(data) => {
            if data.get("error").is_some() {
                return err_response(StatusCode::INTERNAL_SERVER_ERROR, "SCREENSHOT_FAILED", data["error"].as_str().unwrap_or("unknown"));
            }
            if query.get("format").map(|f| f.as_str()) == Some("raw") {
                let base64 = data.get("base64").and_then(|b| b.as_str()).unwrap_or("");
                return (StatusCode::OK, [("content-type", "image/png")], base64.to_string()).into_response();
            }
            Json(json!({ "format": "png", "encoding": "base64", "data": data.get("base64").unwrap_or(&Value::Null) })).into_response()
        }
        Err(e) => err_response(StatusCode::GATEWAY_TIMEOUT, "TIMEOUT", &e),
    }
}

async fn eval_on_app(ws: &WsManager, app_id: &str, label: &str, code: &str, timeout_ms: u64) -> Response<Body> {
    match exec_for_app(ws, app_id, "eval", Some(json!({ "label": label, "code": code })), timeout_ms).await {
        Ok(data) => Json(data.get("result").cloned().unwrap_or(json!({ "error": "no result" }))).into_response(),
        Err(e) => err_response(StatusCode::GATEWAY_TIMEOUT, "TIMEOUT", &e),
    }
}

macro_rules! check_app {
    ($state:expr, $app_id:expr) => {{
        let apps = get_all_apps(&$state.ws).await;
        if find_app(&apps, $app_id).is_none() {
            return err_response(StatusCode::NOT_FOUND, "NOT_FOUND", &format!("App {} is not running", $app_id));
        }
    }};
}

pub async fn dom_tree(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>, Query(query): Query<HashMap<String, String>>) -> Response<Body> {
    check_app!(state, &app_id);
    let max_depth = query.get("maxDepth").and_then(|d| d.parse().ok()).unwrap_or(10u32);
    let selector = query.get("selector").map(|s| s.as_str());
    let code = js_codegen::dom_tree(selector, max_depth);
    eval_on_app(&state.ws, &app_id, &label, &code, 8000).await
}

pub async fn dom_query(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>, Query(query): Query<HashMap<String, String>>) -> Response<Body> {
    check_app!(state, &app_id);
    let selector = match query.get("selector") {
        Some(s) => s.as_str(),
        None => return err_response(StatusCode::BAD_REQUEST, "BAD_REQUEST", "selector query param required"),
    };
    let code = js_codegen::dom_query(selector, query.get("type").map(|t| t.as_str()).unwrap_or("auto"), false, 1, query.get("includeStyles").map(|s| s == "true").unwrap_or(false));
    eval_on_app(&state.ws, &app_id, &label, &code, 5000).await
}

pub async fn dom_query_all(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>, Query(query): Query<HashMap<String, String>>) -> Response<Body> {
    check_app!(state, &app_id);
    let selector = match query.get("selector") {
        Some(s) => s.as_str(),
        None => return err_response(StatusCode::BAD_REQUEST, "BAD_REQUEST", "selector query param required"),
    };
    let limit = query.get("limit").and_then(|l| l.parse().ok()).unwrap_or(50usize);
    let code = js_codegen::dom_query(selector, query.get("type").map(|t| t.as_str()).unwrap_or("auto"), true, limit, query.get("includeStyles").map(|s| s == "true").unwrap_or(false));
    eval_on_app(&state.ws, &app_id, &label, &code, 5000).await
}

pub async fn eval(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>, Json(body): Json<Value>) -> Response<Body> {
    check_app!(state, &app_id);
    let code = body.get("code").and_then(|c| c.as_str()).unwrap_or("");
    match exec_for_app(&state.ws, &app_id, "eval", Some(json!({ "label": label, "code": code })), 12000).await {
        Ok(data) => Json(data).into_response(),
        Err(e) => err_response(StatusCode::GATEWAY_TIMEOUT, "TIMEOUT", &e),
    }
}

pub async fn resize(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>, Json(body): Json<Value>) -> Response<Body> {
    check_app!(state, &app_id);
    if body.get("width").is_none() || body.get("height").is_none() {
        return err_response(StatusCode::BAD_REQUEST, "BAD_REQUEST", "width and height required");
    }
    match exec_for_app(&state.ws, &app_id, "resize", Some(json!({ "label": label, "width": body["width"], "height": body["height"] })), 5000).await {
        Ok(data) => Json(data).into_response(),
        Err(e) => err_response(StatusCode::GATEWAY_TIMEOUT, "TIMEOUT", &e),
    }
}

pub async fn move_window(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>, Json(body): Json<Value>) -> Response<Body> {
    check_app!(state, &app_id);
    if body.get("x").is_none() || body.get("y").is_none() {
        return err_response(StatusCode::BAD_REQUEST, "BAD_REQUEST", "x and y required");
    }
    match exec_for_app(&state.ws, &app_id, "move", Some(json!({ "label": label, "x": body["x"], "y": body["y"] })), 5000).await {
        Ok(data) => Json(data).into_response(),
        Err(e) => err_response(StatusCode::GATEWAY_TIMEOUT, "TIMEOUT", &e),
    }
}

pub async fn click(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>, Json(body): Json<Value>) -> Response<Body> {
    check_app!(state, &app_id);
    let code = js_codegen::click(&body);
    eval_on_app(&state.ws, &app_id, &label, &code, 5000).await
}

pub async fn type_text(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>, Json(body): Json<Value>) -> Response<Body> {
    check_app!(state, &app_id);
    if body.get("text").is_none() {
        return err_response(StatusCode::BAD_REQUEST, "BAD_REQUEST", "text required");
    }
    let code = js_codegen::type_text(&body);
    eval_on_app(&state.ws, &app_id, &label, &code, 5000).await
}

pub async fn scroll(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>, Json(body): Json<Value>) -> Response<Body> {
    check_app!(state, &app_id);
    let code = js_codegen::scroll(&body);
    eval_on_app(&state.ws, &app_id, &label, &code, 5000).await
}

pub async fn wait_for_selector(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>, Json(body): Json<Value>) -> Response<Body> {
    check_app!(state, &app_id);
    let selector = match body.get("selector").and_then(|s| s.as_str()) {
        Some(s) => s,
        None => return err_response(StatusCode::BAD_REQUEST, "BAD_REQUEST", "selector required"),
    };
    let timeout = body.get("timeout").and_then(|t| t.as_u64()).unwrap_or(5000).min(30000);
    let code = js_codegen::wait_for_selector(selector, timeout);
    match exec_for_app(&state.ws, &app_id, "eval", Some(json!({ "label": label, "code": code })), timeout + 3000).await {
        Ok(data) => {
            let result = data.get("result").cloned().unwrap_or(json!({ "found": false }));
            if result.get("timeout").and_then(|t| t.as_bool()) == Some(true) {
                return err_response(StatusCode::REQUEST_TIMEOUT, "TIMEOUT", &format!("Selector {} not found within {}ms", selector, timeout));
            }
            Json(result).into_response()
        }
        Err(e) => err_response(StatusCode::GATEWAY_TIMEOUT, "TIMEOUT", &e),
    }
}

pub async fn wait_for_navigation(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>, Json(body): Json<Value>) -> Response<Body> {
    check_app!(state, &app_id);
    let timeout = body.get("timeout").and_then(|t| t.as_u64()).unwrap_or(5000).min(30000);
    let code = js_codegen::wait_for_navigation(timeout);
    match exec_for_app(&state.ws, &app_id, "eval", Some(json!({ "label": label, "code": code })), timeout + 3000).await {
        Ok(data) => {
            let result = data.get("result").cloned().unwrap_or(json!({ "navigated": false }));
            if result.get("timeout").and_then(|t| t.as_bool()) == Some(true) {
                return err_response(StatusCode::REQUEST_TIMEOUT, "TIMEOUT", &format!("No navigation within {}ms", timeout));
            }
            Json(result).into_response()
        }
        Err(e) => err_response(StatusCode::GATEWAY_TIMEOUT, "TIMEOUT", &e),
    }
}

pub async fn wait_for_idle(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>, Json(body): Json<Value>) -> Response<Body> {
    check_app!(state, &app_id);
    let timeout = body.get("timeout").and_then(|t| t.as_u64()).unwrap_or(5000).min(30000);
    let code = js_codegen::wait_for_idle(timeout);
    match exec_for_app(&state.ws, &app_id, "eval", Some(json!({ "label": label, "code": code })), timeout + 3000).await {
        Ok(data) => {
            let result = data.get("result").cloned().unwrap_or(json!({ "idle": false }));
            if result.get("timeout").and_then(|t| t.as_bool()) == Some(true) {
                return err_response(StatusCode::REQUEST_TIMEOUT, "TIMEOUT", &format!("Not idle within {}ms", timeout));
            }
            Json(result).into_response()
        }
        Err(e) => err_response(StatusCode::GATEWAY_TIMEOUT, "TIMEOUT", &e),
    }
}

pub async fn console_start(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>) -> Response<Body> {
    check_app!(state, &app_id);
    let code = js_codegen::console_start();
    eval_on_app(&state.ws, &app_id, &label, &code, 5000).await
}

pub async fn console_logs(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>, Query(query): Query<HashMap<String, String>>) -> Response<Body> {
    check_app!(state, &app_id);
    let since = query.get("since").and_then(|s| s.parse().ok()).unwrap_or(0u64);
    let level = query.get("level").map(|l| l.as_str()).unwrap_or("");
    let code = js_codegen::console_logs(since, level);
    eval_on_app(&state.ws, &app_id, &label, &code, 5000).await
}

pub async fn dom_observe(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>, Json(body): Json<Value>) -> Response<Body> {
    check_app!(state, &app_id);
    let selector = body.get("selector").and_then(|s| s.as_str()).unwrap_or("body");
    let opts = body.get("options").cloned().unwrap_or(json!({}));
    let code = js_codegen::dom_observe(
        selector,
        opts.get("childList").and_then(|v| v.as_bool()).unwrap_or(true),
        opts.get("attributes").and_then(|v| v.as_bool()).unwrap_or(true),
        opts.get("subtree").and_then(|v| v.as_bool()).unwrap_or(true),
        opts.get("characterData").and_then(|v| v.as_bool()).unwrap_or(false),
    );
    eval_on_app(&state.ws, &app_id, &label, &code, 5000).await
}

pub async fn dom_mutations(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>, Query(query): Query<HashMap<String, String>>) -> Response<Body> {
    check_app!(state, &app_id);
    let since = query.get("since").and_then(|s| s.parse().ok()).unwrap_or(0u64);
    let clear = query.get("clear").map(|c| c == "true").unwrap_or(false);
    let code = js_codegen::dom_mutations(since, clear);
    eval_on_app(&state.ws, &app_id, &label, &code, 5000).await
}

pub async fn dom_observe_stop(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>) -> Response<Body> {
    check_app!(state, &app_id);
    let code = js_codegen::dom_observe_stop();
    eval_on_app(&state.ws, &app_id, &label, &code, 5000).await
}

pub async fn dom_snapshot(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>, Json(body): Json<Value>) -> Response<Body> {
    check_app!(state, &app_id);
    let selector = body.get("selector").and_then(|s| s.as_str()).unwrap_or("body");
    let max_depth = body.get("maxDepth").and_then(|d| d.as_u64()).unwrap_or(5) as u32;
    let code = js_codegen::snapshot(selector, max_depth);
    eval_on_app(&state.ws, &app_id, &label, &code, 5000).await
}

pub async fn dom_diff(_state: State<Arc<AppState>>, Json(body): Json<Value>) -> Response<Body> {
    let before = body.get("before");
    let after = body.get("after");
    if before.is_none() || after.is_none() {
        return err_response(StatusCode::BAD_REQUEST, "BAD_REQUEST", "before and after snapshots required");
    }
    let diffs = diff_snapshots(before.unwrap(), after.unwrap(), "");
    Json(json!({ "changes": diffs, "count": diffs.len() })).into_response()
}

fn diff_snapshots(before: &Value, after: &Value, path: &str) -> Vec<Value> {
    let mut diffs = Vec::new();
    if before.is_null() && !after.is_null() {
        diffs.push(json!({ "type": "added", "path": path, "node": after }));
        return diffs;
    }
    if !before.is_null() && after.is_null() {
        diffs.push(json!({ "type": "removed", "path": path, "node": before }));
        return diffs;
    }
    if before.is_null() && after.is_null() { return diffs; }

    for field in &["tag", "id", "cls", "text"] {
        if before.get(field) != after.get(field) {
            diffs.push(json!({ "type": "changed", "path": path, "field": field, "before": before.get(field), "after": after.get(field) }));
        }
    }
    let bc = before.get("children").and_then(|c| c.as_array());
    let ac = after.get("children").and_then(|c| c.as_array());
    let bc_len = bc.map(|a| a.len()).unwrap_or(0);
    let ac_len = ac.map(|a| a.len()).unwrap_or(0);
    let max_len = bc_len.max(ac_len);
    for i in 0..max_len {
        let b_child = bc.and_then(|a| a.get(i)).unwrap_or(&Value::Null);
        let a_child = ac.and_then(|a| a.get(i)).unwrap_or(&Value::Null);
        let tag = a_child.get("tag").or(b_child.get("tag")).and_then(|t| t.as_str()).unwrap_or("*");
        let child_path = format!("{}/{}[{}]", path, tag, i);
        diffs.extend(diff_snapshots(b_child, a_child, &child_path));
    }
    diffs
}

pub async fn accessibility(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>, Query(query): Query<HashMap<String, String>>) -> Response<Body> {
    check_app!(state, &app_id);
    let selector = query.get("selector").map(|s| s.as_str()).unwrap_or("body");
    let max_depth = query.get("maxDepth").and_then(|d| d.parse().ok()).unwrap_or(5u32);
    let code = js_codegen::accessibility(selector, max_depth);
    eval_on_app(&state.ws, &app_id, &label, &code, 5000).await
}

pub async fn perf(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>) -> Response<Body> {
    check_app!(state, &app_id);
    let code = js_codegen::performance();
    eval_on_app(&state.ws, &app_id, &label, &code, 5000).await
}

pub async fn network_start(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>) -> Response<Body> {
    check_app!(state, &app_id);
    let code = js_codegen::network_start();
    eval_on_app(&state.ws, &app_id, &label, &code, 5000).await
}

pub async fn network_requests(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>, Query(query): Query<HashMap<String, String>>) -> Response<Body> {
    check_app!(state, &app_id);
    let since = query.get("since").and_then(|s| s.parse().ok()).unwrap_or(0u64);
    let clear = query.get("clear").map(|c| c == "true").unwrap_or(false);
    let code = js_codegen::network_requests(since, clear);
    eval_on_app(&state.ws, &app_id, &label, &code, 5000).await
}

pub async fn network_stop(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>) -> Response<Body> {
    check_app!(state, &app_id);
    let code = js_codegen::network_stop();
    eval_on_app(&state.ws, &app_id, &label, &code, 5000).await
}

pub async fn storage_get(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>, Query(query): Query<HashMap<String, String>>) -> Response<Body> {
    check_app!(state, &app_id);
    let stype = query.get("type").map(|t| t.as_str()).unwrap_or("local");
    let key = query.get("key").map(|k| k.as_str());
    let code = js_codegen::storage_get(stype, key);
    eval_on_app(&state.ws, &app_id, &label, &code, 5000).await
}

pub async fn storage_set(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>, Json(body): Json<Value>) -> Response<Body> {
    check_app!(state, &app_id);
    if body.get("action").is_none() {
        return err_response(StatusCode::BAD_REQUEST, "BAD_REQUEST", "action required (set/remove/clear)");
    }
    let code = js_codegen::storage_set(&body);
    eval_on_app(&state.ws, &app_id, &label, &code, 5000).await
}

pub async fn mock_set(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>, Json(body): Json<Value>) -> Response<Body> {
    check_app!(state, &app_id);
    let module = match body.get("module").and_then(|m| m.as_str()) {
        Some(m) => m,
        None => return err_response(StatusCode::BAD_REQUEST, "BAD_REQUEST", "module and command required"),
    };
    let command = match body.get("command").and_then(|c| c.as_str()) {
        Some(c) => c,
        None => return err_response(StatusCode::BAD_REQUEST, "BAD_REQUEST", "module and command required"),
    };
    let response = body.get("response").unwrap_or(&Value::Null);
    let code = js_codegen::mock_set(module, command, response);
    eval_on_app(&state.ws, &app_id, &label, &code, 5000).await
}

pub async fn mock_clear(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>, Json(body): Json<Value>) -> Response<Body> {
    check_app!(state, &app_id);
    let module = body.get("module").and_then(|m| m.as_str());
    let command = body.get("command").and_then(|c| c.as_str());
    let code = js_codegen::mock_clear(module, command);
    eval_on_app(&state.ws, &app_id, &label, &code, 5000).await
}

pub async fn mock_list(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>) -> Response<Body> {
    check_app!(state, &app_id);
    let code = js_codegen::mock_list();
    eval_on_app(&state.ws, &app_id, &label, &code, 5000).await
}

pub async fn batch(State(state): State<Arc<AppState>>, Path((app_id, label)): Path<(String, String)>, Json(body): Json<Value>) -> Response<Body> {
    check_app!(state, &app_id);
    let steps = match body.get("steps").and_then(|s| s.as_array()) {
        Some(s) if !s.is_empty() => s,
        _ => return err_response(StatusCode::BAD_REQUEST, "BAD_REQUEST", "steps array required"),
    };
    if steps.len() > 20 {
        return err_response(StatusCode::BAD_REQUEST, "BAD_REQUEST", "max 20 steps per batch");
    }
    let stop_on_error = body.get("stopOnError").and_then(|s| s.as_bool()).unwrap_or(false);
    let mut results: Vec<Value> = Vec::new();
    let mut aborted = false;
    for step in steps {
        if aborted { results.push(json!({ "skipped": true })); continue; }
        let code = match js_codegen::batch_step(step) {
            Some(c) => c,
            None => {
                let action = step.get("action").and_then(|a| a.as_str()).unwrap_or("?");
                results.push(json!({ "error": format!("unknown action: {}", action) }));
                if stop_on_error { aborted = true; }
                continue;
            }
        };
        match exec_for_app(&state.ws, &app_id, "eval", Some(json!({ "label": label, "code": code })), 12000).await {
            Ok(data) => results.push(data.get("result").cloned().unwrap_or(Value::Null)),
            Err(e) => {
                results.push(json!({ "error": e }));
                if stop_on_error { aborted = true; }
            }
        }
        if let Some(delay) = step.get("delay").and_then(|d| d.as_u64()) {
            tokio::time::sleep(std::time::Duration::from_millis(delay.min(5000))).await;
        }
    }
    Json(json!({ "results": results, "count": results.len(), "aborted": aborted })).into_response()
}
