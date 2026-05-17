//! UNDERSTAND phase — structured goal before planning.

use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;

use crate::accessibility::scanner::ScanContext;
use crate::llm::client::LlmClient;
use crate::prompts;

const BRIEF_TIMEOUT_SECS: f32 = 10.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBrief {
    pub raw_utterance: String,
    pub goal_summary: String,
    pub app_hints: Vec<String>,
    pub object_hints: Vec<String>,
    pub risk_flags: Vec<String>,
}

impl TaskBrief {
    pub fn empty() -> Self {
        Self {
            raw_utterance: String::new(),
            goal_summary: String::new(),
            app_hints: vec![],
            object_hints: vec![],
            risk_flags: vec![],
        }
    }

    pub fn enrich_scan_context(&self, scan_ctx: &mut ScanContext) {
        for h in &self.app_hints {
            let lower = h.to_lowercase();
            if !scan_ctx.window_hints.iter().any(|w| w == &lower) {
                scan_ctx.window_hints.push(lower);
            }
        }
        scan_ctx.enrich_from_utterance(&self.raw_utterance);
    }
}

#[derive(Debug, Deserialize)]
struct BriefJson {
    goal_summary: Option<String>,
    app_hints: Option<Vec<String>>,
    object_hints: Option<Vec<String>>,
    risk_flags: Option<Vec<String>>,
}

pub fn parse_brief_json(value: serde_json::Value, utterance: &str) -> Option<TaskBrief> {
    let parsed: BriefJson = serde_json::from_value(value).ok()?;
    let goal_summary = parsed
        .goal_summary
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| utterance.chars().take(120).collect());
    Some(TaskBrief {
        raw_utterance: utterance.to_string(),
        goal_summary,
        app_hints: normalize_hints(parsed.app_hints.unwrap_or_default()),
        object_hints: normalize_hints(parsed.object_hints.unwrap_or_default()),
        risk_flags: parsed.risk_flags.unwrap_or_default(),
    })
}

fn normalize_hints(mut v: Vec<String>) -> Vec<String> {
    v.retain(|s| !s.trim().is_empty());
    for s in &mut v {
        *s = s.trim().to_lowercase();
    }
    v.sort();
    v.dedup();
    v
}

pub fn heuristic_brief(utterance: &str, goal_target: &str) -> TaskBrief {
    let lower = utterance.to_lowercase();
    let mut app_hints = Vec::new();
    let mut object_hints = Vec::new();
    let mut risk_flags = Vec::new();

    if lower.contains("explorador") || lower.contains("carpeta") || lower.contains("archivo") {
        app_hints.push("explorador".into());
    }
    if lower.contains("chrome") || lower.contains("navegador") || lower.contains("internet") {
        app_hints.push("chrome".into());
    }
    if lower.contains("configuración") || lower.contains("configuracion") || lower.contains("settings")
    {
        app_hints.push("configuración".into());
    }
    if lower.contains("wifi") || lower.contains("wi-fi") || lower.contains("wi fi") {
        app_hints.push("configuración".into());
        object_hints.push("wi-fi".into());
    }
    if lower.contains("bluetooth") {
        app_hints.push("configuración".into());
        object_hints.push("bluetooth".into());
    }
    if lower.contains("volumen") || lower.contains("sonido") {
        object_hints.push("volumen".into());
    }
    if lower.contains("cursor") {
        app_hints.push("cursor".into());
    }
    if lower.contains("correo") || lower.contains("email") || lower.contains("gmail") {
        app_hints.push("correo".into());
        risk_flags.push("email".into());
    }
    if lower.contains("borrar") || lower.contains("eliminar") || lower.contains("delete") {
        risk_flags.push("delete".into());
    }

    if !goal_target.trim().is_empty() {
        object_hints.push(goal_target.trim().to_lowercase());
    }
    for token in utterance.split_whitespace() {
        let t = token
            .trim()
            .trim_matches(|c: char| !c.is_alphanumeric() && c != 'ñ' && c != 'Ñ');
        if t.chars().count() >= 4 {
            let tl = t.to_lowercase();
            if !object_hints.contains(&tl) {
                object_hints.push(tl);
            }
        }
    }

    let goal_summary = if !goal_target.trim().is_empty() {
        format!("Completar: {}", goal_target.trim())
    } else {
        utterance.chars().take(120).collect()
    };

    TaskBrief {
        raw_utterance: utterance.to_string(),
        goal_summary,
        app_hints: normalize_hints(app_hints),
        object_hints: normalize_hints(object_hints),
        risk_flags,
    }
}

pub struct BriefExtractor {
    llm: Arc<dyn LlmClient>,
}

impl BriefExtractor {
    pub fn new(llm: Arc<dyn LlmClient>) -> Self {
        Self { llm }
    }

    pub async fn understand(&self, utterance: &str, goal_target: &str) -> TaskBrief {
        let prompt = prompts::render_task_brief(utterance, goal_target);
        let timeout = Duration::from_secs_f32(BRIEF_TIMEOUT_SECS);
        let llm_fut = self
            .llm
            .complete_json(&prompt, Some(prompts::SYSTEM_PROMPT));
        match tokio::time::timeout(timeout, llm_fut).await {
            Ok(Ok(v)) => parse_brief_json(v, utterance).unwrap_or_else(|| {
                heuristic_brief(utterance, goal_target)
            }),
            Ok(Err(err)) => {
                tracing::warn!(target: "roota.brief", "LLM brief failed: {err}");
                heuristic_brief(utterance, goal_target)
            }
            Err(_) => {
                tracing::warn!(target: "roota.brief", "LLM brief timed out");
                heuristic_brief(utterance, goal_target)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_brief_json_extracts_hints() {
        let v = serde_json::json!({
            "goal_summary": "Abrir la carpeta Descargas",
            "app_hints": ["explorador"],
            "object_hints": ["descargas"],
            "risk_flags": []
        });
        let brief = parse_brief_json(v, "Abre Descargas").unwrap();
        assert_eq!(brief.object_hints, vec!["descargas"]);
        assert_eq!(brief.app_hints, vec!["explorador"]);
    }

    #[test]
    fn heuristic_brief_non_empty_summary() {
        let b = heuristic_brief("Abre Descargas", "Descargas");
        assert!(!b.goal_summary.is_empty());
        assert!(b.object_hints.iter().any(|h| h.contains("descargas")));
    }
}
