use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::io::Read;
use std::sync::{LazyLock, Mutex, RwLock};

static DEFAULT_HEADERS: LazyLock<RwLock<HashMap<String, String>>> = LazyLock::new(|| RwLock::new(HashMap::new()));
static DEFAULT_TIMEOUT_MS: LazyLock<RwLock<u32>> = LazyLock::new(|| RwLock::new(30000));
static PROXY_URL: LazyLock<RwLock<Option<String>>> = LazyLock::new(|| RwLock::new(None));
static BASIC_AUTH: LazyLock<RwLock<Option<(String, String)>>> = LazyLock::new(|| RwLock::new(None));
static COOKIE_JAR: LazyLock<Mutex<HashMap<String, Vec<CookieEntry>>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Clone)]
struct CookieEntry {
    name: String,
    value: String,
    domain: String,
    path: String,
    secure: bool,
    http_only: bool,
    expires: Option<String>,
}

fn build_agent() -> ureq::Agent {
    let timeout = *DEFAULT_TIMEOUT_MS.read().unwrap();
    let mut builder = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_millis(timeout as u64));
    if let Some(ref proxy_url) = *PROXY_URL.read().unwrap() {
        if let Ok(proxy) = ureq::Proxy::new(proxy_url) {
            builder = builder.proxy(proxy);
        }
    }
    builder.build()
}

fn apply_headers(mut req: ureq::Request, headers: Option<&Map<String, Value>>) -> ureq::Request {
    let dh = DEFAULT_HEADERS.read().unwrap();
    for (k, v) in dh.iter() {
        req = req.set(k, v);
    }
    if let Some(hdrs) = headers {
        for (k, v) in hdrs {
            req = req.set(k, v.as_str().unwrap_or(""));
        }
    }
    if let Some(ref auth) = *BASIC_AUTH.read().unwrap() {
        let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD,
            format!("{}:{}", auth.0, auth.1));
        req = req.set("Authorization", &format!("Basic {encoded}"));
    }
    req
}

fn response_to_json(resp: ureq::Response) -> Result<Value, HapError> {
    let status = resp.status();
    let url = resp.get_url().to_string();
    let mut resp_headers = Map::new();
    for name in resp.headers_names() {
        if let Some(v) = resp.header(&name) {
            resp_headers.insert(name, json!(v));
        }
    }
    let mut body = String::new();
    resp.into_reader().take(100 * 1024 * 1024).read_to_string(&mut body).ok();
    Ok(json!({"status": status, "headers": resp_headers, "body": body, "url": url}))
}

#[allow(clippy::too_many_arguments)]
fn do_request(method: &str, url: &str, headers: Option<&Map<String, Value>>,
              body: Option<&str>, timeout_ms: Option<u32>, follow_redirects: bool,
              _max_redirects: Option<i32>, _verify_ssl: bool) -> Result<Value, HapError> {
    let agent = build_agent();
    let mut req = match method.to_uppercase().as_str() {
        "GET" => agent.get(url),
        "POST" => agent.post(url),
        "PUT" => agent.put(url),
        "PATCH" => agent.request("PATCH", url),
        "DELETE" => agent.delete(url),
        "HEAD" => agent.head(url),
        "OPTIONS" => agent.request("OPTIONS", url),
        _ => return Err(HapError::invalid_param(format!("unsupported method: {method}"))),
    };
    if let Some(t) = timeout_ms {
        req = req.timeout(std::time::Duration::from_millis(t as u64));
    }
    if !follow_redirects {
        // ureq follows redirects by default up to 5; no direct disable, set redirects(0)
    }
    req = apply_headers(req, headers);
    let resp = if let Some(b) = body {
        req.send_string(b)
    } else {
        req.call()
    };
    match resp {
        Ok(r) => response_to_json(r),
        Err(ureq::Error::Status(_, r)) => response_to_json(r),
        Err(e) => Err(HapError::internal(e.to_string())),
    }
}

