//! Deterministic offline fallback LLM. The hackathon laptop sometimes
//! runs out of RAM mid-session, so the stub is the safety net.

use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::{json, Value};

use crate::llm::client::{LlmClient, LlmError};
use crate::settings::Lang;

#[derive(Debug, Clone, Default)]
pub struct StubLlmClient;

struct Rule {
    pattern: Regex,
    intent: &'static str,
    target: &'static str,
}

static RULES: Lazy<Vec<Rule>> = Lazy::new(|| {
    let r = |p: &str, intent: &'static str, target: &'static str| Rule {
        pattern: Regex::new(&format!(r"(?i){p}")).expect("regex compile"),
        intent,
        target,
    };
    vec![
        r(
            r"(abrir|abre|open).*(descarga|descargas)",
            "open_folder",
            "Descargas",
        ),
        r(
            r"(abrir|abre|open).*(download|downloads)",
            "open_folder",
            "Downloads",
        ),
        r(
            r"(abrir|abre|open).*(documento|documentos)",
            "open_folder",
            "Documentos",
        ),
        r(
            r"(abrir|abre|open).*(documents)",
            "open_folder",
            "Documents",
        ),
        r(
            r"(mover|move).*(foto|photo|file|archivo)",
            "move_file",
            "selected_file",
        ),
        r(
            r"(borr|elimin|delete|remove)",
            "delete_file",
            "selected_file",
        ),
        r(r"(buscar|search|google)", "search_web", "Chrome"),
        r(
            r"(escrib|enviar|send|write|email|correo).*(elena|hija|hijo|son|daughter|amig)",
            "compose_email",
            "Elena",
        ),
        r(r"(correo|email|gmail)", "compose_email", ""),
        r(r"(chrome|navegador|browser)", "open_browser", "Chrome"),
        r(r"(word|documento de word)", "open_word_document", ""),
    ]
});

/// Classify a raw user utterance (not the full classifier prompt).
pub fn classify_utterance(utterance: &str) -> Value {
    for rule in RULES.iter() {
        if rule.pattern.is_match(utterance) {
            return json!({
                "intent": rule.intent,
                "target": rule.target,
                "params": {},
            });
        }
    }
    json!({ "intent": "unknown", "target": "", "params": {} })
}

pub fn is_known_intent(value: &Value) -> bool {
    value
        .get("intent")
        .and_then(|v| v.as_str())
        .is_some_and(|i| i != "unknown")
}

/// Map English UI labels to Spanish Explorer names when needed.
pub fn localize_target(target: &str, lang: Lang) -> String {
    if lang != Lang::Es {
        return target.to_string();
    }
    match target {
        "Downloads" => "Descargas".into(),
        "Documents" => "Documentos".into(),
        _ => target.to_string(),
    }
}

fn extract_user_utterance<'a>(prompt: &'a str) -> &'a str {
    if let Some(start) = prompt.rfind("Petición:") {
        let tail = &prompt[start..];
        if let Some(q0) = tail.find('"') {
            let rest = &tail[q0 + 1..];
            if let Some(q1) = rest.find('"') {
                return &rest[..q1];
            }
        }
    }
    prompt
}

#[async_trait::async_trait]
impl LlmClient for StubLlmClient {
    fn name(&self) -> &str {
        "stub"
    }

    async fn health_check(&self) -> bool {
        true
    }

    async fn complete_text(&self, prompt: &str, _system: Option<&str>) -> Result<String, LlmError> {
        let utterance = extract_user_utterance(prompt);
        let lowered = utterance.to_lowercase();
        let resp = if lowered.contains("hola") || lowered.contains("hello") {
            "¿Qué tarea quieres que haga por ti hoy?"
        } else if lowered.contains("perfecto") || lowered.contains("great") {
            "Sigamos con el siguiente paso."
        } else {
            "Aún no sé hacer eso. ¿Puedes decírmelo de otra forma?"
        };
        Ok(resp.to_string())
    }

    async fn complete_json(&self, prompt: &str, _system: Option<&str>) -> Result<Value, LlmError> {
        Ok(classify_utterance(extract_user_utterance(prompt)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_spanish_downloads_folder() {
        let v = classify_utterance("Abre la carpeta de Descargas");
        assert_eq!(v["intent"], "open_folder");
        assert_eq!(v["target"], "Descargas");
    }

    #[tokio::test]
    async fn classifies_compose_email_for_elena() {
        let stub = StubLlmClient;
        let v = stub
            .complete_json("Quiero escribir un correo para mi hija Elena", None)
            .await
            .unwrap();
        assert_eq!(v["intent"], "compose_email");
        assert_eq!(v["target"], "Elena");
    }

    #[tokio::test]
    async fn unknown_when_no_rule_matches() {
        let stub = StubLlmClient;
        let v = stub.complete_json("foo bar baz qux", None).await.unwrap();
        assert_eq!(v["intent"], "unknown");
    }

    #[test]
    fn does_not_match_examples_inside_classifier_prompt() {
        let prompt = include_str!("../../prompts/intent_classifier.txt")
            .replace("{utterance}", "Abre el navegador");
        let v = classify_utterance(extract_user_utterance(&prompt));
        assert_eq!(v["intent"], "open_browser");
    }

    #[test]
    fn localizes_downloads_for_spanish() {
        assert_eq!(localize_target("Downloads", Lang::Es), "Descargas");
    }
}
