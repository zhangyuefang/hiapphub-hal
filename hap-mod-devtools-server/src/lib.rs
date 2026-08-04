pub mod automation_routes;
pub mod callback_routes;
pub mod js_codegen;
pub mod server;
pub mod ws_manager;

use hap_common::{hap_fn, ffi::str_to_c};
use serde::Deserialize;
use serde_json::Value;
use std::ffi::c_char;

hap_common::hap_module_init!("devtools_server");
hap_common::hap_free_string!();

#[no_mangle]
pub extern "C" fn hap_module_describe() -> *const c_char {
    str_to_c(include_str!("../manifest.json"))
}

#[derive(Deserialize)]
pub struct StartParams {
    pub http_port: Option<u16>,
    pub ws_port: Option<u16>,
    pub internal_port: Option<u16>,
}

hap_fn!(hap_devtools_server_start, StartParams, |p| {
    let http = p.http_port.unwrap_or(19769);
    let ws = p.ws_port.unwrap_or(19768);
    let internal = p.internal_port.unwrap_or(19767);
    server::start(http, ws, internal).map_err(hap_common::HapError::internal)
});

#[derive(Deserialize)]
pub struct EmptyParams {}

hap_fn!(hap_devtools_server_stop, EmptyParams, |_p| {
    server::stop().map_err(hap_common::HapError::internal)
});

hap_fn!(hap_devtools_server_status, EmptyParams, |_p| {
    server::status().map_err(hap_common::HapError::internal)
});

hap_fn!(hap_devtools_server_get_ws_clients, EmptyParams, |_p| {
    let state = server::get_state().map_err(hap_common::HapError::internal)?;
    let rt = server::runtime();
    let clients = rt.block_on(state.ws.get_clients_info());
    Ok(serde_json::to_value(clients).unwrap_or(serde_json::json!([])))
});

#[derive(Deserialize)]
pub struct WsSendParams {
    pub client_id: String,
    pub message: String,
}

hap_fn!(hap_devtools_server_ws_send, WsSendParams, |p| {
    let state = server::get_state().map_err(hap_common::HapError::internal)?;
    let msg: Value = serde_json::from_str(&p.message).unwrap_or(Value::String(p.message));
    let rt = server::runtime();
    let ok = rt.block_on(state.ws.send_to_client(&p.client_id, &msg));
    Ok(serde_json::json!(ok))
});

#[derive(Deserialize)]
pub struct WsBroadcastParams {
    pub message: String,
}

hap_fn!(hap_devtools_server_ws_broadcast, WsBroadcastParams, |p| {
    let state = server::get_state().map_err(hap_common::HapError::internal)?;
    let msg: Value = serde_json::from_str(&p.message).unwrap_or(Value::String(p.message));
    let rt = server::runtime();
    rt.block_on(state.ws.broadcast(&msg));
    Ok(serde_json::json!(true))
});

#[derive(Deserialize)]
pub struct WsSendToRoleParams {
    pub role: String,
    pub message: String,
}

hap_fn!(hap_devtools_server_ws_send_to_role, WsSendToRoleParams, |p| {
    let state = server::get_state().map_err(hap_common::HapError::internal)?;
    let msg: Value = serde_json::from_str(&p.message).unwrap_or(Value::String(p.message));
    let rt = server::runtime();
    let count = rt.block_on(state.ws.send_to_role(&p.role, &msg));
    Ok(serde_json::json!(count))
});

#[derive(Deserialize)]
pub struct PollParams {
    pub limit: Option<usize>,
}

hap_fn!(hap_devtools_server_poll_callbacks, PollParams, |p| {
    let state = server::get_state().map_err(hap_common::HapError::internal)?;
    let rt = server::runtime();
    let limit = p.limit.unwrap_or(10);
    let callbacks = rt.block_on(async {
        let pending = state.pending_callbacks.lock().await;
        pending.iter().take(limit).map(|r| serde_json::json!({
            "request_id": r.request_id,
            "method": r.method,
            "path": r.path,
            "body": r.body,
        })).collect::<Vec<_>>()
    });
    Ok(serde_json::to_value(callbacks).unwrap_or(serde_json::json!([])))
});

#[derive(Deserialize)]
pub struct RespondCallbackParams {
    pub request_id: String,
    pub status: Option<u16>,
    pub body: String,
}

hap_fn!(hap_devtools_server_respond_callback, RespondCallbackParams, |p| {
    let state = server::get_state().map_err(hap_common::HapError::internal)?;
    let rt = server::runtime();
    let found = rt.block_on(async {
        let mut pending = state.pending_callbacks.lock().await;
        if let Some(pos) = pending.iter().position(|r| r.request_id == p.request_id) {
            let req = pending.remove(pos);
            let _ = req.respond_tx.send(callback_routes::CallbackResponse {
                status: p.status.unwrap_or(200),
                body: p.body,
            });
            true
        } else {
            false
        }
    });
    Ok(serde_json::json!(found))
});

#[derive(Deserialize)]
pub struct RegisterRouteParams {
    pub method: String,
    pub path: String,
}

hap_fn!(hap_devtools_server_register_internal_route, RegisterRouteParams, |p| {
    let state = server::get_state().map_err(hap_common::HapError::internal)?;
    let rt = server::runtime();
    rt.block_on(async {
        let mut routes = state.internal_routes.write().await;
        routes.push((p.method, p.path));
    });
    Ok(serde_json::json!(true))
});

hap_fn!(hap_devtools_server_poll_internal_requests, PollParams, |p| {
    let state = server::get_state().map_err(hap_common::HapError::internal)?;
    let rt = server::runtime();
    let limit = p.limit.unwrap_or(10);
    let requests = rt.block_on(async {
        let reqs = state.internal_requests.lock().await;
        reqs.iter().take(limit).map(|r| serde_json::json!({
            "request_id": r.request_id,
            "method": r.method,
            "uri": r.uri,
            "headers": r.headers,
            "body": r.body,
        })).collect::<Vec<_>>()
    });
    Ok(serde_json::to_value(requests).unwrap_or(serde_json::json!([])))
});

#[derive(Deserialize)]
pub struct RespondInternalParams {
    pub request_id: String,
    pub status: Option<u16>,
    pub headers: Option<std::collections::HashMap<String, String>>,
    pub body: String,
}

hap_fn!(hap_devtools_server_respond_internal, RespondInternalParams, |p| {
    let state = server::get_state().map_err(hap_common::HapError::internal)?;
    let rt = server::runtime();
    let found = rt.block_on(async {
        let mut reqs = state.internal_requests.lock().await;
        if let Some(pos) = reqs.iter().position(|r| r.request_id == p.request_id) {
            let mut req = reqs.remove(pos);
            if let Some(tx) = req.respond_tx.take() {
                let _ = tx.send(server::InternalResponse {
                    status: p.status.unwrap_or(200),
                    headers: p.headers.unwrap_or_default(),
                    body: p.body,
                });
            }
            true
        } else {
            false
        }
    });
    Ok(serde_json::json!(found))
});