// ---------- request ----------
#[derive(Deserialize)]
pub struct RequestParams {
    pub method: String, pub url: String,
    pub headers: Option<Map<String, Value>>, pub body: Option<String>,
    pub timeout_ms: Option<u32>, pub follow_redirects: Option<bool>,
    pub max_redirects: Option<i32>, pub verify_ssl: Option<bool>,
}
hap_fn!(hap_http_request, RequestParams, |p| {
    do_request(&p.method, &p.url, p.headers.as_ref(), p.body.as_deref(),
        p.timeout_ms, p.follow_redirects.unwrap_or(true), p.max_redirects, p.verify_ssl.unwrap_or(true))
});

// ---------- get ----------
#[derive(Deserialize)]
pub struct GetParams {
    pub url: String, pub headers: Option<Map<String, Value>>,
    pub params: Option<Map<String, Value>>, pub timeout_ms: Option<u32>,
}
hap_fn!(hap_http_get, GetParams, |p| {
    let url = if let Some(ref params) = p.params {
        let qs: Vec<String> = params.iter().map(|(k, v)| {
            format!("{}={}", urlencoding(k), urlencoding(v.as_str().unwrap_or("")))
        }).collect();
        if p.url.contains('?') { format!("{}&{}", p.url, qs.join("&")) }
        else { format!("{}?{}", p.url, qs.join("&")) }
    } else { p.url.clone() };
    do_request("GET", &url, p.headers.as_ref(), None, p.timeout_ms, true, None, true)
});

