use std::sync::Arc;

use crate::llm::stub::localize_target;
use crate::perception::sanitize_plan_target;
use crate::orchestration::bootstrap::TaskBootstrapper;
use crate::orchestration::state::Intent;
use crate::orchestration::templates::TemplateRegistry;
use crate::settings::Lang;

/// Map LLM / stub JSON to a validated [`Intent`].
pub(crate) fn intent_from_json_value(
    templates: &TemplateRegistry,
    value: serde_json::Value,
    raw_utterance: &str,
    lang: Lang,
) -> Intent {
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
    let target = sanitize_plan_target(&localize_target(target, lang));
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
    let final_intent = if templates.get(&intent_name).is_some() {
        intent_name
    } else if intent_name == "unknown" || intent_name.is_empty() {
        if raw_utterance.trim().is_empty() {
            "unknown".to_string()
        } else {
            "windows_task".to_string()
        }
    } else {
        "windows_task".to_string()
    };
    Intent {
        intent: final_intent,
        target,
        params,
        raw_utterance: raw_utterance.to_string(),
    }
}

/// Thin wrapper kept for existing call sites and tests.
pub struct IntentRecognizer {
    bootstrapper: TaskBootstrapper,
}

impl IntentRecognizer {
    pub fn new(
        llm: Arc<dyn crate::llm::LlmClient>,
        templates: Arc<TemplateRegistry>,
        lang: Lang,
        timeout_secs: f32,
    ) -> Self {
        Self {
            bootstrapper: TaskBootstrapper::new(llm, templates, lang, timeout_secs),
        }
    }

    pub async fn recognise(&self, utterance: &str) -> Intent {
        self.bootstrapper.bootstrap(utterance).await.0
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
        let rec = IntentRecognizer::new(llm, templates, Lang::Es, 30.0);
        let intent = rec.recognise("Abre Descargas").await;
        assert_eq!(intent.intent, "open_folder");
        assert_eq!(intent.target, "Descargas");
    }

    #[tokio::test]
    async fn fast_path_skips_slow_llm() {
        use std::time::Duration;
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
    async fn unregistered_intent_becomes_windows_task() {
        let llm: Arc<dyn LlmClient> = Arc::new(BogusLlm);
        let templates = Arc::new(crate::orchestration::templates::default_registry());
        let rec = IntentRecognizer::new(llm, templates, Lang::Es, 30.0);
        let intent = rec.recognise("haz café").await;
        assert_eq!(intent.intent, "windows_task");
    }
}
