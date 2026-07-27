use hap_common::hap_fn;
use serde::Deserialize;
use serde_json::json;

use crate::connect::get_client;

#[derive(Deserialize)]
struct QueryParams {
    conn_id: String,
    selector: String,
    #[serde(rename = "type")]
    query_type: Option<String>,
    #[serde(rename = "includeStyles")]
    include_styles: Option<bool>,
}

#[derive(Deserialize)]
struct QueryAllParams {
    conn_id: String,
    selector: String,
    #[serde(rename = "type")]
    query_type: Option<String>,
    limit: Option<u32>,
}

#[derive(Deserialize)]
struct WaitForParams {
    conn_id: String,
    selector: String,
    timeout: Option<u64>,
}

#[derive(Deserialize)]
struct ClickParams {
    conn_id: String,
    selector: Option<String>,
    x: Option<f64>,
    y: Option<f64>,
}

#[derive(Deserialize)]
struct TypeTextParams {
    conn_id: String,
    selector: String,
    text: String,
    clear: Option<bool>,
}

#[derive(Deserialize)]
struct ScrollParams {
    conn_id: String,
    selector: Option<String>,
    #[serde(rename = "deltaX")]
    delta_x: Option<f64>,
    #[serde(rename = "deltaY")]
    delta_y: Option<f64>,
}

#[derive(Deserialize)]
struct EvalParams {
    conn_id: String,
    code: String,
}

hap_fn!(hap_automation_query, QueryParams, |params| {
    let (client, app_id, window) = get_client(&params.conn_id)?;
    let mut path = client.app_window_path(&app_id, &window, "dom/query");
    let mut query_parts = vec![format!("selector={}", urlencoded(&params.selector))];
    if let Some(t) = &params.query_type {
        query_parts.push(format!("type={}", t));
    }
    if params.include_styles.unwrap_or(false) {
        query_parts.push("includeStyles=true".to_string());
    }
    path = format!("{}?{}", path, query_parts.join("&"));
    client.get(&path)
});

hap_fn!(hap_automation_query_all, QueryAllParams, |params| {
    let (client, app_id, window) = get_client(&params.conn_id)?;
    let mut path = client.app_window_path(&app_id, &window, "dom/query-all");
    let mut query_parts = vec![format!("selector={}", urlencoded(&params.selector))];
    if let Some(t) = &params.query_type {
        query_parts.push(format!("type={}", t));
    }
    if let Some(limit) = params.limit {
        query_parts.push(format!("limit={}", limit));
    }
    path = format!("{}?{}", path, query_parts.join("&"));
    client.get(&path)
});

hap_fn!(hap_automation_wait_for, WaitForParams, |params| {
    let (client, app_id, window) = get_client(&params.conn_id)?;
    let path = client.app_window_path(&app_id, &window, "wait-for-selector");
    let body = json!({
        "selector": params.selector,
        "timeout": params.timeout.unwrap_or(5000)
    });
    client.post(&path, &body)
});

hap_fn!(hap_automation_click, ClickParams, |params| {
    let (client, app_id, window) = get_client(&params.conn_id)?;
    let path = client.app_window_path(&app_id, &window, "click");
    let mut body = serde_json::Map::new();
    if let Some(s) = &params.selector {
        body.insert("selector".into(), json!(s));
    }
    if let Some(x) = params.x {
        body.insert("x".into(), json!(x));
    }
    if let Some(y) = params.y {
        body.insert("y".into(), json!(y));
    }
    client.post(&path, &serde_json::Value::Object(body))
});

hap_fn!(hap_automation_type_text, TypeTextParams, |params| {
    let (client, app_id, window) = get_client(&params.conn_id)?;
    let path = client.app_window_path(&app_id, &window, "type");
    let mut body = json!({
        "selector": params.selector,
        "text": params.text
    });
    if let Some(clear) = params.clear {
        body["clear"] = json!(clear);
    }
    client.post(&path, &body)
});

hap_fn!(hap_automation_scroll, ScrollParams, |params| {
    let (client, app_id, window) = get_client(&params.conn_id)?;
    let path = client.app_window_path(&app_id, &window, "scroll");
    let mut body = serde_json::Map::new();
    if let Some(s) = &params.selector {
        body.insert("selector".into(), json!(s));
    }
    if let Some(dx) = params.delta_x {
        body.insert("deltaX".into(), json!(dx));
    }
    if let Some(dy) = params.delta_y {
        body.insert("deltaY".into(), json!(dy));
    }
    client.post(&path, &serde_json::Value::Object(body))
});

hap_fn!(hap_automation_eval, EvalParams, |params| {
    let (client, app_id, window) = get_client(&params.conn_id)?;
    let path = client.app_window_path(&app_id, &window, "eval");
    let body = json!({ "code": params.code });
    client.post(&path, &body)
});

fn urlencoded(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}
