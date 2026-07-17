use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Read;
use std::net::TcpStream;
use std::process::{Child, Command};
use std::sync::{atomic::{AtomicU64, Ordering}, Mutex, OnceLock};
use tungstenite::{connect, Message, WebSocket, stream::MaybeTlsStream};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static MSG_ID: AtomicU64 = AtomicU64::new(1);

#[allow(dead_code)]
struct BrowserInstance {
    process: Option<Child>,
    ws_url: String,
    debug_port: u16,
}

#[allow(dead_code)]
struct PageConnection {
    ws: WebSocket<MaybeTlsStream<TcpStream>>,
    target_id: String,
    browser_id: String,
}

static BROWSERS: OnceLock<Mutex<HashMap<String, BrowserInstance>>> = OnceLock::new();
static PAGES: OnceLock<Mutex<HashMap<String, PageConnection>>> = OnceLock::new();

fn browsers() -> &'static Mutex<HashMap<String, BrowserInstance>> {
    BROWSERS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn pages() -> &'static Mutex<HashMap<String, PageConnection>> {
    PAGES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn gen_id(prefix: &str) -> String {
    format!("{}_{}", prefix, NEXT_ID.fetch_add(1, Ordering::Relaxed))
}

fn find_available_port(start: u16) -> u16 {
    for offset in 0..100 {
        let port = start + offset;
        if std::net::TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return port;
        }
    }
    start
}

fn find_browser_executable() -> Option<String> {
    let candidates = if cfg!(target_os = "macos") {
        vec![
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
        ]
    } else if cfg!(target_os = "windows") {
        vec![
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        ]
    } else {
        vec![
            "/usr/bin/google-chrome",
            "/usr/bin/chromium-browser",
            "/usr/bin/chromium",
        ]
    };
    candidates.into_iter().find(|p| std::path::Path::new(p).exists()).map(String::from)
}

fn http_request(host: &str, port: u16, method: &str, path: &str) -> Result<String, HapError> {
    let url = format!("http://{}:{}{}", host, port, path);
    let output = Command::new("curl")
        .args(["-s", "-X", method, "--connect-timeout", "2", "--max-time", "5", &url])
        .output()
        .map_err(|e| HapError::internal(format!("curl exec: {e}")))?;
    if !output.status.success() {
        return Err(HapError::internal(format!("curl failed: status={}", output.status)));
    }
    String::from_utf8(output.stdout)
        .map_err(|e| HapError::internal(format!("invalid utf8: {e}")))
}

fn http_get(host: &str, port: u16, path: &str) -> Result<String, HapError> {
    http_request(host, port, "GET", path)
}

fn http_get_json(host: &str, port: u16, path: &str) -> Result<Value, HapError> {
    let body = http_get(host, port, path)?;
    serde_json::from_str(&body)
        .map_err(|e| HapError::internal(format!("json parse: {e}")))
}

fn http_get_json_array(host: &str, port: u16, path: &str) -> Result<Vec<Value>, HapError> {
    let body = http_get(host, port, path)?;
    serde_json::from_str(&body)
        .map_err(|e| HapError::internal(format!("json parse: {e}")))
}

fn send_cdp_command(ws: &mut WebSocket<MaybeTlsStream<TcpStream>>, method: &str, params: Value) -> Result<Value, HapError> {
    let id = MSG_ID.fetch_add(1, Ordering::Relaxed);
    let msg = json!({ "id": id, "method": method, "params": params });

    ws.send(Message::Text(msg.to_string()))
        .map_err(|e| HapError::internal(format!("ws send failed: {e}")))?;

    let timeout = std::time::Instant::now();
    loop {
        if timeout.elapsed() > std::time::Duration::from_secs(30) {
            return Err(HapError::internal("CDP response timeout"));
        }
        match ws.read() {
            Ok(Message::Text(text)) => {
                if let Ok(resp) = serde_json::from_str::<Value>(&text) {
                    if resp.get("id").and_then(|v| v.as_u64()) == Some(id) {
                        if let Some(err) = resp.get("error") {
                            return Err(HapError::internal(format!("CDP error: {}", err)));
                        }
                        return Ok(resp.get("result").cloned().unwrap_or(json!({})));
                    }
                }
            }
            Ok(_) => continue,
            Err(e) => return Err(HapError::internal(format!("ws read: {e}"))),
        }
    }
}

