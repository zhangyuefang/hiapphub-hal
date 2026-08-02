use hap_common::hap_fn;
use serde_json::{json, Value};

hap_fn!(hap_ohos_network_get_type, Value, |_params| {
    Ok(json!({ "action": "connection.getDefaultNet.getNetCapabilities", "delegate": "arkts" }))
});

hap_fn!(hap_ohos_network_is_connected, Value, |_params| {
    Ok(json!({ "action": "connection.hasDefaultNet", "delegate": "arkts" }))
});

hap_fn!(hap_ohos_network_get_wifi_info, Value, |_params| {
    Ok(json!({ "action": "wifiManager.getLinkedInfo", "delegate": "arkts" }))
});

hap_fn!(hap_ohos_network_get_cellular_info, Value, |_params| {
    Ok(json!({ "action": "radio.getSignalInformation", "delegate": "arkts" }))
});

hap_fn!(hap_ohos_network_on_change, Value, |_params| {
    Ok(json!({ "action": "connection.on.netAvailable", "delegate": "arkts" }))
});

hap_fn!(hap_ohos_network_get_ip_address, Value, |_params| {
    Ok(json!({ "action": "connection.getDefaultNet.getAddressesByName", "delegate": "arkts" }))
});
