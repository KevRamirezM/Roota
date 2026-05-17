use thiserror::Error;

use crate::i18n;
use crate::orchestration::state::{ActionVerb, GuideStep, Intent, SessionState};
use crate::orchestration::templates::{GuidanceTemplate, StepBlueprint};
use crate::perception::ScreenElement;
use crate::perception::ScreenFrame;
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
        frame: &ScreenFrame,
        session: &SessionState,
    ) -> Result<GuideStep, StepResolutionError> {
        if session.step_index >= template.steps.len() {
            return Err(StepResolutionError::NoMoreSteps);
        }
        let blueprint = &template.steps[session.step_index];
        let target_text = materialise_target(blueprint, intent);
        let element = find_element(frame, &target_text, blueprint, blueprint.action, intent);
        let anchor = element.map(ScreenElement::center);
        let anchor_bounds = element.map(|e| (e.bounds.x, e.bounds.y, e.bounds.width, e.bounds.height));

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
    frame: &'a ScreenFrame,
    target_text: &str,
    blueprint: &StepBlueprint,
    action: ActionVerb,
    intent: &Intent,
) -> Option<&'a ScreenElement> {
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
    queries.extend(utterance_search_tokens(&intent.raw_utterance));
    if !intent.target.is_empty() && intent.target != target_text {
        queries.extend(search_queries(&intent.target));
        for token in intent.target.split_whitespace() {
            if token.chars().count() > 2 {
                queries.extend(search_queries(token));
            }
        }
    }
    frame.find_best_for_action(&queries, action)
}

/// Keywords from the original request that may match on-screen menu labels.
fn utterance_search_tokens(utterance: &str) -> Vec<String> {
    const STOP: &[&str] = &[
        "como", "cómo", "quiero", "necesito", "puedes", "para", "una", "uno", "los", "las",
        "del", "de", "la", "el", "en", "y", "o", "abrir", "abre", "open", "hacer", "ayuda",
    ];
    let mut out = Vec::new();
    for token in utterance.split_whitespace() {
        let t = token.trim().trim_matches(|c: char| !c.is_alphanumeric() && c != 'ñ' && c != 'Ñ');
        if t.chars().count() < 3 {
            continue;
        }
        let lower = t.to_lowercase();
        if STOP.contains(&lower.as_str()) {
            continue;
        }
        if !out.iter().any(|s: &String| s.eq_ignore_ascii_case(t)) {
            out.push(t.to_string());
            out.extend(search_queries(t));
        }
    }
    out
}

/// Bilingual / alias variants for common desktop labels.
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
        "terminal" => &[
            "terminal",
            "nueva terminal",
            "new terminal",
            "consola",
            "powershell",
            "integrated terminal",
        ],
        "consola" => &["terminal", "consola", "powershell"],
        "powershell" => &["powershell", "terminal"],
        "cursor" => &["cursor", "visual studio code", "vscode"],
        "configuración" | "configuracion" => &["configuración", "settings", "configuracion"],
        "settings" => &["settings", "configuración"],
        "inicio" => &["inicio", "start"],
        "start" => &["start", "inicio"],
        "ver" | "view" => &["ver", "view"],
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
    use crate::orchestration::templates::default_registry;
    use crate::perception::{
        ElementSource, PerceptionQuality, Rect, ScreenElement, ScreenFrame, WindowId,
        WindowSnapshot,
    };

    fn frame_with(elements: Vec<ScreenElement>, primary_title: &str) -> ScreenFrame {
        ScreenFrame {
            primary_window_id: WindowId(1),
            windows: vec![WindowSnapshot {
                id: WindowId(1),
                title: primary_title.into(),
                class_name: "CabinetWClass".into(),
                bounds: Rect::new(0, 0, 1280, 720),
                is_foreground: true,
                z_order: 0,
                uia_element_count: elements.len(),
            }],
            elements,
            quality: PerceptionQuality::Full,
            ..ScreenFrame::empty()
        }
    }

    fn el(text: &str, x: i32, y: i32) -> ScreenElement {
        ScreenElement {
            source: ElementSource::Uia,
            text: text.into(),
            bounds: Rect::new(x, y, 160, 32),
            window_id: WindowId(1),
            kind: "Button".into(),
            confidence: 1.0,
            automation_id: Some(text.to_lowercase()),
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
            raw_utterance: "abre descargas".into(),
        };
        let frame = frame_with(vec![el("Descargas", 100, 300)], "Explorer");
        let mut session = SessionState::default();
        session.begin(intent.clone(), template.steps.len());
        let engine = DecisionEngine::new(Lang::Es);
        let step = engine
            .next_step(&intent, template, &frame, &session)
            .unwrap();
        assert_eq!(step.target_text, "Descargas");
        assert_eq!(step.anchor_xy, Some((180, 316)));
    }

    #[test]
    fn utterance_tokens_help_match_terminal() {
        let intent = Intent {
            intent: "windows_task".into(),
            target: "Terminal".into(),
            params: Default::default(),
            raw_utterance: "como abro una terminal en cursor".into(),
        };
        let frame = frame_with(vec![el("Nueva terminal", 50, 100)], "Cursor");
        let template = GuidanceTemplate {
            intent: "windows_task".into(),
            confirmation_action_key: "confirm.windows_task".into(),
            expected_window: Some("Cursor".into()),
            steps: vec![StepBlueprint {
                action: ActionVerb::Click,
                target_query: "Terminal".into(),
                instruction_key: "guidance.click_target".into(),
                fallback_window: None,
            }],
        };
        let mut session = SessionState::default();
        session.begin(intent.clone(), 1);
        let step = DecisionEngine::new(Lang::Es)
            .next_step(&intent, &template, &frame, &session)
            .unwrap();
        assert!(step.anchor_xy.is_some());
        assert_eq!(step.target_text, "Terminal");
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
        let frame = frame_with(vec![el("Downloads", 50, 200)], "Explorer");
        let mut session = SessionState::default();
        session.begin(intent.clone(), template.steps.len());
        let engine = DecisionEngine::new(Lang::Es);
        let step = engine
            .next_step(&intent, template, &frame, &session)
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
            raw_utterance: "abre música".into(),
        };
        let frame = frame_with(vec![el("Descargas", 100, 300)], "Explorer");
        let mut session = SessionState::default();
        session.begin(intent.clone(), template.steps.len());
        let engine = DecisionEngine::new(Lang::Es);
        let step = engine
            .next_step(&intent, template, &frame, &session)
            .unwrap();
        assert!(step.anchor_xy.is_none());
        assert_eq!(step.target_text, "Música");
    }
}