#[derive(Deserialize)]
struct LaunchParams {
    executable_path: Option<String>,
    headless: Option<bool>,
    user_data_dir: Option<String>,
    args: Option<Vec<String>>,
}

hap_fn!(hap_browser_launch, LaunchParams, |params| {
    let exe = params.executable_path
        .or_else(find_browser_executable)
        .ok_or_else(|| HapError::internal("no browser found, specify executable_path"))?;

    // Find an available port (start from 9500+ to avoid Cursor/devtools conflicts)
    let base_port = 9500 + (NEXT_ID.fetch_add(1, Ordering::Relaxed) % 100) as u16;
    let port = find_available_port(base_port);

    let mut cmd_args = vec![
        format!("--remote-debugging-port={}", port),
        "--no-first-run".to_string(),
        "--no-default-browser-check".to_string(),
        "--disable-gpu".to_string(),
    ];
    if params.headless.unwrap_or(false) {
        cmd_args.push("--headless=new".to_string());
    }
    if let Some(ref dir) = params.user_data_dir {
        cmd_args.push(format!("--user-data-dir={}", dir));
    } else {
        let tmp = std::env::temp_dir().join(format!("hap_browser_{}", port));
        cmd_args.push(format!("--user-data-dir={}", tmp.display()));
    }
    if let Some(extra) = params.args {
        cmd_args.extend(extra);
    }

    // On macOS, use `open -n -a` to force a new instance (avoids single-instance redirect)
    let is_macos_app = cfg!(target_os = "macos") && exe.contains(".app/");
    let mut child = if is_macos_app {
        // Extract app bundle path (e.g., "/Applications/Microsoft Edge.app")
        let app_path = exe.split(".app/").next().unwrap_or(&exe).to_string() + ".app";
        Command::new("open")
            .args(["-n", "-a", &app_path, "--args"])
            .args(&cmd_args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| HapError::internal(format!("failed to launch browser via open: {e}")))?
    } else {
        Command::new(&exe)
            .args(&cmd_args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| HapError::internal(format!("failed to launch browser: {e}")))?
    };

    // On macOS with `open -n`, the child is the `open` command which exits immediately.
    // The actual browser runs as a separate process. We just poll the CDP port.
    let is_open_cmd = is_macos_app;

    // Wait for browser CDP endpoint
    let start = std::time::Instant::now();
    let mut ws_url = String::new();
    let mut attempt = 0u32;
    while start.elapsed() < std::time::Duration::from_secs(20) {
        let delay = if attempt < 6 { 1000 } else { 500 };
        std::thread::sleep(std::time::Duration::from_millis(delay));
        attempt += 1;

        // For non-open launches, check if process exited prematurely
        if !is_open_cmd {
            if let Ok(Some(status)) = child.try_wait() {
                let stderr_msg = child.stderr.take()
                    .map(|mut s| { let mut b = String::new(); s.read_to_string(&mut b).ok(); b })
                    .unwrap_or_default();
                return Err(HapError::internal(format!(
                    "browser exited with {}, port={}, stderr={}", status, port, &stderr_msg[..stderr_msg.len().min(500)]
                )));
            }
        }

        match http_get_json("127.0.0.1", port, "/json/version") {
            Ok(v) => {
                if let Some(url) = v.get("webSocketDebuggerUrl").and_then(|u| u.as_str()) {
                    ws_url = url.to_string();
                    break;
                }
            }
            Err(_) => continue,
        }
    }

    if ws_url.is_empty() {
        if !is_open_cmd { let _ = child.kill(); }
        return Err(HapError::internal(format!(
            "CDP endpoint not available: port={}, attempts={}, elapsed={}ms", port, attempt, start.elapsed().as_millis()
        )));
    }

    let id = gen_id("browser");
    let process = if is_open_cmd { None } else { Some(child) };
    browsers().lock().unwrap().insert(id.clone(), BrowserInstance {
        process,
        ws_url: ws_url.clone(),
        debug_port: port,
    });

    Ok(json!({ "browser_id": id, "ws_url": ws_url, "port": port }))
});

