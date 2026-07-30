use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex, RwLock};

#[derive(Clone, Debug, Serialize)]
pub struct WsClient {
    pub client_id: String,
    pub role: Option<String>,
    pub window_label: Option<String>,
    pub app_id: Option<String>,
    pub manifest_path: Option<String>,
    #[serde(skip)]
    pub tx: mpsc::UnboundedSender<String>,
}

#[derive(Deserialize)]
pub struct RegisterMsg {
    pub role: Option<String>,
    #[serde(rename = "windowLabel")]
    pub window_label: Option<String>,
    #[serde(rename = "appId")]
    pub app_id: Option<String>,
    #[serde(rename = "manifestPath")]
    pub manifest_path: Option<String>,
    pub manifest: Option<Value>,
    pub port: Option<u16>,
}

pub struct PendingApiRequest {
    pub respond_tx: oneshot::Sender<Value>,
}

pub struct AppQueue {
    pub queue: Vec<Box<dyn FnOnce() + Send>>,
    pub busy: bool,
    pub request_count: u32,
    pub timeout_count: u32,
}

pub struct WsManager {
    pub clients: RwLock<Vec<WsClient>>,
    pub api_responses: Mutex<HashMap<String, PendingApiRequest>>,
    pub on_plugin_register: Mutex<Option<mpsc::UnboundedSender<Value>>>,
    pub on_plugin_disconnect: Mutex<Option<mpsc::UnboundedSender<Value>>>,
    pub on_runner_disconnect: Mutex<Option<mpsc::UnboundedSender<String>>>,
}

