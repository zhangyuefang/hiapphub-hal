use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::json;

use crate::server;

#[derive(Deserialize)]
pub struct EmptyParams {}

hap_fn!(hap_ipc_server_start, EmptyParams, |_p| {
    let path = server::start_server()
        .map_err(HapError::internal)?;
    Ok(json!({ "socket_path": path, "running": true }))
});

hap_fn!(hap_ipc_server_stop, EmptyParams, |_p| {
    Ok(json!(server::stop_server()))
});

hap_fn!(hap_ipc_server_status, EmptyParams, |_p| {
    Ok(json!({
        "running": server::is_running(),
        "socket_path": server::get_socket_path(),
        "connected_apps": server::list_connected_apps(),
    }))
});

#[derive(Deserialize)]
pub struct GenerateTokenParams {
    pub app_id: String,
}

hap_fn!(hap_ipc_server_generate_token, GenerateTokenParams, |p| {
    let token = server::generate_token(&p.app_id);
    Ok(json!({ "token": token, "app_id": p.app_id }))
});

hap_fn!(hap_ipc_server_list_apps, EmptyParams, |_p| {
    Ok(json!(server::list_connected_apps()))
});

#[derive(Deserialize)]
pub struct AppIdParams {
    pub app_id: String,
}

hap_fn!(hap_ipc_server_is_app_connected, AppIdParams, |p| {
    Ok(json!(server::is_app_connected(&p.app_id)))
});

#[derive(Deserialize)]
pub struct PollParams {
    pub limit: Option<usize>,
}

hap_fn!(hap_ipc_server_poll_requests, PollParams, |p| {
    let reqs = server::poll_requests(p.limit.unwrap_or(10));
    Ok(json!(reqs))
});

#[derive(Deserialize)]
pub struct RespondParams {
    pub request_id: String,
    pub result: Option<serde_json::Value>,
    pub error_code: Option<i32>,
    pub error_message: Option<String>,
}

hap_fn!(hap_ipc_server_respond, RespondParams, |p| {
    let error = p.error_code.map(|code| server::JsonRpcError {
        code,
        message: p.error_message.unwrap_or_else(|| "error".to_string()),
    });
    Ok(json!(server::respond_request(&p.request_id, p.result, error)))
});

#[derive(Deserialize)]
pub struct SendParams {
    pub app_id: String,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

hap_fn!(hap_ipc_server_send_to_app, SendParams, |p| {
    server::send_to_app(&p.app_id, &p.method, p.params.unwrap_or(json!({})))
        .map_err(HapError::internal)?;
    Ok(json!(true))
});

hap_fn!(hap_ipc_server_activate_app, AppIdParams, |p| {
    server::send_to_app(&p.app_id, "window.activate", json!({}))
        .map_err(HapError::internal)?;
    Ok(json!(true))
});

hap_fn!(hap_ipc_server_terminate_app, AppIdParams, |p| {
    server::send_to_app(&p.app_id, "lifecycle.terminate", json!({}))
        .map_err(HapError::internal)?;
    Ok(json!(true))
});
