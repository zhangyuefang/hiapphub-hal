use hap_common::hap_fn;
use serde::Deserialize;
use serde_json::json;

use crate::connect::get_client;

#[derive(Deserialize)]
struct ConnIdParams {
    conn_id: String,
}

#[derive(Deserialize)]
struct ResizeParams {
    conn_id: String,
    width: u32,
    height: u32,
}

#[derive(Deserialize)]
struct MoveParams {
    conn_id: String,
    x: i32,
    y: i32,
}

hap_fn!(hap_automation_get_bounds, ConnIdParams, |params| {
    let (client, app_id, window) = get_client(&params.conn_id)?;
    let path = client.app_window_path(&app_id, &window, "bounds");
    client.get(&path)
});

hap_fn!(hap_automation_resize, ResizeParams, |params| {
    let (client, app_id, window) = get_client(&params.conn_id)?;
    let path = client.app_window_path(&app_id, &window, "resize");
    let body = json!({ "width": params.width, "height": params.height });
    client.post(&path, &body)
});

hap_fn!(hap_automation_move_window, MoveParams, |params| {
    let (client, app_id, window) = get_client(&params.conn_id)?;
    let path = client.app_window_path(&app_id, &window, "move");
    let body = json!({ "x": params.x, "y": params.y });
    client.post(&path, &body)
});

hap_fn!(hap_automation_screenshot, ConnIdParams, |params| {
    let (client, app_id, window) = get_client(&params.conn_id)?;
    let path = client.app_window_path(&app_id, &window, "screenshot");
    client.get(&path)
});
