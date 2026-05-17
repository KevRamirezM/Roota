//! `LlmClient` trait — the surface every Roota LLM backend implements.

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("network/transport error: {0}")]
    Transport(String),
    #[error("model returned non-JSON output: {0}")]
    InvalidJson(String),
    #[error("model returned a non-object JSON value")]
    NotAnObject,
    #[error("timeout after {secs}s")]
    Timeout { secs: f32 },
    #[error("backend unavailable")]
    Unavailable,
}

impl Serialize for LlmError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

#[async_trait::async_trait]
pub trait LlmClient: Send + Sync {
    fn name(&self) -> &str;

    async fn health_check(&self) -> bool;

    async fn complete_text(&self, prompt: &str, system: Option<&str>) -> Result<String, LlmError>;

    async fn complete_json(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<serde_json::Value, LlmError>;
}
