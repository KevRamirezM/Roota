//! LLM client abstraction. Roota always has a working brain — the
//! resilient wrapper falls back to the deterministic stub if Ollama is
//! unreachable or returns an error mid-session.

pub mod client;
pub mod ollama;
pub mod resilient;
pub mod stub;

pub use client::{LlmClient, LlmError};
pub use ollama::OllamaClient;
pub use resilient::ResilientLlmClient;
pub use stub::StubLlmClient;

use crate::settings::Settings;
use std::sync::Arc;

pub async fn build_llm(settings: &Settings) -> Arc<dyn LlmClient> {
    let primary = OllamaClient::new(settings);
    let fallback = StubLlmClient;
    if primary.health_check().await {
        tracing::info!(target: "roota.llm", "Using Ollama as primary with stub fallback");
        Arc::new(ResilientLlmClient::new(primary, fallback))
    } else {
        tracing::warn!(target: "roota.llm", "Ollama unreachable; using deterministic stub");
        Arc::new(fallback)
    }
}
