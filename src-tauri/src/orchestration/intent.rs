use std::sync::Arc;
use std::time::Duration;

use crate::llm::stub::{classify_utterance, is_known_intent, localize_target, StubLlmClient};
use crate::llm::LlmClient;
use crate::orchestration::state::Intent;
use crate::orchestration::templates::TemplateRegistry;
use crate::prompts;
use crate::settings::Lang;

pub struct IntentRecognizer {
    llm: Arc<dyn LlmClient>,
    templates: Arc<TemplateRegistry>,
    lang: Lang,
    intent_timeout: Duration,
}

impl IntentRecognizer {
    pub fn new(
        llm: Arc<dyn LlmClient>,
        templates: Arc<TemplateRegistry>,
        lang: Lang,
        intent_timeout_secs: f32,
    ) -> Self {
        Self {
            llm,
            templates,
            lang,
            intent_timeout: Duration::from_secs_f32(intent_timeout_secs.max(3.0)),
        }
    }

    pub async fn recognise(&self, utterance: &str) -> Intent {
        let trimmed = utterance.trim();
        if trimmed.is_empty() {
            return Intent::unknown(utterance);
        }

        // Fast path: deterministic rules on the raw utterance (instant).
        let fast = classify_utterance(trimmed);
        if is_known_intent(&fast) {
            tracing::info!(target: "roota.intent", "stub fast-path for {:?}", trimmed);
            return self.value_to_intent(fast, utterance);
        }

        // Slow path: Ollama with a short cap so the UI is not blocked for 30s.
        let prompt = prompts::render_intent_classifier(trimmed);
        let llm_fut = self
            .llm
            .complete_json(&prompt, Some(prompts::SYSTEM_PROMPT));
        let value = match tokio::time::timeout(self.intent_timeout, llm_fut).await {
            Ok(Ok(v)) => {
                tracing::info!(target: "roota.intent", "LLM classified {:?}", trimmed);
                v
            }
            Ok(Err(err)) => {
                tracing::warn!(
                    target: "roota.intent",
                    "LLM JSON failed ({err}); using stub fallback"
                );
                StubLlmClient
                    .complete_json(trimmed, None)
                    .await
                    .unwrap_or_else(|_| classify_utterance(trimmed))
            }
            Err(_) => {
                tracing::warn!(
                    target: "roota.intent",
                    "LLM timed out after {:?}; using stub fallback",
                    self.intent_timeout
                );
                classify_utterance(trimmed)
            }
        };

        self.value_to_intent(value, utterance)
    }

    fn value_to_intent(&self, value: serde_json::Value, raw_utterance: &str) -> Intent {
        let intent_name = value
            .get("intent")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .trim()
            .to_lowercase();
        let target = value
            .get("target")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let target = localize_target(target, self.lang);
        let mut params = std::collections::BTreeMap::new();
        if let Some(obj) = value.get("params").and_then(|v| v.as_object()) {
            for (k, v) in obj {
                let s = match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                params.insert(k.clone(), s);
            }
        }
        let final_intent = if self.templates.get(&intent_name).is_none() {
            "unknown".to_string()
        } else {
            intent_name
        };
        Intent {
            intent: final_intent,
            target,
            params,
            raw_utterance: raw_utterance.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::client::{LlmClient, LlmError};
    use serde_json::Value;

    struct CannedLlm;

    #[async_trait::async_trait]
    impl LlmClient for CannedLlm {
        fn name(&self) -> &str {
            "canned"
        }
        async fn health_check(&self) -> bool {
            true
        }
        async fn complete_text(&self, _: &str, _: Option<&str>) -> Result<String, LlmError> {
            Ok(String::new())
        }
        async fn complete_json(&self, _: &str, _: Option<&str>) -> Result<Value, LlmError> {
            Ok(serde_json::json!({"intent": "open_folder", "target": "Descargas", "params": {}}))
        }
    }

    #[tokio::test]
    async fn known_intent_resolves() {
        let llm: Arc<dyn LlmClient> = Arc::new(CannedLlm);
        let templates = Arc::new(crate::orchestration::templates::default_registry());
        let rec = IntentRecognizer::new(llm, templates, Lang::Es, 10.0);
        let intent = rec.recognise("Abre Descargas").await;
        assert_eq!(intent.intent, "open_folder");
        assert_eq!(intent.target, "Descargas");
    }

    #[tokio::test]
    async fn fast_path_skips_slow_llm() {
        struct SlowLlm;
        #[async_trait::async_trait]
        impl LlmClient for SlowLlm {
            fn name(&self) -> &str {
                "slow"
            }
            async fn health_check(&self) -> bool {
                true
            }
            async fn complete_text(&self, _: &str, _: Option<&str>) -> Result<String, LlmError> {
                Ok(String::new())
            }
            async fn complete_json(&self, _: &str, _: Option<&str>) -> Result<Value, LlmError> {
                tokio::time::sleep(Duration::from_secs(60)).await;
                Ok(serde_json::json!({"intent": "unknown", "target": "", "params": {}}))
            }
        }
        let llm: Arc<dyn LlmClient> = Arc::new(SlowLlm);
        let templates = Arc::new(crate::orchestration::templates::default_registry());
        let rec = IntentRecognizer::new(llm, templates, Lang::Es, 10.0);
        let started = std::time::Instant::now();
        let intent = rec.recognise("Abre la carpeta de Descargas").await;
        assert!(started.elapsed() < Duration::from_secs(2));
        assert_eq!(intent.intent, "open_folder");
    }

    struct BogusLlm;

    #[async_trait::async_trait]
    impl LlmClient for BogusLlm {
        fn name(&self) -> &str {
            "bogus"
        }
        async fn health_check(&self) -> bool {
            true
        }
        async fn complete_text(&self, _: &str, _: Option<&str>) -> Result<String, LlmError> {
            Ok(String::new())
        }
        async fn complete_json(&self, _: &str, _: Option<&str>) -> Result<Value, LlmError> {
            Ok(serde_json::json!({"intent": "make_coffee", "target": "espresso", "params": {}}))
        }
    }

    #[tokio::test]
    async fn unregistered_intent_becomes_unknown() {
        let llm: Arc<dyn LlmClient> = Arc::new(BogusLlm);
        let templates = Arc::new(crate::orchestration::templates::default_registry());
        let rec = IntentRecognizer::new(llm, templates, Lang::Es, 10.0);
        let intent = rec.recognise("haz café").await;
        assert_eq!(intent.intent, "unknown");
    }
}
