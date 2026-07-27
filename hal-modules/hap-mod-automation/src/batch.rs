use hap_common::hap_fn;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::connect::get_client;

#[derive(Deserialize)]
struct BatchParams {
    conn_id: String,
    steps: Vec<Value>,
    #[serde(rename = "stopOnError")]
    stop_on_error: Option<bool>,
}

hap_fn!(hap_automation_batch, BatchParams, |params| {
    let (client, app_id, window) = get_client(&params.conn_id)?;
    let path = client.app_window_path(&app_id, &window, "batch");
    let mut body = json!({ "steps": params.steps });
    if let Some(stop) = params.stop_on_error {
        body["stopOnError"] = json!(stop);
    }
    client.post(&path, &body)
});
