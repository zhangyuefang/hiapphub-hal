use serde::Serialize;
use std::fmt;

#[derive(Debug, Serialize)]
pub struct HapError {
    pub code: &'static str,
    pub message: String,
}

impl HapError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }

    pub fn invalid_param(msg: impl Into<String>) -> Self {
        Self::new("INVALID_PARAM", msg)
    }

    pub fn io(e: std::io::Error) -> Self {
        Self::new("IO_ERROR", e.to_string())
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::new("INTERNAL_ERROR", msg)
    }
}

impl fmt::Display for HapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for HapError {}

impl From<serde_json::Error> for HapError {
    fn from(e: serde_json::Error) -> Self {
        Self::new("JSON_ERROR", e.to_string())
    }
}

impl From<std::io::Error> for HapError {
    fn from(e: std::io::Error) -> Self {
        Self::io(e)
    }
}
