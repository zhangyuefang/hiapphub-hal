use hap_common::{hap_fn, HapError};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::client::{ApiClient, discover_api};
use crate::types::{ConnectParams, DisconnectParams, EmptyParams};

struct Connection {
    port: u16,
    token: String,
    app_id: String,
    window: String,
}

static CONNS: OnceLock<Mutex<HashMap<String, Connection>>> = OnceLock::new();

fn conns() -> &'static Mutex<HashMap<String, Connection>> {
    CONNS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_conns() -> std::sync::MutexGuard<'static, HashMap<String, Connection>> {
    conns().lock().unwrap_or_else(|e| e.into_inner())
}

fn gen_conn_id() -> String {
    format!("conn_{}", uuid::Uuid::new_v4().as_simple())
}

pub fn get_client(conn_id: &str) -> Result<(ApiClient, String, String), HapError> {
    let map = lock_conns();
    let conn = map.get(conn_id)
        .ok_or_else(|| HapError::internal("not_connected: invalid conn_id"))?;
    Ok((
        ApiClient::new(conn.port, conn.token.clone()),
        conn.app_id.clone(),
        conn.window.clone(),
    ))
}

hap_fn!(hap_automation_connect, ConnectParams, |params| {
    let (port, token) = discover_api()?;
    let client = ApiClient::new(port, token.clone());

    let apps = client.get("/api/v1/apps")?;
    let apps_arr = apps.get("apps").and_then(|v| v.as_array())
        .or_else(|| apps.as_array())
        .ok_or_else(|| HapError::internal("invalid apps response"))?;

    let found = apps_arr.iter().any(|app| {
        app.get("appId").and_then(|v| v.as_str()) == Some(&params.app_id)
    });
    if !found {
        return Err(HapError::internal(
            format!("app_not_found: '{}' not in running apps", params.app_id)
        ));
    }

    let window = params.window.unwrap_or_else(|| "main".to_string());
    let conn_id = gen_conn_id();

    conns().lock().unwrap_or_else(|e| e.into_inner()).insert(conn_id.clone(), Connection {
        port,
        token,
        app_id: params.app_id.clone(),
        window: window.clone(),
    });

    Ok(json!({ "conn_id": conn_id, "appId": params.app_id, "window": window }))
});

hap_fn!(hap_automation_disconnect, DisconnectParams, |params| {
    let removed = lock_conns().remove(&params.conn_id).is_some();
    if !removed {
        return Err(HapError::internal("not_connected: conn_id not found"));
    }
    Ok(json!({ "success": true }))
});

hap_fn!(hap_automation_list_apps, EmptyParams, |_params| {
    let (port, token) = discover_api()?;
    let client = ApiClient::new(port, token);
    client.get("/api/v1/apps")
});
