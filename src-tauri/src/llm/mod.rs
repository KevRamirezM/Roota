//! LLM client abstraction. Roota always has a working brain — the
//! resilient wrapper falls back to the deterministic stub if the text
//! backend is unreachable or returns an error mid-session.

pub mod client;
pub mod llamacpp;
pub mod ollama;
pub mod resilient;
pub mod stub;

pub use client::{LlmClient, LlmError};
pub use llamacpp::LlamaCppClient;
pub use ollama::OllamaClient;
pub use resilient::ResilientLlmClient;
pub use stub::StubLlmClient;

use crate::settings::{LlmBackend, Settings};
use std::sync::Arc;

/// Synchronous startup path for Tauri `.setup()` — never call `block_on` there.
pub fn build_llm_sync(settings: &Settings) -> Arc<dyn LlmClient> {
    let fallback = StubLlmClient;
    match settings.llm_backend {
        LlmBackend::Ollama => {
            let c = OllamaClient::new(settings);
            if c.health_check_blocking() {
                tracing::info!(target: "roota.llm", backend = "ollama", "text LLM ready");
                Arc::new(ResilientLlmClient::new(c, fallback))
            } else {
                tracing::warn!(target: "roota.llm", "Ollama unreachable; stub only");
                Arc::new(fallback)
            }
        }
        LlmBackend::LlamaCpp => {
            let c = LlamaCppClient::new(settings);
            if c.health_check_blocking() {
                tracing::info!(target: "roota.llm", backend = "llamacpp", "text LLM ready");
                Arc::new(ResilientLlmClient::new(c, fallback))
            } else {
                tracing::warn!(target: "roota.llm", "llama-server unreachable; stub only");
                Arc::new(fallback)
            }
        }
    }
}

pub async fn build_llm(settings: &Settings) -> Arc<dyn LlmClient> {
    build_llm_sync(settings)
}
