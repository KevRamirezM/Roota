use thiserror::Error;

use crate::accessibility::element::{UiElement, UiSnapshot};
use crate::i18n;
use crate::orchestration::state::{ActionVerb, GuideStep, Intent, SessionState};
use crate::orchestration::templates::{GuidanceTemplate, StepBlueprint};
use crate::safety::{ActionType, GuideAction, SafetyGuard};
use crate::settings::Lang;

#[derive(Debug, Error)]
pub enum StepResolutionError {
    #[error("template exhausted")]
    NoMoreSteps,
    #[error("safety violation: {0}")]
    Unsafe(#[from] crate::safety::UnsafeActionError),
}

pub struct DecisionEngine {
    safety: SafetyGuard,
    lang: Lang,
}

impl DecisionEngine {
    pub fn new(lang: Lang) -> Self {
        Self {
            safety: SafetyGuard::default(),
            lang,
        }
    }

    pub fn next_step(
        &self,
        intent: &Intent,
        template: &GuidanceTemplate,
        snapshot: &UiSnapshot,
        session: &SessionState,
    ) -> Result<GuideStep, StepResolutionError> {
        if session.step_index >= template.steps.len() {
            return Err(StepResolutionError::NoMoreSteps);
        }
        let blueprint = &template.steps[session.step_index];
        let target_text = materialise_target(blueprint, intent);
        let element = find_element(snapshot, &target_text, blueprint, blueprint.action);
        let anchor = element.map(UiElement::center);
        let anchor_bounds = element.map(|e| (e.x, e.y, e.width, e.height));

        let instruction = i18n::t(
            &blueprint.instruction_key,
            self.lang,
            &[("target", &target_text)],
        );

        self.safety.review(GuideAction {
            kind: ActionType::Anchor,
            target: Some(target_text.clone()),
            payload: Some(instruction.clone()),
        })?;

        Ok(GuideStep {
            index: session.step_index + 1,
            total: template.steps.len(),
            action: blueprint.action,
            target_text,
            instruction,
            anchor_xy: anchor,
            anchor_bounds,
        })
    }
}

fn materialise_target(blueprint: &StepBlueprint, intent: &Intent) -> String {
    let mut text = blueprint.target_query.replace("{target}", &intent.target);
    for (k, v) in &intent.params {
        text = text.replace(&format!("{{{k}}}"), v);
    }
    text
}

fn find_element<'a>(
    snapshot: &'a UiSnapshot,
    target_text: &str,
    blueprint: &StepBlueprint,
    action: ActionVerb,
) -> Option<&'a UiElement> {
    if target_text.is_empty() {
        return None;
    }
    let mut queries = search_queries(target_text);
    if blueprint.target_query != target_text {
        queries.extend(search_queries(&blueprint.target_query));
    }
    for token in target_text.split_whitespace() {
        if token.chars().count() <= 2 {
            continue;
        }
        queries.extend(search_queries(token));
    }
    snapshot.find_best_for_action(&queries, action)
}

/// Bilingual / alias variants for common Explorer and browser labels.
fn search_queries(target: &str) -> Vec<String> {
    let mut out = vec![target.trim().to_string()];
    let lower = target.trim().to_lowercase();
    let aliases: &[&str] = match lower.as_str() {
        "descargas" => &["downloads", "descargas"],
        "downloads" => &["descargas", "downloads"],
        "documentos" => &["documents", "documentos"],
        "documents" => &["documentos", "documents"],
        "imágenes" | "imagenes" => &["pictures", "imágenes"],
        "pictures" => &["imágenes", "pictures"],
        "escritorio" => &["desktop", "escritorio"],
        "desktop" => &["escritorio", "desktop"],
        "nueva pestaña" | "nueva pestana" => &["new tab", "nueva pestaña"],
        "new tab" => &["nueva pestaña", "new tab"],
        "redactar" => &["compose", "redactar"],
        "compose" => &["redactar", "compose"],
        "bandeja de entrada" => &["inbox", "bandeja de entrada"],
        "inbox" => &["bandeja de entrada", "inbox"],
        _ => &[],
    };
    for alias in aliases {
        if !out.iter().any(|s| s.eq_ignore_ascii_case(alias)) {
            out.push((*alias).to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accessibility::element::UiElement;
    use crate::orchestration::templates::default_registry;

    fn snapshot(elements: Vec<UiElement>, window: &str) -> UiSnapshot {
        UiSnapshot {
            window: window.into(),
            elements,
        }
    }

    fn el(text: &str, x: i32, y: i32) -> UiElement {
        UiElement {
            kind: "button".into(),
            text: text.into(),
            x,
            y,
            width: 160,
            height: 32,
            automation_id: Some(text.to_lowercase()),
            window: "Explorer".into(),
        }
    }

    #[test]
    fn open_folder_targets_descargas() {
        let registry = default_registry();
        let template = registry.get("open_folder").unwrap();
        let intent = Intent {
            intent: "open_folder".into(),
            target: "Descargas".into(),
            params: Default::default(),
            raw_utterance: "abre".into(),
        };
        let snap = snapshot(vec![el("Descargas", 100, 300)], "Explorer");
        let mut session = SessionState::default();
        session.begin(intent.clone(), template.steps.len());
        let engine = DecisionEngine::new(Lang::Es);
        let step = engine
            .next_step(&intent, template, &snap, &session)
            .unwrap();
        assert_eq!(step.target_text, "Descargas");
        assert_eq!(step.anchor_xy, Some((180, 316)));
        assert_eq!(step.anchor_bounds, Some((100, 300, 160, 32)));
    }

    #[test]
    fn descargas_matches_downloads_alias() {
        let registry = default_registry();
        let template = registry.get("open_folder").unwrap();
        let intent = Intent {
            intent: "open_folder".into(),
            target: "Descargas".into(),
            params: Default::default(),
            raw_utterance: "abre".into(),
        };
        let snap = snapshot(vec![el("Downloads", 50, 200)], "Explorer");
        let mut session = SessionState::default();
        session.begin(intent.clone(), template.steps.len());
        let engine = DecisionEngine::new(Lang::Es);
        let step = engine
            .next_step(&intent, template, &snap, &session)
            .unwrap();
        assert!(step.anchor_xy.is_some());
    }

    #[test]
    fn missing_element_returns_step_without_anchor() {
        let registry = default_registry();
        let template = registry.get("open_folder").unwrap();
        let intent = Intent {
            intent: "open_folder".into(),
            target: "Música".into(),
            params: Default::default(),
            raw_utterance: "abre".into(),
        };
        let snap = snapshot(vec![el("Descargas", 100, 300)], "Explorer");
        let mut session = SessionState::default();
        session.begin(intent.clone(), template.steps.len());
        let engine = DecisionEngine::new(Lang::Es);
        let step = engine
            .next_step(&intent, template, &snap, &session)
            .unwrap();
        assert!(step.anchor_xy.is_none());
        assert!(step.anchor_bounds.is_none());
        assert_eq!(step.target_text, "Música");
    }
}
