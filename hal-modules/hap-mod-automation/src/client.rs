use hap_common::HapError;
use serde_json::Value;
use std::time::Duration;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const READ_TIMEOUT: Duration = Duration::from_secs(15);

pub struct ApiClient {
    pub port: u16,
    pub token: String,
}

impl ApiClient {
    pub fn new(port: u16, token: String) -> Self {
        Self { port, token }
    }

    fn base_url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{}", self.port, path)
    }

    pub fn get(&self, path: &str) -> Result<Value, HapError> {
        let resp = ureq::builder()
            .timeout_connect(CONNECT_TIMEOUT)
            .timeout(READ_TIMEOUT)
            .build()
            .get(&self.base_url(path))
            .set("Authorization", &format!("Bearer {}", self.token))
            .call()
            .map_err(|e| HapError::internal(format!("api_unavailable: {e}")))?;
        let body: Value = resp.into_json()
            .map_err(|e| HapError::internal(format!("json parse: {e}")))?;
        Ok(body)
    }

    pub fn post(&self, path: &str, body: &Value) -> Result<Value, HapError> {
        let resp = ureq::builder()
            .timeout_connect(CONNECT_TIMEOUT)
            .timeout(READ_TIMEOUT)
            .build()
            .post(&self.base_url(path))
            .set("Authorization", &format!("Bearer {}", self.token))
            .set("Content-Type", "application/json")
            .send_json(body)
            .map_err(|e| HapError::internal(format!("api_unavailable: {e}")))?;
        let resp_body: Value = resp.into_json()
            .map_err(|e| HapError::internal(format!("json parse: {e}")))?;
        Ok(resp_body)
    }

    pub fn app_window_path(&self, app_id: &str, window: &str, endpoint: &str) -> String {
        format!("/api/v1/apps/{}/windows/{}/{}", app_id, window, endpoint)
    }
}

/// 读取 port file，返回 (port, token)
pub fn discover_api() -> Result<(u16, String), HapError> {
    let home = std::env::var("HOME")
        .map_err(|_| HapError::internal("HOME not set"))?;

    // 优先级 1: 环境变量
    if let (Ok(port_str), Ok(token)) = (
        std::env::var("HAP_AUTOMATION_PORT"),
        std::env::var("HAP_AUTOMATION_TOKEN"),
    ) {
        if let Ok(port) = port_str.parse::<u16>() {
            return Ok((port, token));
        }
    }

    // 优先级 2: devtools.port
    let devtools_path = format!("{}/.hiapphub/devtools.port", home);
    if let Ok(content) = std::fs::read_to_string(&devtools_path) {
        if let Ok(v) = serde_json::from_str::<Value>(&content) {
            if let (Some(port), Some(token)) = (
                v.get("port").and_then(|p| p.as_u64()),
                v.get("token").and_then(|t| t.as_str()),
            ) {
                return Ok((port as u16, token.to_string()));
            }
        }
    }

    // 优先级 3: shell.port
    let shell_path = format!("{}/.hiapphub/shell.port", home);
    if let Ok(content) = std::fs::read_to_string(&shell_path) {
        if let Ok(v) = serde_json::from_str::<Value>(&content) {
            if let (Some(port), Some(token)) = (
                v.get("port").and_then(|p| p.as_u64()),
                v.get("token").and_then(|t| t.as_str()),
            ) {
                return Ok((port as u16, token.to_string()));
            }
        }
    }

    Err(HapError::internal("api_unavailable: no port file found (~/.hiapphub/devtools.port or shell.port)"))
}
