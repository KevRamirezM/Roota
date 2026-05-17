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

/// Synchronous startup path for Tauri `.setup()` — never call `block_on` there.
pub fn build_llm_sync(settings: &Settings) -> Arc<dyn LlmClient> {
    let primary = OllamaClient::new(settings);
    let fallback = StubLlmClient;
    if primary.health_check_blocking() {
        tracing::info!(target: "roota.llm", "Using Ollama as primary with stub fallback");
        Arc::new(ResilientLlmClient::new(primary, fallback))
    } else {
        tracing::warn!(target: "roota.llm", "Ollama unreachable; using deterministic stub");
        Arc::new(fallback)
    }
}

pub async fn build_llm(settings: &Settings) -> Arc<dyn LlmClient> {
    build_llm_sync(settings)
}