#[derive(Deserialize)]
struct ConnectParams {
    ws_url: String,
}

hap_fn!(hap_browser_connect, ConnectParams, |params| {
    let (ws, _) = connect(&params.ws_url)
        .map_err(|e| HapError::internal(format!("ws connect failed: {e}")))?;
    drop(ws);

    let port = url::Url::parse(&params.ws_url)
        .ok().and_then(|u| u.port()).unwrap_or(9222);

    let id = gen_id("browser");
    browsers().lock().unwrap().insert(id.clone(), BrowserInstance {
        process: None,
        ws_url: params.ws_url.clone(),
        debug_port: port,
    });
    Ok(json!({ "browser_id": id, "ws_url": params.ws_url }))
});

#[derive(Deserialize)]
struct BrowserIdParams {
    browser_id: String,
}

hap_fn!(hap_browser_close, BrowserIdParams, |params| {
    let mut map = browsers().lock().unwrap();
    if let Some(mut inst) = map.remove(&params.browser_id) {
        let port = inst.debug_port;
        drop(map);

        // Send Browser.close via CDP WebSocket (safe shutdown)
        let ws_url = http_get_json("127.0.0.1", port, "/json/version")
            .ok()
            .and_then(|v| v.get("webSocketDebuggerUrl").and_then(|u| u.as_str()).map(|s| s.to_string()));
        if let Some(url) = ws_url {
            if let Ok((mut ws, _)) = tungstenite::connect(&url) {
                let _ = send_cdp_command(&mut ws, "Browser.close", json!({}));
                let _ = ws.close(None);
            }
        }

        if let Some(ref mut child) = inst.process {
            let _ = child.kill();
            let _ = child.wait();
        }
        // Close all page WebSocket connections
        let mut page_map = pages().lock().unwrap();
        let to_remove: Vec<String> = page_map.iter()
            .filter(|(_, p)| p.browser_id == params.browser_id)
            .map(|(k, _)| k.clone())
            .collect();
        for k in to_remove {
            if let Some(mut pc) = page_map.remove(&k) {
                let _ = pc.ws.close(None);
            }
        }
        Ok(json!(true))
    } else {
        Err(HapError::invalid_param("browser_id not found"))
    }
});

#[derive(Deserialize)]
struct NewPageParams {
    browser_id: String,
    url: Option<String>,
}

hap_fn!(hap_browser_new_page, NewPageParams, |params| {
    let map = browsers().lock().unwrap();
    let inst = map.get(&params.browser_id)
        .ok_or_else(|| HapError::invalid_param("browser_id not found"))?;
    let port = inst.debug_port;
    drop(map);

    let target_url = params.url.unwrap_or_else(|| "about:blank".to_string());

    // Create new target via CDP HTTP API
    let encoded_url = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("url", &target_url)
        .finish();
    let path = format!("/json/new?{}", encoded_url);
    let body = http_request("127.0.0.1", port, "PUT", &path)?;
    let target_info: Value = serde_json::from_str(&body)
        .map_err(|e| HapError::internal(format!("new page json parse: {e}, body={}", &body[..body.len().min(200)])))?;

    let target_id = target_info.get("id").and_then(|v| v.as_str())
        .ok_or_else(|| HapError::internal("no target id in response"))?
        .to_string();
    let ws_url = target_info.get("webSocketDebuggerUrl").and_then(|v| v.as_str())
        .ok_or_else(|| HapError::internal("no ws url for target"))?;

    let (ws, _) = connect(ws_url)
        .map_err(|e| HapError::internal(format!("ws connect to page: {e}")))?;
    if let MaybeTlsStream::Plain(ref tcp) = ws.get_ref() {
        tcp.set_read_timeout(Some(std::time::Duration::from_secs(30))).ok();
    }

    let page_id = gen_id("page");
    pages().lock().unwrap().insert(page_id.clone(), PageConnection {
        ws,
        target_id: target_id.clone(),
        browser_id: params.browser_id.clone(),
    });

    Ok(json!({ "page_id": page_id, "target_id": target_id, "url": target_url }))
});

