//! Resilient wrapper: try the primary client, swap to fallback on per-call error.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde_json::Value;

use crate::llm::client::{LlmClient, LlmError};

pub struct ResilientLlmClient {
    primary: Arc<dyn LlmClient>,
    fallback: Arc<dyn LlmClient>,
    primary_healthy: AtomicBool,
}

impl ResilientLlmClient {
    pub fn new<P, F>(primary: P, fallback: F) -> Self
    where
        P: LlmClient + 'static,
        F: LlmClient + 'static,
    {
        Self {
            primary: Arc::new(primary),
            fallback: Arc::new(fallback),
            primary_healthy: AtomicBool::new(true),
        }
    }

    pub fn active_backend(&self) -> &str {
        if self.primary_healthy.load(Ordering::Relaxed) {
            self.primary.name()
        } else {
            self.fallback.name()
        }
    }

    pub fn reset(&self) {
        self.primary_healthy.store(true, Ordering::Relaxed);
    }
}

#[async_trait::async_trait]
impl LlmClient for ResilientLlmClient {
    fn name(&self) -> &str {
        "resilient"
    }

    async fn health_check(&self) -> bool {
        self.primary.health_check().await || self.fallback.health_check().await
    }

    async fn complete_text(&self, prompt: &str, system: Option<&str>) -> Result<String, LlmError> {
        if self.primary_healthy.load(Ordering::Relaxed) {
            match self.primary.complete_text(prompt, system).await {
                Ok(value) => return Ok(value),
                Err(err) => {
                    tracing::warn!(target: "roota.llm.resilient", "primary failed: {err}; using fallback");
                    self.primary_healthy.store(false, Ordering::Relaxed);
                }
            }
        }
        self.fallback.complete_text(prompt, system).await
    }

    async fn complete_json(&self, prompt: &str, system: Option<&str>) -> Result<Value, LlmError> {
        if self.primary_healthy.load(Ordering::Relaxed) {
            match self.primary.complete_json(prompt, system).await {
                Ok(value) => return Ok(value),
                Err(err) => {
                    tracing::warn!(target: "roota.llm.resilient", "primary JSON failed: {err}; using fallback");
                    self.primary_healthy.store(false, Ordering::Relaxed);
                }
            }
        }
        self.fallback.complete_json(prompt, system).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::stub::StubLlmClient;
    use serde_json::Value;

    struct Failing;

    #[async_trait::async_trait]
    impl LlmClient for Failing {
        fn name(&self) -> &str {
            "failing"
        }
        async fn health_check(&self) -> bool {
            true
        }
        async fn complete_text(&self, _: &str, _: Option<&str>) -> Result<String, LlmError> {
            Err(LlmError::Unavailable)
        }
        async fn complete_json(&self, _: &str, _: Option<&str>) -> Result<Value, LlmError> {
            Err(LlmError::Unavailable)
        }
    }

    #[tokio::test]
    async fn falls_back_on_primary_text_error() {
        let r = ResilientLlmClient::new(Failing, StubLlmClient);
        let out = r.complete_text("hola", None).await.unwrap();
        assert!(!out.is_empty());
        assert_eq!(r.active_backend(), "stub");
    }

    #[tokio::test]
    async fn falls_back_on_primary_json_error() {
        let r = ResilientLlmClient::new(Failing, StubLlmClient);
        let v = r
            .complete_json("Abre la carpeta de Descargas", None)
            .await
            .unwrap();
        assert_eq!(v["intent"], "open_folder");
        assert_eq!(r.active_backend(), "stub");
    }

    #[tokio::test]
    async fn reset_re_enables_primary() {
        let r = ResilientLlmClient::new(Failing, StubLlmClient);
        let _ = r.complete_text("trigger", None).await;
        assert_eq!(r.active_backend(), "stub");
        r.reset();
        assert_eq!(r.active_backend(), "failing");
    }
}
