//! Single LLM call for intent classification + task brief (pre-confirmation).

use std::sync::Arc;
use std::time::Duration;

use crate::llm::stub::{classify_utterance_detailed, is_known_intent};
use crate::llm::LlmClient;
use crate::orchestration::brief::{heuristic_brief, parse_brief_json, TaskBrief};
use crate::orchestration::intent::intent_from_json_value;
use crate::orchestration::state::Intent;
use crate::orchestration::templates::TemplateRegistry;
use crate::prompts;
use crate::settings::Lang;

pub struct TaskBootstrapper {
    llm: Arc<dyn LlmClient>,
    templates: Arc<TemplateRegistry>,
    lang: Lang,
    timeout: Duration,
}

impl TaskBootstrapper {
    pub fn new(
        llm: Arc<dyn LlmClient>,
        templates: Arc<TemplateRegistry>,
        lang: Lang,
        timeout_secs: f32,
    ) -> Self {
        Self {
            llm,
            templates,
            lang,
            timeout: Duration::from_secs_f32(timeout_secs.max(3.0)),
        }
    }

    pub async fn bootstrap(&self, utterance: &str) -> (Intent, TaskBrief) {
        let trimmed = utterance.trim();
        if trimmed.is_empty() {
            let intent = Intent::unknown(utterance);
            return (intent.clone(), heuristic_brief(utterance, ""));
        }

        let (fast, rule_matched) = classify_utterance_detailed(trimmed);
        if rule_matched && is_known_intent(&fast) {
            tracing::info!(target: "roota.bootstrap", "stub fast-path for {:?}", trimmed);
            let intent = intent_from_json_value(&self.templates, fast, utterance, self.lang);
            let brief = heuristic_brief(utterance, &intent.target);
            return (intent, brief);
        }

        let allowed = self.templates.known_intents();
        let prompt = prompts::render_unified_bootstrap(trimmed, &allowed);
        let llm_fut = self
            .llm
            .complete_json(&prompt, Some(prompts::SYSTEM_PROMPT));
        let value = match tokio::time::timeout(self.timeout, llm_fut).await {
            Ok(Ok(v)) => {
                tracing::info!(target: "roota.bootstrap", "LLM bootstrap {:?}", trimmed);
                v
            }
            Ok(Err(e)) => {
                tracing::warn!(
                    target: "roota.bootstrap",
                    reason = "stub_fallback",
                    cause = "llm_error",
                    error = %e
                );
                classify_utterance_detailed(trimmed).0
            }
            Err(_) => {
                tracing::warn!(
                    target: "roota.bootstrap",
                    reason = "stub_fallback",
                    cause = "timeout"
                );
                classify_utterance_detailed(trimmed).0
            }
        };

        let intent = intent_from_json_value(&self.templates, value.clone(), utterance, self.lang);
        let brief = match parse_brief_json(value, utterance) {
            Some(b) => b,
            None => {
                tracing::warn!(
                    target: "roota.bootstrap",
                    reason = "stub_fallback",
                    cause = "json_parse_failed",
                    "brief fields missing; using heuristic"
                );
                heuristic_brief(utterance, &intent.target)
            }
        };
        (intent, brief)
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
            Ok(serde_json::json!({
                "intent": "open_folder",
                "target": "Descargas",
                "params": {},
                "goal_summary": "Abrir Descargas",
                "app_hints": ["explorador"],
                "object_hints": ["descargas"],
                "risk_flags": []
            }))
        }
    }

    #[tokio::test]
    async fn bootstrap_maps_json_to_intent_and_brief() {
        let llm: Arc<dyn LlmClient> = Arc::new(CannedLlm);
        let templates = Arc::new(crate::orchestration::templates::default_registry());
        let b = TaskBootstrapper::new(llm, templates, Lang::Es, 30.0);
        let (intent, brief) = b.bootstrap("Por favor abre mi carpeta Descargas ahora").await;
        assert_eq!(intent.intent, "open_folder");
        assert!(brief.object_hints.iter().any(|h| h == "descargas"));
    }
}