#[derive(Deserialize)]
struct NavigateParams {
    page_id: String,
    url: String,
    wait_until: Option<String>,
}

hap_fn!(hap_browser_navigate, NavigateParams, |params| {
    let mut map = pages().lock().unwrap();
    let page = map.get_mut(&params.page_id)
        .ok_or_else(|| HapError::invalid_param("page_id not found"))?;

    let result = send_cdp_command(&mut page.ws, "Page.navigate", json!({ "url": params.url }))?;

    // Wait briefly for load event (best-effort, don't block forever)
    let wait = params.wait_until.unwrap_or_else(|| "load".to_string());
    if wait == "load" || wait == "domcontentloaded" {
        let _ = send_cdp_command(&mut page.ws, "Page.enable", json!({}));
        // Set a short read timeout for event listening
        if let MaybeTlsStream::Plain(ref tcp) = page.ws.get_ref() {
            tcp.set_read_timeout(Some(std::time::Duration::from_secs(5))).ok();
        }
        let nav_timeout = std::time::Instant::now();
        while nav_timeout.elapsed() < std::time::Duration::from_secs(10) {
            match page.ws.read() {
                Ok(Message::Text(text)) => {
                    if let Ok(evt) = serde_json::from_str::<Value>(&text) {
                        let method = evt.get("method").and_then(|m| m.as_str()).unwrap_or("");
                        if (wait == "load" && method == "Page.loadEventFired") ||
                           (wait == "domcontentloaded" && method == "Page.domContentEventFired") {
                            break;
                        }
                    }
                }
                Ok(_) => continue,
                Err(_) => break,
            }
        }
        // Restore read timeout
        if let MaybeTlsStream::Plain(ref tcp) = page.ws.get_ref() {
            tcp.set_read_timeout(Some(std::time::Duration::from_secs(30))).ok();
        }
    }

    Ok(json!({
        "page_id": params.page_id,
        "url": params.url,
        "frame_id": result.get("frameId").cloned().unwrap_or(json!(null))
    }))
});

#[derive(Deserialize)]
struct EvaluateParams {
    page_id: String,
    expression: String,
}

hap_fn!(hap_browser_evaluate, EvaluateParams, |params| {
    let mut map = pages().lock().unwrap();
    let page = map.get_mut(&params.page_id)
        .ok_or_else(|| HapError::invalid_param("page_id not found"))?;

    let result = send_cdp_command(&mut page.ws, "Runtime.evaluate", json!({
        "expression": params.expression,
        "returnByValue": true
    }))?;

    let value = result.get("result").and_then(|r| r.get("value")).cloned().unwrap_or(json!(null));
    let exception = result.get("exceptionDetails").cloned();

    if let Some(exc) = exception {
        Err(HapError::internal(format!("JS error: {}", exc)))
    } else {
        Ok(json!({ "value": value, "type": result.get("result").and_then(|r| r.get("type")).cloned().unwrap_or(json!("undefined")) }))
    }
});

#[derive(Deserialize)]
struct ClickParams {
    page_id: String,
    selector: String,
}

