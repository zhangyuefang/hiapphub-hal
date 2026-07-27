use hap_common::hap_fn;
use serde::Deserialize;
use serde_json::json;

use crate::connect::get_client;

#[derive(Deserialize)]
struct ConnIdParams {
    conn_id: String,
}

#[derive(Deserialize)]
struct ObserveParams {
    conn_id: String,
    selector: Option<String>,
    attributes: Option<bool>,
    #[serde(rename = "childList")]
    child_list: Option<bool>,
    #[serde(rename = "characterData")]
    character_data: Option<bool>,
    subtree: Option<bool>,
}

// --- DOM Mutation ---

hap_fn!(hap_automation_observe_mutations, ObserveParams, |params| {
    let (client, app_id, window) = get_client(&params.conn_id)?;
    let path = client.app_window_path(&app_id, &window, "dom/observe");
    let mut body = serde_json::Map::new();
    if let Some(s) = &params.selector {
        body.insert("selector".into(), json!(s));
    }
    if let Some(v) = params.attributes {
        body.insert("attributes".into(), json!(v));
    }
    if let Some(v) = params.child_list {
        body.insert("childList".into(), json!(v));
    }
    if let Some(v) = params.character_data {
        body.insert("characterData".into(), json!(v));
    }
    if let Some(v) = params.subtree {
        body.insert("subtree".into(), json!(v));
    }
    client.post(&path, &serde_json::Value::Object(body))
});

hap_fn!(hap_automation_get_mutations, ConnIdParams, |params| {
    let (client, app_id, window) = get_client(&params.conn_id)?;
    let path = client.app_window_path(&app_id, &window, "dom/mutations");
    client.get(&path)
});

hap_fn!(hap_automation_stop_observe, ConnIdParams, |params| {
    let (client, app_id, window) = get_client(&params.conn_id)?;
    let path = client.app_window_path(&app_id, &window, "dom/observe/stop");
    client.post(&path, &json!({}))
});

// --- Console ---

hap_fn!(hap_automation_console_start, ConnIdParams, |params| {
    let (client, app_id, window) = get_client(&params.conn_id)?;
    let path = client.app_window_path(&app_id, &window, "console/start");
    client.post(&path, &json!({}))
});

hap_fn!(hap_automation_console_logs, ConnIdParams, |params| {
    let (client, app_id, window) = get_client(&params.conn_id)?;
    let path = client.app_window_path(&app_id, &window, "console/logs");
    client.get(&path)
});

// --- Network ---

hap_fn!(hap_automation_network_start, ConnIdParams, |params| {
    let (client, app_id, window) = get_client(&params.conn_id)?;
    let path = client.app_window_path(&app_id, &window, "network/start");
    client.post(&path, &json!({}))
});

hap_fn!(hap_automation_network_requests, ConnIdParams, |params| {
    let (client, app_id, window) = get_client(&params.conn_id)?;
    let path = client.app_window_path(&app_id, &window, "network/requests");
    client.get(&path)
});

hap_fn!(hap_automation_network_stop, ConnIdParams, |params| {
    let (client, app_id, window) = get_client(&params.conn_id)?;
    let path = client.app_window_path(&app_id, &window, "network/stop");
    client.post(&path, &json!({}))
});

// --- Performance ---

hap_fn!(hap_automation_performance, ConnIdParams, |params| {
    let (client, app_id, window) = get_client(&params.conn_id)?;
    let path = client.app_window_path(&app_id, &window, "performance");
    client.get(&path)
});

// --- Accessibility ---

hap_fn!(hap_automation_accessibility, ConnIdParams, |params| {
    let (client, app_id, window) = get_client(&params.conn_id)?;
    let path = client.app_window_path(&app_id, &window, "accessibility");
    client.get(&path)
});

// --- DOM Tree ---

hap_fn!(hap_automation_dom_tree, ConnIdParams, |params| {
    let (client, app_id, window) = get_client(&params.conn_id)?;
    let path = client.app_window_path(&app_id, &window, "dom");
    client.get(&path)
});
