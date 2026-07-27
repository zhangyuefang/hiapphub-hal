use hap_common::hap_fn;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::connect::get_client;

#[derive(Deserialize)]
struct StorageGetParams {
    conn_id: String,
    #[serde(rename = "type")]
    storage_type: Option<String>,
    key: Option<String>,
}

#[derive(Deserialize)]
struct StorageSetParams {
    conn_id: String,
    #[serde(rename = "type")]
    storage_type: String,
    action: String,
    key: Option<String>,
    value: Option<String>,
}

#[derive(Deserialize)]
struct MockSetParams {
    conn_id: String,
    module: String,
    function: String,
    response: Value,
}

#[derive(Deserialize)]
struct ConnIdParams {
    conn_id: String,
}

// --- Storage ---

hap_fn!(hap_automation_storage_get, StorageGetParams, |params| {
    let (client, app_id, window) = get_client(&params.conn_id)?;
    let mut path = client.app_window_path(&app_id, &window, "storage");
    let mut query_parts = Vec::new();
    if let Some(t) = &params.storage_type {
        query_parts.push(format!("type={}", t));
    }
    if let Some(k) = &params.key {
        query_parts.push(format!("key={}", k));
    }
    if !query_parts.is_empty() {
        path = format!("{}?{}", path, query_parts.join("&"));
    }
    client.get(&path)
});

hap_fn!(hap_automation_storage_set, StorageSetParams, |params| {
    let (client, app_id, window) = get_client(&params.conn_id)?;
    let path = client.app_window_path(&app_id, &window, "storage");
    let mut body = json!({
        "type": params.storage_type,
        "action": params.action
    });
    if let Some(k) = &params.key {
        body["key"] = json!(k);
    }
    if let Some(v) = &params.value {
        body["value"] = json!(v);
    }
    client.post(&path, &body)
});

// --- Mock ---

hap_fn!(hap_automation_mock_set, MockSetParams, |params| {
    let (client, app_id, window) = get_client(&params.conn_id)?;
    let path = client.app_window_path(&app_id, &window, "mock/set");
    let body = json!({
        "module": params.module,
        "function": params.function,
        "response": params.response
    });
    client.post(&path, &body)
});

hap_fn!(hap_automation_mock_clear, ConnIdParams, |params| {
    let (client, app_id, window) = get_client(&params.conn_id)?;
    let path = client.app_window_path(&app_id, &window, "mock/clear");
    client.post(&path, &json!({}))
});

// --- DOM Snapshot / Diff ---

#[derive(Deserialize)]
struct SnapshotParams {
    conn_id: String,
    selector: Option<String>,
    depth: Option<u32>,
}

#[derive(Deserialize)]
struct DiffParams {
    before: Value,
    after: Value,
}

hap_fn!(hap_automation_dom_snapshot, SnapshotParams, |params| {
    let (client, app_id, window) = get_client(&params.conn_id)?;
    let path = client.app_window_path(&app_id, &window, "dom/snapshot");
    let mut body = serde_json::Map::new();
    if let Some(s) = &params.selector {
        body.insert("selector".into(), json!(s));
    }
    if let Some(d) = params.depth {
        body.insert("depth".into(), json!(d));
    }
    client.post(&path, &Value::Object(body))
});

hap_fn!(hap_automation_dom_diff, DiffParams, |params| {
    fn diff_nodes(before: &Value, after: &Value) -> Value {
        let mut changes = Vec::new();
        if before != after {
            let mut change = serde_json::Map::new();
            if before.get("tag") != after.get("tag") {
                change.insert("type".into(), json!("replaced"));
                change.insert("before".into(), before.clone());
                change.insert("after".into(), after.clone());
            } else {
                if before.get("attributes") != after.get("attributes") {
                    change.insert("type".into(), json!("attributes_changed"));
                    change.insert("tag".into(), before.get("tag").cloned().unwrap_or(json!("")));
                    change.insert("before".into(), before.get("attributes").cloned().unwrap_or(json!({})));
                    change.insert("after".into(), after.get("attributes").cloned().unwrap_or(json!({})));
                }
                if before.get("text") != after.get("text") {
                    change.insert("type".into(), json!("text_changed"));
                    change.insert("tag".into(), before.get("tag").cloned().unwrap_or(json!("")));
                    change.insert("before".into(), before.get("text").cloned().unwrap_or(json!("")));
                    change.insert("after".into(), after.get("text").cloned().unwrap_or(json!("")));
                }
            }
            if !change.is_empty() {
                changes.push(Value::Object(change));
            }
        }
        json!({ "changes": changes, "count": changes.len() })
    }
    Ok(diff_nodes(&params.before, &params.after))
});