hap_fn!(hap_browser_click, ClickParams, |params| {
    let mut map = pages().lock().unwrap();
    let page = map.get_mut(&params.page_id)
        .ok_or_else(|| HapError::invalid_param("page_id not found"))?;

    // Get element coordinates via JS
    let escaped_sel = params.selector.replace('\\', "\\\\").replace('\'', "\\'");
    let js = format!(
        r#"(function(){{ var el = document.querySelector('{}'); if(!el) return null; var r = el.getBoundingClientRect(); return {{x: r.x + r.width/2, y: r.y + r.height/2}}; }})()"#,
        escaped_sel
    );
    let result = send_cdp_command(&mut page.ws, "Runtime.evaluate", json!({
        "expression": js, "returnByValue": true
    }))?;

    let coords = result.get("result").and_then(|r| r.get("value"));
    match coords {
        Some(v) if !v.is_null() => {
            let x = v.get("x").and_then(|x| x.as_f64()).unwrap_or(0.0);
            let y = v.get("y").and_then(|y| y.as_f64()).unwrap_or(0.0);

            send_cdp_command(&mut page.ws, "Input.dispatchMouseEvent", json!({
                "type": "mousePressed", "x": x, "y": y, "button": "left", "clickCount": 1
            }))?;
            send_cdp_command(&mut page.ws, "Input.dispatchMouseEvent", json!({
                "type": "mouseReleased", "x": x, "y": y, "button": "left", "clickCount": 1
            }))?;
            Ok(json!(true))
        }
        _ => Err(HapError::internal(format!("element not found: {}", params.selector)))
    }
});

#[derive(Deserialize)]
struct TypeTextParams {
    page_id: String,
    selector: String,
    text: String,
    delay_ms: Option<i32>,
}

hap_fn!(hap_browser_type_text, TypeTextParams, |params| {
    let mut map = pages().lock().unwrap();
    let page = map.get_mut(&params.page_id)
        .ok_or_else(|| HapError::invalid_param("page_id not found"))?;

    // Focus element
    let escaped_sel = params.selector.replace('\\', "\\\\").replace('\'', "\\'");
    let js = format!(
        r#"(function(){{ var el = document.querySelector('{}'); if(el){{ el.focus(); return true; }} return false; }})()"#,
        escaped_sel
    );
    send_cdp_command(&mut page.ws, "Runtime.evaluate", json!({ "expression": js, "returnByValue": true }))?;

    let delay = params.delay_ms.unwrap_or(0) as u64;
    for ch in params.text.chars() {
        send_cdp_command(&mut page.ws, "Input.dispatchKeyEvent", json!({
            "type": "keyDown", "text": ch.to_string()
        }))?;
        send_cdp_command(&mut page.ws, "Input.dispatchKeyEvent", json!({
            "type": "keyUp"
        }))?;
        if delay > 0 {
            std::thread::sleep(std::time::Duration::from_millis(delay));
        }
    }
    Ok(json!(true))
});

#[derive(Deserialize)]
struct ScreenshotParams {
    page_id: String,
    path: Option<String>,
    full_page: Option<bool>,
    format: Option<String>,
}

hap_fn!(hap_browser_screenshot, ScreenshotParams, |params| {
    let mut map = pages().lock().unwrap();
    let page = map.get_mut(&params.page_id)
        .ok_or_else(|| HapError::invalid_param("page_id not found"))?;

    let fmt = params.format.unwrap_or_else(|| "png".to_string());
    let mut cdp_params = json!({ "format": fmt });

    if params.full_page.unwrap_or(false) {
        // Get full page metrics
        let metrics = send_cdp_command(&mut page.ws, "Page.getLayoutMetrics", json!({}))?;
        if let Some(content_size) = metrics.get("contentSize") {
            let w = content_size.get("width").and_then(|v| v.as_f64()).unwrap_or(1920.0);
            let h = content_size.get("height").and_then(|v| v.as_f64()).unwrap_or(1080.0);
            cdp_params["clip"] = json!({ "x": 0, "y": 0, "width": w, "height": h, "scale": 1 });
        }
    }

    let result = send_cdp_command(&mut page.ws, "Page.captureScreenshot", cdp_params)?;
    let data = result.get("data").and_then(|d| d.as_str()).unwrap_or("");

    if let Some(ref save_path) = params.path {
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data)
            .map_err(|e| HapError::internal(format!("base64 decode: {e}")))?;
        std::fs::write(save_path, &bytes)
            .map_err(|e| HapError::internal(format!("write file: {e}")))?;
        Ok(json!({ "path": save_path, "size": bytes.len(), "format": fmt }))
    } else {
        Ok(json!({ "data": data, "format": fmt }))
    }
});

