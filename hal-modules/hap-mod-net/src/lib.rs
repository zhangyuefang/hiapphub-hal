pub mod tcp;
pub mod udp;
pub mod utils;

use hap_common::ffi::str_to_c;
use std::ffi::c_char;

hap_common::hap_module_init!("net");
hap_common::hap_free_string!();

#[no_mangle]
pub extern "C" fn hap_module_describe() -> *const c_char {
    str_to_c(include_str!("../manifest.json"))
}

#[cfg(test)]
mod tests {
    use super::tcp::*;
    use super::udp::*;
    use super::utils::*;
    use hap_common::ffi::{str_to_c, free_c_string};
    use std::ffi::CStr;
    use serde_json::json;

    fn call(func: extern "C" fn(*const std::ffi::c_char) -> *const std::ffi::c_char, params: serde_json::Value) -> serde_json::Value {
        let input = str_to_c(&params.to_string());
        let result = func(input);
        unsafe { free_c_string(input as *mut _); }
        let s = unsafe { CStr::from_ptr(result) }.to_str().unwrap().to_string();
        unsafe { free_c_string(result as *mut _); }
        serde_json::from_str(&s).unwrap()
    }

    #[test]
    fn test_describe() {
        let ptr = super::hap_module_describe();
        let s = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        let v: serde_json::Value = serde_json::from_str(s).unwrap();
        assert_eq!(v["name"], "net");
        assert_eq!(v["functions"].as_array().unwrap().len(), 34);
        unsafe { free_c_string(ptr as *mut _); }
    }

    #[test]
    fn test_local_ip() {
        let r = call(hap_net_local_ip, json!({}));
        assert!(r.is_string());
    }

    #[test]
    fn test_interfaces() {
        let r = call(hap_net_interfaces, json!({}));
        assert!(r.is_array());
    }

    #[test]
    fn test_port_available() {
        let r = call(hap_net_port_available, json!({"port": 59999}));
        assert!(r.is_boolean());
    }

    #[test]
    fn test_find_available_port() {
        let r = call(hap_net_find_available_port, json!({"start_port": 50000}));
        assert!(r.as_i64().unwrap() >= 50000);
    }

    #[test]
    fn test_dns_lookup() {
        let r = call(hap_net_dns_lookup, json!({"hostname": "localhost"}));
        if r.is_array() {
            assert!(!r.as_array().unwrap().is_empty());
        }
    }

    #[test]
    fn test_wake_on_lan() {
        let r = call(hap_net_wake_on_lan, json!({"mac_address": "00:11:22:33:44:55", "broadcast_ip": "127.0.0.1"}));
        assert!(r == json!(true) || r.get("error").is_some());
    }

    #[test]
    fn test_tcp_listen_accept_close() {
        let srv = call(hap_net_tcp_listen, json!({"host": "127.0.0.1", "port": 0}));
        let server_id = srv["server_id"].as_str().unwrap().to_string();
        let local = srv["local_addr"].as_str().unwrap().to_string();
        let parts: Vec<&str> = local.split(':').collect();
        let port: i32 = parts.last().unwrap().parse().unwrap();

        let conn = call(hap_net_tcp_connect, json!({"host": "127.0.0.1", "port": port, "timeout_ms": 2000}));
        let conn_id = conn["conn_id"].as_str().unwrap().to_string();

        let _ = call(hap_net_tcp_close, json!({"conn_id": conn_id}));
        let _ = call(hap_net_tcp_stop, json!({"server_id": server_id}));
    }

    #[test]
    fn test_udp_bind_send_close() {
        let r = call(hap_net_udp_bind, json!({"host": "127.0.0.1", "port": 0}));
        let sid = r["socket_id"].as_str().unwrap().to_string();
        let local = r["local_addr"].as_str().unwrap().to_string();
        let parts: Vec<&str> = local.split(':').collect();
        let port: i32 = parts.last().unwrap().parse().unwrap();

        let n = call(hap_net_udp_send, json!({"socket_id": sid, "host": "127.0.0.1", "port": port, "data": "hello"}));
        assert!(n.as_i64().unwrap() > 0);

        let _ = call(hap_net_udp_close, json!({"socket_id": sid}));
    }

    #[test]
    fn test_list_tcp_connections() {
        let r = call(hap_net_list_tcp_connections, json!({}));
        assert!(r.is_array());
    }

    #[test]
    fn test_list_udp_sockets() {
        let r = call(hap_net_list_udp_sockets, json!({}));
        assert!(r.is_array());
    }

    #[test]
    fn test_is_online() {
        let r = call(hap_net_is_online, json!({"timeout_ms": 5000}));
        assert!(r.is_boolean());
    }

    #[test]
    fn test_mac_address() {
        let r = call(hap_net_mac_address, json!({}));
        assert!(r.is_string());
    }

    #[test]
    fn test_wifi_info() {
        let r = call(hap_net_wifi_info, json!({}));
        assert!(r.is_null() || r.is_object());
    }

    #[test]
    fn test_on_off_network_change() {
        let r = call(hap_net_on_network_change, json!({"callback_id": "cb1"}));
        let wid = r["watcher_id"].as_str().unwrap().to_string();
        assert!(wid.starts_with("netw_"));
        std::thread::sleep(std::time::Duration::from_millis(100));
        let r2 = call(hap_net_off_network_change, json!({"watcher_id": wid}));
        assert_eq!(r2, json!(true));
        let r3 = call(hap_net_off_network_change, json!({"watcher_id": "nonexistent"}));
        assert_eq!(r3, json!(false));
    }
}