impl WsManager {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            clients: RwLock::new(Vec::new()),
            api_responses: Mutex::new(HashMap::new()),
            on_plugin_register: Mutex::new(None),
            on_plugin_disconnect: Mutex::new(None),
            on_runner_disconnect: Mutex::new(None),
        })
    }

    pub async fn add_client(&self, client: WsClient) {
        let mut clients = self.clients.write().await;
        if let Some(app_id) = &client.app_id {
            if client.role.as_deref() == Some("runner") {
                clients.retain(|c| !(c.role.as_deref() == Some("runner") && c.app_id.as_deref() == Some(app_id)));
            }
        }
        clients.push(client);
    }

    pub async fn remove_client(&self, client_id: &str) {
        let mut clients = self.clients.write().await;
        if let Some(pos) = clients.iter().position(|c| c.client_id == client_id) {
            let removed = clients.remove(pos);
            if removed.role.as_deref() == Some("plugin") {
                if let Ok(tx) = self.on_plugin_disconnect.lock().await.as_ref().ok_or(()) {
                    let _ = tx.send(serde_json::json!({
                        "client_id": client_id,
                        "manifest": null
                    }));
                }
            }
            if removed.role.as_deref() == Some("runner") {
                if let Some(app_id) = &removed.app_id {
                    if let Some(tx) = self.on_runner_disconnect.lock().await.as_ref() {
                        let _ = tx.send(app_id.clone());
                    }
                }
            }
        }
    }

    pub async fn handle_message(&self, client_id: &str, raw: &str) {
        let msg: Value = match serde_json::from_str(raw) {
            Ok(v) => v,
            Err(_) => return,
        };
        let msg_type = msg.get("type").and_then(|t| t.as_str()).unwrap_or("");

        match msg_type {
            "register" => {
                if let Ok(reg) = serde_json::from_value::<RegisterMsg>(msg.clone()) {
                    let mut clients = self.clients.write().await;
                    if let Some(c) = clients.iter_mut().find(|c| c.client_id == client_id) {
                        c.role = reg.role.clone();
                        c.window_label = reg.window_label;
                        if reg.app_id.is_some() { c.app_id = reg.app_id.clone(); }
                        if reg.manifest_path.is_some() { c.manifest_path = reg.manifest_path; }
                    }
                    if reg.role.as_deref() == Some("plugin") {
                        if let Some(tx) = self.on_plugin_register.lock().await.as_ref() {
                            let _ = tx.send(msg);
                        }
                    }
                }
            }
            "api:response" => {
                if let Some(request_id) = msg.get("requestId").and_then(|r| r.as_str()) {
                    let mut responses = self.api_responses.lock().await;
                    if let Some(pending) = responses.remove(request_id) {
                        let data = msg.get("data").cloned().unwrap_or(Value::Null);
                        let _ = pending.respond_tx.send(data);
                    }
                }
            }
            "custom" => {
                // plugin custom events — not handled in Rust, pass-through
            }
            _ => {}
        }
    }

    pub async fn send_to_client(&self, client_id: &str, msg: &Value) -> bool {
        let clients = self.clients.read().await;
        if let Some(c) = clients.iter().find(|c| c.client_id == client_id) {
            c.tx.send(serde_json::to_string(msg).unwrap_or_default()).is_ok()
        } else {
            false
        }
    }

    pub async fn broadcast(&self, msg: &Value) {
        let msg_str = serde_json::to_string(msg).unwrap_or_default();
        let clients = self.clients.read().await;
        for c in clients.iter() {
            let _ = c.tx.send(msg_str.clone());
        }
    }

    pub async fn send_to_role(&self, role: &str, msg: &Value) -> usize {
        let msg_str = serde_json::to_string(msg).unwrap_or_default();
        let clients = self.clients.read().await;
        let mut count = 0;
        for c in clients.iter() {
            if c.role.as_deref() == Some(role) {
                if c.tx.send(msg_str.clone()).is_ok() { count += 1; }
            }
        }
        count
    }

    pub async fn find_runner(&self, app_id: &str) -> Option<String> {
        let clients = self.clients.read().await;
        clients.iter()
            .find(|c| c.role.as_deref() == Some("runner") && c.app_id.as_deref() == Some(app_id))
            .map(|c| c.client_id.clone())
    }

    pub async fn has_plugin(&self) -> bool {
        let clients = self.clients.read().await;
        clients.iter().any(|c| c.role.as_deref() == Some("plugin"))
    }

    pub async fn get_clients_info(&self) -> Vec<Value> {
        let clients = self.clients.read().await;
        clients.iter().map(|c| serde_json::json!({
            "clientId": c.client_id,
            "role": c.role,
            "label": c.window_label,
            "appId": c.app_id,
            "manifestPath": c.manifest_path,
        })).collect()
    }

    pub async fn send_api_request(
        &self,
        app_id: &str,
        action: &str,
        params: Option<Value>,
        timeout_ms: u64,
    ) -> Result<Value, String> {
        let client_id = self.find_runner(app_id).await
            .ok_or_else(|| format!("Runner {} not connected", app_id))?;

        let request_id = format!("api_{}_{}", chrono_millis(), &uuid::Uuid::new_v4().to_string()[..8]);

        let (tx, rx) = oneshot::channel();
        {
            let mut responses = self.api_responses.lock().await;
            responses.insert(request_id.clone(), PendingApiRequest { respond_tx: tx });
        }

        let msg = serde_json::json!({
            "type": "api:request",
            "requestId": request_id,
            "action": action,
            "params": params,
        });
        if !self.send_to_client(&client_id, &msg).await {
            let mut responses = self.api_responses.lock().await;
            responses.remove(&request_id);
            return Err(format!("Failed to send to runner {}", app_id));
        }

        match tokio::time::timeout(
            std::time::Duration::from_millis(timeout_ms),
            rx,
        ).await {
            Ok(Ok(data)) => Ok(data),
            Ok(Err(_)) => {
                let mut responses = self.api_responses.lock().await;
                responses.remove(&request_id);
                Err("channel closed".into())
            }
            Err(_) => {
                let mut responses = self.api_responses.lock().await;
                responses.remove(&request_id);
                Err("timeout".into())
            }
        }
    }
}

fn chrono_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