#[derive(Deserialize)]
struct GetHtmlParams {
    page_id: String,
    selector: Option<String>,
}

hap_fn!(hap_browser_get_html, GetHtmlParams, |params| {
    let mut map = pages().lock().unwrap();
    let page = map.get_mut(&params.page_id)
        .ok_or_else(|| HapError::invalid_param("page_id not found"))?;

    let js = if let Some(ref sel) = params.selector {
        format!(
            r#"(function(){{ var el = document.querySelector('{}'); return el ? el.outerHTML : null; }})()"#,
            sel.replace('\'', "\\'")
        )
    } else {
        "document.documentElement.outerHTML".to_string()
    };

    let result = send_cdp_command(&mut page.ws, "Runtime.evaluate", json!({
        "expression": js, "returnByValue": true
    }))?;

    let html = result.get("result").and_then(|r| r.get("value"))
        .and_then(|v| v.as_str()).unwrap_or("").to_string();
    Ok(json!(html))
});

#[derive(Deserialize)]
struct WaitForSelectorParams {
    page_id: String,
    selector: String,
    timeout_ms: Option<i32>,
}

hap_fn!(hap_browser_wait_for_selector, WaitForSelectorParams, |params| {
    let timeout = params.timeout_ms.unwrap_or(30000) as u64;
    let start = std::time::Instant::now();
    let interval = std::time::Duration::from_millis(200);

    loop {
        if start.elapsed() > std::time::Duration::from_millis(timeout) {
            return Err(HapError::internal(format!("timeout waiting for selector: {}", params.selector)));
        }

        let mut map = pages().lock().unwrap();
        let page = map.get_mut(&params.page_id)
            .ok_or_else(|| HapError::invalid_param("page_id not found"))?;

        let escaped_sel = params.selector.replace('\\', "\\\\").replace('\'', "\\'");
        let js = format!(
            r#"document.querySelector('{}') !== null"#,
            escaped_sel
        );
        let result = send_cdp_command(&mut page.ws, "Runtime.evaluate", json!({
            "expression": js, "returnByValue": true
        }))?;

        let found = result.get("result").and_then(|r| r.get("value"))
            .and_then(|v| v.as_bool()).unwrap_or(false);
        if found {
            return Ok(json!(true));
        }
        drop(map);
        std::thread::sleep(interval);
    }
});

#[derive(Deserialize)]
struct PageIdParams {
    page_id: String,
}

hap_fn!(hap_browser_get_cookies, PageIdParams, |params| {
    let mut map = pages().lock().unwrap();
    let page = map.get_mut(&params.page_id)
        .ok_or_else(|| HapError::invalid_param("page_id not found"))?;

    let result = send_cdp_command(&mut page.ws, "Network.getCookies", json!({}))?;
    let cookies = result.get("cookies").cloned().unwrap_or(json!([]));
    Ok(cookies)
});

#[derive(Deserialize)]
struct SetCookiesParams {
    page_id: String,
    cookies: Vec<Value>,
}

hap_fn!(hap_browser_set_cookies, SetCookiesParams, |params| {
    let mut map = pages().lock().unwrap();
    let page = map.get_mut(&params.page_id)
        .ok_or_else(|| HapError::invalid_param("page_id not found"))?;

    send_cdp_command(&mut page.ws, "Network.setCookies", json!({ "cookies": params.cookies }))?;
    Ok(json!(true))
});

#[derive(Deserialize)]
struct PdfParams {
    page_id: String,
    path: String,
    landscape: Option<bool>,
}

hap_fn!(hap_browser_pdf, PdfParams, |params| {
    let mut map = pages().lock().unwrap();
    let page = map.get_mut(&params.page_id)
        .ok_or_else(|| HapError::invalid_param("page_id not found"))?;

    let result = send_cdp_command(&mut page.ws, "Page.printToPDF", json!({
        "landscape": params.landscape.unwrap_or(false),
        "printBackground": true
    }))?;

    let data = result.get("data").and_then(|d| d.as_str()).unwrap_or("");
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data)
        .map_err(|e| HapError::internal(format!("base64 decode: {e}")))?;
    std::fs::write(&params.path, &bytes)
        .map_err(|e| HapError::internal(format!("write pdf: {e}")))?;

    Ok(json!({ "path": params.path, "size": bytes.len() }))
});

