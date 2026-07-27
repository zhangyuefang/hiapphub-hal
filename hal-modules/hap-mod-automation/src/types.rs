use serde::Deserialize;

#[derive(Deserialize)]
pub struct ConnectParams {
    #[serde(rename = "appId")]
    pub app_id: String,
    pub window: Option<String>,
}

#[derive(Deserialize)]
pub struct DisconnectParams {
    pub conn_id: String,
}

#[derive(Deserialize)]
pub struct EmptyParams {}