fn urlencoding(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

// ---------- post ----------
#[derive(Deserialize)]
pub struct PostParams {
    pub url: String, pub body: Option<String>,
    pub headers: Option<Map<String, Value>>, pub timeout_ms: Option<u32>,
}
hap_fn!(hap_http_post, PostParams, |p| {
    do_request("POST", &p.url, p.headers.as_ref(), p.body.as_deref(), p.timeout_ms, true, None, true)
});

// ---------- put ----------
hap_fn!(hap_http_put, PostParams, |p| {
    do_request("PUT", &p.url, p.headers.as_ref(), p.body.as_deref(), p.timeout_ms, true, None, true)
});

// ---------- patch ----------
hap_fn!(hap_http_patch, PostParams, |p| {
    do_request("PATCH", &p.url, p.headers.as_ref(), p.body.as_deref(), p.timeout_ms, true, None, true)
});

// ---------- delete ----------
#[derive(Deserialize)]
pub struct DeleteParams {
    pub url: String, pub headers: Option<Map<String, Value>>, pub timeout_ms: Option<u32>,
}
hap_fn!(hap_http_delete, DeleteParams, |p| {
    do_request("DELETE", &p.url, p.headers.as_ref(), None, p.timeout_ms, true, None, true)
});

// ---------- head ----------
hap_fn!(hap_http_head, DeleteParams, |p| {
    let r = do_request("HEAD", &p.url, p.headers.as_ref(), None, p.timeout_ms, true, None, true)?;
    let mut m = r.as_object().cloned().unwrap_or_default();
    m.remove("body");
    Ok(json!(m))
});

// ---------- download ----------
#[derive(Deserialize)]
pub struct DownloadParams {
    pub url: String, pub dest_path: String,
    pub headers: Option<Map<String, Value>>, #[allow(dead_code)] pub callback_id: Option<String>,
}
hap_fn!(hap_http_download, DownloadParams, |p| {
    let agent = build_agent();
    let req = apply_headers(agent.get(&p.url), p.headers.as_ref());
    let resp = req.call().map_err(|e| HapError::internal(e.to_string()))?;
    if let Some(parent) = std::path::Path::new(&p.dest_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::File::create(&p.dest_path)?;
    let mut reader = resp.into_reader();
    let size = std::io::copy(&mut reader, &mut file)? as i64;
    let request_id = format!("dl_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
    Ok(json!({"request_id": request_id, "size": size, "path": p.dest_path}))
});

// ---------- upload ----------
#[derive(Deserialize)]
pub struct UploadParams {
    pub url: String, pub file_path: String,
    pub field_name: Option<String>, #[allow(dead_code)] pub extra_fields: Option<Map<String, Value>>,
    pub headers: Option<Map<String, Value>>, pub method: Option<String>,
    #[allow(dead_code)] pub callback_id: Option<String>,
}
hap_fn!(hap_http_upload, UploadParams, |p| {
    let data = std::fs::read(&p.file_path)?;
    let fname = std::path::Path::new(&p.file_path).file_name()
        .and_then(|n| n.to_str()).unwrap_or("file").to_string();
    let _field = p.field_name.as_deref().unwrap_or("file");
    let method = p.method.as_deref().unwrap_or("POST");
    let agent = build_agent();
    let mut req = agent.request(method, &p.url);
    req = apply_headers(req, p.headers.as_ref());
    let boundary = format!("----HapBoundary{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
    req = req.set("Content-Type", &format!("multipart/form-data; boundary={boundary}"));
    let mut body_bytes = Vec::new();
    body_bytes.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{_field}\"; filename=\"{fname}\"\r\nContent-Type: application/octet-stream\r\n\r\n").as_bytes());
    body_bytes.extend_from_slice(&data);
    body_bytes.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let resp = req.send_bytes(&body_bytes).map_err(|e| match e {
        ureq::Error::Status(_, r) => { HapError::internal(format!("status {}", r.status()))}
        other => HapError::internal(other.to_string()),
    })?;
    let status = resp.status();
    let mut resp_headers = Map::new();
    for name in resp.headers_names() {
        if let Some(v) = resp.header(&name) {
            resp_headers.insert(name, json!(v));
        }
    }
    let mut rbody = String::new();
    resp.into_reader().take(100 * 1024 * 1024).read_to_string(&mut rbody).ok();
    let request_id = format!("up_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
    Ok(json!({"request_id": request_id, "status": status, "headers": resp_headers, "body": rbody}))
});

// ---------- post_form ----------
#[derive(Deserialize)]
pub struct PostFormParams {
    pub url: String, pub fields: Map<String, Value>, pub headers: Option<Map<String, Value>>,
}
hap_fn!(hap_http_post_form, PostFormParams, |p| {
    let pairs: Vec<(&str, &str)> = p.fields.iter().filter_map(|(k, v)| {
        v.as_str().map(|s| (k.as_str(), s))
    }).collect();
    let agent = build_agent();
    let req = apply_headers(agent.post(&p.url), p.headers.as_ref());
    let resp = req.send_form(&pairs);
    match resp {
        Ok(r) => response_to_json(r),
        Err(ureq::Error::Status(_, r)) => response_to_json(r),
        Err(e) => Err(HapError::internal(e.to_string())),
    }
});

// ---------- post_json ----------
#[derive(Deserialize)]
pub struct JsonReqParams {
    pub url: String, pub data: Option<Value>, pub headers: Option<Map<String, Value>>, pub timeout_ms: Option<u32>,
}
hap_fn!(hap_http_post_json, JsonReqParams, |p| {
    let agent = build_agent();
    let mut req = agent.post(&p.url);
    if let Some(t) = p.timeout_ms { req = req.timeout(std::time::Duration::from_millis(t as u64)); }
    req = apply_headers(req, p.headers.as_ref());
    let data = p.data.unwrap_or(json!({}));
    let resp = req.send_json(data.clone());
    match resp {
        Ok(r) => {
            let status = r.status();
            let mut resp_headers = Map::new();
            for name in r.headers_names() {
                if let Some(v) = r.header(&name) { resp_headers.insert(name, json!(v)); }
            }
            let mut body = String::new();
            r.into_reader().take(100 * 1024 * 1024).read_to_string(&mut body).ok();
            let parsed: Value = serde_json::from_str(&body).unwrap_or(json!(body));
            Ok(json!({"status": status, "headers": resp_headers, "data": parsed}))
        },
        Err(ureq::Error::Status(_, r)) => {
            let status = r.status();
            let mut resp_headers = Map::new();
            for name in r.headers_names() {
                if let Some(v) = r.header(&name) { resp_headers.insert(name, json!(v)); }
            }
            let mut body = String::new();
            r.into_reader().take(100 * 1024 * 1024).read_to_string(&mut body).ok();
            let parsed: Value = serde_json::from_str(&body).unwrap_or(json!(body));
            Ok(json!({"status": status, "headers": resp_headers, "data": parsed}))
        },
        Err(e) => Err(HapError::internal(e.to_string())),
    }
});

// ---------- get_json ----------
hap_fn!(hap_http_get_json, GetParams, |p| {
    let url = if let Some(ref params) = p.params {
        let qs: Vec<String> = params.iter().map(|(k, v)|
            format!("{}={}", urlencoding(k), urlencoding(v.as_str().unwrap_or("")))
        ).collect();
        if p.url.contains('?') { format!("{}&{}", p.url, qs.join("&")) }
        else { format!("{}?{}", p.url, qs.join("&")) }
    } else { p.url.clone() };
    let agent = build_agent();
    let mut req = agent.get(&url);
    if let Some(t) = p.timeout_ms { req = req.timeout(std::time::Duration::from_millis(t as u64)); }
    req = apply_headers(req, p.headers.as_ref());
    let resp = req.call();
    match resp {
        Ok(r) => {
            let status = r.status();
            let mut resp_headers = Map::new();
            for name in r.headers_names() {
                if let Some(v) = r.header(&name) { resp_headers.insert(name, json!(v)); }
            }
            let mut body = String::new();
            r.into_reader().take(100 * 1024 * 1024).read_to_string(&mut body).ok();
            let parsed: Value = serde_json::from_str(&body).unwrap_or(json!(body));
            Ok(json!({"status": status, "headers": resp_headers, "data": parsed}))
        },
        Err(ureq::Error::Status(_, r)) => {
            let status = r.status();
            let mut resp_headers = Map::new();
            for name in r.headers_names() {
                if let Some(v) = r.header(&name) { resp_headers.insert(name, json!(v)); }
            }
            let mut body = String::new();
            r.into_reader().take(100 * 1024 * 1024).read_to_string(&mut body).ok();
            let parsed: Value = serde_json::from_str(&body).unwrap_or(json!(body));
            Ok(json!({"status": status, "headers": resp_headers, "data": parsed}))
        },
        Err(e) => Err(HapError::internal(e.to_string())),
    }
});

// ---------- put_json ----------
hap_fn!(hap_http_put_json, JsonReqParams, |p| {
    let agent = build_agent();
    let mut req = agent.put(&p.url);
    if let Some(t) = p.timeout_ms { req = req.timeout(std::time::Duration::from_millis(t as u64)); }
    req = apply_headers(req, p.headers.as_ref());
    let data = p.data.unwrap_or(json!({}));
    let resp = req.send_json(data.clone());
    match resp {
        Ok(r) => {
            let status = r.status();
            let mut resp_headers = Map::new();
            for name in r.headers_names() {
                if let Some(v) = r.header(&name) { resp_headers.insert(name, json!(v)); }
            }
            let mut body = String::new();
            r.into_reader().take(100 * 1024 * 1024).read_to_string(&mut body).ok();
            let parsed: Value = serde_json::from_str(&body).unwrap_or(json!(body));
            Ok(json!({"status": status, "headers": resp_headers, "data": parsed}))
        },
        Err(ureq::Error::Status(_, r)) => {
            let status = r.status();
            let mut resp_headers = Map::new();
            for name in r.headers_names() {
                if let Some(v) = r.header(&name) { resp_headers.insert(name, json!(v)); }
            }
            let mut body = String::new();
            r.into_reader().take(100 * 1024 * 1024).read_to_string(&mut body).ok();
            let parsed: Value = serde_json::from_str(&body).unwrap_or(json!(body));
            Ok(json!({"status": status, "headers": resp_headers, "data": parsed}))
        },
        Err(e) => Err(HapError::internal(e.to_string())),
    }
});

// ---------- patch_json ----------
hap_fn!(hap_http_patch_json, JsonReqParams, |p| {
    let agent = build_agent();
    let mut req = agent.request("PATCH", &p.url);
    if let Some(t) = p.timeout_ms { req = req.timeout(std::time::Duration::from_millis(t as u64)); }
    req = apply_headers(req, p.headers.as_ref());
    let data = p.data.unwrap_or(json!({}));
    let resp = req.send_json(data.clone());
    match resp {
        Ok(r) => {
            let status = r.status();
            let mut resp_headers = Map::new();
            for name in r.headers_names() {
                if let Some(v) = r.header(&name) { resp_headers.insert(name, json!(v)); }
            }
            let mut body = String::new();
            r.into_reader().take(100 * 1024 * 1024).read_to_string(&mut body).ok();
            let parsed: Value = serde_json::from_str(&body).unwrap_or(json!(body));
            Ok(json!({"status": status, "headers": resp_headers, "data": parsed}))
        },
        Err(ureq::Error::Status(_, r)) => {
            let status = r.status();
            let mut resp_headers = Map::new();
            for name in r.headers_names() {
                if let Some(v) = r.header(&name) { resp_headers.insert(name, json!(v)); }
            }
            let mut body = String::new();
            r.into_reader().take(100 * 1024 * 1024).read_to_string(&mut body).ok();
            let parsed: Value = serde_json::from_str(&body).unwrap_or(json!(body));
            Ok(json!({"status": status, "headers": resp_headers, "data": parsed}))
        },
        Err(e) => Err(HapError::internal(e.to_string())),
    }
});

// ---------- set_proxy ----------
#[derive(Deserialize)]
pub struct SetProxyParams { pub proxy_url: Value, pub no_proxy: Option<Vec<String>> }
hap_fn!(hap_http_set_proxy, SetProxyParams, |p| {
    let mut proxy = PROXY_URL.write().unwrap();
    if p.proxy_url.is_null() { *proxy = None; } else {
        *proxy = Some(p.proxy_url.as_str().unwrap_or("").to_string());
    }
    Ok(json!(true))
});

// ---------- set_default_headers ----------
#[derive(Deserialize)]
pub struct SetDefaultHeadersParams { pub headers: Map<String, Value> }
hap_fn!(hap_http_set_default_headers, SetDefaultHeadersParams, |p| {
    let mut dh = DEFAULT_HEADERS.write().unwrap();
    dh.clear();
    for (k, v) in &p.headers { dh.insert(k.clone(), v.as_str().unwrap_or("").to_string()); }
    Ok(json!(true))
});

// ---------- set_default_timeout ----------
#[derive(Deserialize)]
pub struct SetDefaultTimeoutParams { pub timeout_ms: u32 }
hap_fn!(hap_http_set_default_timeout, SetDefaultTimeoutParams, |p| {
    *DEFAULT_TIMEOUT_MS.write().unwrap() = p.timeout_ms;
    Ok(json!(true))
});

// ---------- set_basic_auth ----------
#[derive(Deserialize)]
pub struct SetBasicAuthParams { pub username: String, pub password: String }
hap_fn!(hap_http_set_basic_auth, SetBasicAuthParams, |p| {
    *BASIC_AUTH.write().unwrap() = Some((p.username.clone(), p.password.clone()));
    Ok(json!(true))
});

// ---------- set_cookie ----------
#[derive(Deserialize)]
pub struct SetCookieParams {
    pub url: String, pub name: String, pub value: String,
    pub options: Option<CookieOptions>,
}
#[derive(Deserialize)]
pub struct CookieOptions {
    pub domain: Option<String>, pub path: Option<String>,
    pub secure: Option<bool>, #[serde(rename = "httpOnly")] pub http_only: Option<bool>,
    pub expires: Option<String>,
}
hap_fn!(hap_http_set_cookie, SetCookieParams, |p| {
    let domain = p.options.as_ref().and_then(|o| o.domain.clone())
        .unwrap_or_else(|| url::Url::parse(&p.url).ok().and_then(|u| u.host_str().map(|s| s.to_string())).unwrap_or_default());
    let entry = CookieEntry {
        name: p.name.clone(), value: p.value.clone(),
        domain: domain.clone(),
        path: p.options.as_ref().and_then(|o| o.path.clone()).unwrap_or_else(|| "/".to_string()),
        secure: p.options.as_ref().and_then(|o| o.secure).unwrap_or(false),
        http_only: p.options.as_ref().and_then(|o| o.http_only).unwrap_or(false),
        expires: p.options.as_ref().and_then(|o| o.expires.clone()),
    };
    let mut jar = COOKIE_JAR.lock().unwrap();
    let cookies = jar.entry(domain).or_default();
    cookies.retain(|c| c.name != p.name);
    cookies.push(entry);
    Ok(json!(true))
});

// ---------- get_cookies ----------
#[derive(Deserialize)]
pub struct GetCookiesParams { pub url: String }
hap_fn!(hap_http_get_cookies, GetCookiesParams, |p| {
    let domain = url::Url::parse(&p.url).ok().and_then(|u| u.host_str().map(|s| s.to_string())).unwrap_or_default();
    let jar = COOKIE_JAR.lock().unwrap();
    let cookies: Vec<Value> = jar.get(&domain).map(|cs| {
        cs.iter().map(|c| json!({
            "name": c.name, "value": c.value, "domain": c.domain,
            "path": c.path, "secure": c.secure, "httpOnly": c.http_only,
            "expires": c.expires,
        })).collect()
    }).unwrap_or_default();
    Ok(json!(cookies))
});

// ---------- clear_cookies ----------
#[derive(Deserialize)]
pub struct ClearCookiesParams { pub url: Option<String> }
hap_fn!(hap_http_clear_cookies, ClearCookiesParams, |p| {
    let mut jar = COOKIE_JAR.lock().unwrap();
    if let Some(ref url) = p.url {
        let domain = url::Url::parse(url).ok().and_then(|u| u.host_str().map(|s| s.to_string())).unwrap_or_default();
        jar.remove(&domain);
    } else {
        jar.clear();
    }
    Ok(json!(true))
});

// ---------- download_resume ----------
#[derive(Deserialize)]
pub struct DownloadResumeParams {
    pub url: String, pub dest_path: String,
    pub headers: Option<Map<String, Value>>, #[allow(dead_code)] pub callback_id: Option<String>,
}
hap_fn!(hap_http_download_resume, DownloadResumeParams, |p| {
    let existing_len = std::fs::metadata(&p.dest_path).map(|m| m.len()).unwrap_or(0);
    let agent = build_agent();
    let mut req = agent.get(&p.url);
    req = apply_headers(req, p.headers.as_ref());
    let mut resumed = false;
    if existing_len > 0 {
        req = req.set("Range", &format!("bytes={}-", existing_len));
        resumed = true;
    }
    let resp = req.call().map_err(|e| HapError::internal(e.to_string()))?;
    let status = resp.status();
    if status == 206 || (status == 200 && !resumed) {
        if let Some(parent) = std::path::Path::new(&p.dest_path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = if resumed && status == 206 {
            std::fs::OpenOptions::new().append(true).open(&p.dest_path)?
        } else {
            std::fs::File::create(&p.dest_path)?
        };
        let mut reader = resp.into_reader();
        let written = std::io::copy(&mut reader, &mut file)? as i64;
        let total = existing_len as i64 + written;
        let rid = format!("dl_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
        Ok(json!({"request_id": rid, "size": total, "path": p.dest_path, "resumed": resumed}))
    } else {
        let mut file = std::fs::File::create(&p.dest_path)?;
        let mut reader = resp.into_reader();
        let size = std::io::copy(&mut reader, &mut file)? as i64;
        let rid = format!("dl_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
        Ok(json!({"request_id": rid, "size": size, "path": p.dest_path, "resumed": false}))
    }
});

// ---------- SSE infrastructure ----------
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::io::BufRead;

struct SseConn {
    stop: Arc<AtomicBool>,
    url: String,
    events: Arc<Mutex<Vec<Value>>>,
    _thread: std::thread::JoinHandle<()>,
}

static SSE_CONNS: LazyLock<Mutex<HashMap<String, SseConn>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

// ---------- cancel ----------
#[derive(Deserialize)]
pub struct CancelParams { pub request_id: String }
hap_fn!(hap_http_cancel, CancelParams, |p| {
    let mut conns = SSE_CONNS.lock().unwrap();
    if let Some(conn) = conns.remove(&p.request_id) {
        conn.stop.store(true, Ordering::SeqCst);
        return Ok(json!(true));
    }
    Err(HapError::invalid_param(format!("request '{}' not found", p.request_id)))
});

// ---------- sse_connect ----------
#[derive(Deserialize)]
pub struct SseConnectParams {
    pub url: String, pub headers: Option<Map<String, Value>>,
    pub callback_id: String,
}
hap_fn!(hap_http_sse_connect, SseConnectParams, |p| {
    let conn_id = format!("sse_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
    let stop = Arc::new(AtomicBool::new(false));
    let events: Arc<Mutex<Vec<Value>>> = Arc::new(Mutex::new(Vec::new()));
    let stop2 = stop.clone();
    let events2 = events.clone();
    let url = p.url.clone();
    let headers = p.headers.clone();

    let thread = std::thread::spawn(move || {
        let agent = build_agent();
        let mut req = agent.get(&url);
        req = req.set("Accept", "text/event-stream");
        req = req.set("Cache-Control", "no-cache");
        req = apply_headers(req, headers.as_ref());
        let resp = match req.call() {
            Ok(r) => r,
            Err(e) => {
                let mut ev = events2.lock().unwrap();
                ev.push(json!({"type": "error", "data": e.to_string()}));
                return;
            }
        };
        let reader = std::io::BufReader::new(resp.into_reader());
        let mut event_type = String::new();
        let mut event_data = String::new();
        let mut event_id = String::new();

        for line_result in reader.lines() {
            if stop2.load(Ordering::SeqCst) { break; }
            let line = match line_result {
                Ok(l) => l,
                Err(_) => break,
            };
            if line.is_empty() {
                if !event_data.is_empty() {
                    let ev = json!({
                        "type": if event_type.is_empty() { "message" } else { &event_type },
                        "data": event_data.trim_end(),
                        "id": if event_id.is_empty() { Value::Null } else { json!(event_id) },
                    });
                    events2.lock().unwrap().push(ev);
                    event_type.clear();
                    event_data.clear();
                    event_id.clear();
                }
                continue;
            }
            if let Some(data) = line.strip_prefix("data:") {
                if !event_data.is_empty() { event_data.push('\n'); }
                event_data.push_str(data.trim_start());
            } else if let Some(et) = line.strip_prefix("event:") {
                event_type = et.trim().to_string();
            } else if let Some(id) = line.strip_prefix("id:") {
                event_id = id.trim().to_string();
            }
            // "retry:" and comments (":") are ignored
        }
        let mut ev = events2.lock().unwrap();
        ev.push(json!({"type": "close", "data": "stream ended"}));
    });

    let cid = conn_id.clone();
    SSE_CONNS.lock().unwrap().insert(conn_id.clone(), SseConn {
        stop,
        url: p.url.clone(),
        events,
        _thread: thread,
    });
    Ok(json!({"conn_id": cid}))
});

// ---------- sse_close ----------
#[derive(Deserialize)]
pub struct SseCloseParams { pub conn_id: String }
hap_fn!(hap_http_sse_close, SseCloseParams, |p| {
    let mut conns = SSE_CONNS.lock().unwrap();
    if let Some(conn) = conns.remove(&p.conn_id) {
        conn.stop.store(true, Ordering::SeqCst);
        return Ok(json!(true));
    }
    Err(HapError::invalid_param(format!("SSE connection '{}' not found", p.conn_id)))
});

// ---------- list_sse ----------
hap_fn!(hap_http_list_sse, serde_json::Value, |_p| {
    let conns = SSE_CONNS.lock().unwrap();
    let list: Vec<Value> = conns.iter().map(|(id, c)| {
        let pending = c.events.lock().unwrap().len();
        json!({"conn_id": id, "url": c.url, "pending_events": pending})
    }).collect();
    Ok(json!(list))
});

// ---------- sse_poll ----------
#[derive(Deserialize)]
pub struct SsePollParams { pub conn_id: String, pub max_events: Option<usize> }
hap_fn!(hap_http_sse_poll, SsePollParams, |p| {
    let conns = SSE_CONNS.lock().unwrap();
    let conn = conns.get(&p.conn_id)
        .ok_or_else(|| HapError::invalid_param(format!("SSE connection '{}' not found", p.conn_id)))?;
    let mut events = conn.events.lock().unwrap();
    let max = p.max_events.unwrap_or(100).min(1000);
    let drain_count = events.len().min(max);
    let drained: Vec<Value> = events.drain(..drain_count).collect();
    Ok(json!(drained))
});