hap_fn!(hap_browser_list_pages, BrowserIdParams, |params| {
    let map = browsers().lock().unwrap();
    let inst = map.get(&params.browser_id)
        .ok_or_else(|| HapError::invalid_param("browser_id not found"))?;
    let port = inst.debug_port;
    drop(map);

    let targets = http_get_json_array("127.0.0.1", port, "/json/list")?;
    let result: Vec<Value> = targets.into_iter()
        .filter(|t| t.get("type").and_then(|v| v.as_str()) == Some("page"))
        .map(|t| json!({
            "target_id": t.get("id"),
            "url": t.get("url"),
            "title": t.get("title"),
        }))
        .collect();

    Ok(json!(result))
});

hap_fn!(hap_browser_close_page, PageIdParams, |params| {
    let mut map = pages().lock().unwrap();
    if let Some(mut pc) = map.remove(&params.page_id) {
        let _ = send_cdp_command(&mut pc.ws, "Page.close", json!({}));
        let _ = pc.ws.close(None);
        Ok(json!(true))
    } else {
        Err(HapError::invalid_param("page_id not found"))
    }
});

#[derive(Deserialize)]
struct SelectParams {
    page_id: String,
    selector: String,
    value: String,
}

hap_fn!(hap_browser_select, SelectParams, |params| {
    let mut map = pages().lock().unwrap();
    let page = map.get_mut(&params.page_id)
        .ok_or_else(|| HapError::invalid_param("page_id not found"))?;

    let escaped_sel = params.selector.replace('\\', "\\\\").replace('\'', "\\'");
    let escaped_val = params.value.replace('\\', "\\\\").replace('\'', "\\'");
    let js = format!(
        r#"(function(){{ var el = document.querySelector('{}'); if(!el) return false; el.value = '{}'; el.dispatchEvent(new Event('change', {{bubbles:true}})); return true; }})()"#,
        escaped_sel, escaped_val
    );
    let result = send_cdp_command(&mut page.ws, "Runtime.evaluate", json!({
        "expression": js, "returnByValue": true
    }))?;

    let success = result.get("result").and_then(|r| r.get("value"))
        .and_then(|v| v.as_bool()).unwrap_or(false);
    if success { Ok(json!(true)) } else { Err(HapError::internal("element not found")) }
});

#[derive(Deserialize)]
struct QuerySelectorParams {
    page_id: String,
    selector: String,
    attribute: Option<String>,
}

hap_fn!(hap_browser_query_selector, QuerySelectorParams, |params| {
    let mut map = pages().lock().unwrap();
    let page = map.get_mut(&params.page_id)
        .ok_or_else(|| HapError::invalid_param("page_id not found"))?;

    let escaped_sel = params.selector.replace('\\', "\\\\").replace('\'', "\\'");
    let js = if let Some(ref attr) = params.attribute {
        let escaped_attr = attr.replace('\\', "\\\\").replace('\'', "\\'");
        format!(
            r#"(function(){{ var el = document.querySelector('{}'); if(!el) return null; return el.getAttribute('{}'); }})()"#,
            escaped_sel, escaped_attr
        )
    } else {
        format!(
            r#"(function(){{ var el = document.querySelector('{}'); if(!el) return null; var attrs = {{}}; for(var a of el.attributes) attrs[a.name] = a.value; attrs.tagName = el.tagName; attrs.textContent = el.textContent.substring(0,1000); return attrs; }})()"#,
            escaped_sel
        )
    };

    let result = send_cdp_command(&mut page.ws, "Runtime.evaluate", json!({
        "expression": js, "returnByValue": true
    }))?;

    let value = result.get("result").and_then(|r| r.get("value")).cloned().unwrap_or(json!(null));
    if value.is_null() {
        Err(HapError::internal(format!("element not found: {}", params.selector)))
    } else {
        Ok(value)
    }
});
