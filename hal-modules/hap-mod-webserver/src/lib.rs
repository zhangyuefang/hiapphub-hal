mod server;

use hap_common::{hap_fn, ffi::str_to_c};
use serde::Deserialize;
use serde_json::Value;
use std::ffi::c_char;

hap_common::hap_module_init!("webserver");
hap_common::hap_free_string!();

#[no_mangle]
pub extern "C" fn hap_module_describe() -> *const c_char {
    str_to_c(include_str!("../manifest.json"))
}

// --- server group ---

#[derive(Deserialize)]
pub struct ListenParams {
    pub port: Option<u16>,
    pub host: Option<String>,
    pub cors: Option<bool>,
}

hap_fn!(hap_webserver_listen, ListenParams, |p| {
    server::listen(p.port, p.host, p.cors)
});

#[derive(Deserialize)]
pub struct StopParams {
    pub server_id: String,
}

hap_fn!(hap_webserver_stop, StopParams, |p| {
    server::stop(&p.server_id)
});

#[derive(Deserialize)]
pub struct EmptyParams {}

hap_fn!(hap_webserver_list_servers, EmptyParams, |_p| {
    server::list_servers()
});

#[derive(Deserialize)]
pub struct ServerIdParams {
    pub server_id: String,
}

hap_fn!(hap_webserver_server_info, ServerIdParams, |p| {
    server::server_info(&p.server_id)
});

hap_fn!(hap_webserver_pending_count, ServerIdParams, |p| {
    server::pending_count(&p.server_id)
});

hap_fn!(hap_webserver_get_requests, ServerIdParams, |p| {
    server::get_requests(&p.server_id)
});

// --- handler group ---

#[derive(Deserialize)]
pub struct RespondParams {
    pub server_id: String,
    pub request_id: String,
    pub status: Option<u16>,
    pub headers: Option<std::collections::HashMap<String, String>>,
    pub body: Option<String>,
}

hap_fn!(hap_webserver_respond, RespondParams, |p| {
    server::respond(&p.server_id, &p.request_id, p.status, p.headers, p.body)
});

#[derive(Deserialize)]
pub struct RespondJsonParams {
    pub server_id: String,
    pub request_id: String,
    pub status: Option<u16>,
    pub data: Value,
}

hap_fn!(hap_webserver_respond_json, RespondJsonParams, |p| {
    server::respond_json(&p.server_id, &p.request_id, p.status, p.data)
});

#[derive(Deserialize)]
pub struct RespondFileParams {
    pub server_id: String,
    pub request_id: String,
    pub file_path: String,
    pub content_type: Option<String>,
}

hap_fn!(hap_webserver_respond_file, RespondFileParams, |p| {
    server::respond_file(&p.server_id, &p.request_id, &p.file_path, p.content_type)
});

#[derive(Deserialize)]
pub struct RedirectParams {
    pub server_id: String,
    pub request_id: String,
    pub url: String,
    pub status: Option<u16>,
}

hap_fn!(hap_webserver_redirect, RedirectParams, |p| {
    server::redirect(&p.server_id, &p.request_id, &p.url, p.status)
});

// --- config group ---

#[derive(Deserialize)]
pub struct SetTimeoutParams {
    pub server_id: String,
    pub timeout_ms: u64,
}

hap_fn!(hap_webserver_set_timeout, SetTimeoutParams, |p| {
    server::set_timeout(&p.server_id, p.timeout_ms)
});

#[derive(Deserialize)]
pub struct AddStaticParams {
    pub server_id: String,
    pub dir: String,
}

hap_fn!(hap_webserver_add_static, AddStaticParams, |p| {
    server::add_static(&p.server_id, &p.dir)
});

#[cfg(test)]
mod tests {
    use hap_common::ffi::free_c_string;
    use std::ffi::{CStr, CString};

    #[test]
    fn test_list_empty() {
        let arg = CString::new("{}").unwrap();
        let result = unsafe { super::hap_webserver_list_servers(arg.as_ptr()) };
        assert!(!result.is_null());
        let s = unsafe { CStr::from_ptr(result).to_string_lossy().to_string() };
        unsafe { free_c_string(result as *mut _) };
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert!(v.is_array());
    }

    #[test]
    fn test_listen_and_stop() {
        let arg = CString::new(r#"{"port":0,"host":"127.0.0.1"}"#).unwrap();
        let result = unsafe { super::hap_webserver_listen(arg.as_ptr()) };
        let s = unsafe { CStr::from_ptr(result).to_string_lossy().to_string() };
        unsafe { free_c_string(result as *mut _) };
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        let sid = v["server_id"].as_str().unwrap();
        let port = v["port"].as_u64().unwrap();
        assert!(port > 0);

        let stop_arg = CString::new(format!(r#"{{"server_id":"{}"}}"#, sid)).unwrap();
        let result2 = unsafe { super::hap_webserver_stop(stop_arg.as_ptr()) };
        let s2 = unsafe { CStr::from_ptr(result2).to_string_lossy().to_string() };
        unsafe { free_c_string(result2 as *mut _) };
        assert_eq!(s2, "true");
    }

}
