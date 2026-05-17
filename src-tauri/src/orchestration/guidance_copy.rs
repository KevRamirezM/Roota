//! Canonical step instructions and LLM prompt facts — one contract for overlay + panel.

use crate::i18n;
use crate::settings::Lang;
use crate::orchestration::state::{ActionVerb, GuideStep};
use crate::orchestration::templates::GuidanceTemplate;
use crate::perception::{ScreenFrame, ScreenElement};

/// Human-readable goal for prompts (from confirmation copy, not intent id).
pub fn goal_summary(lang: Lang, template: &GuidanceTemplate, target: &str) -> String {
    i18n::t(
        &template.confirmation_action_key,
        lang,
        &[("target", target)],
    )
}

/// Short gesture label shown on the overlay pill — must match LLM contract.
pub fn click_hint(lang: Lang, action: ActionVerb) -> String {
    let key = match action {
        ActionVerb::Click => "guidance.hint.click",
        ActionVerb::DoubleClick => "guidance.hint.double_click",
        ActionVerb::RightClick => "guidance.hint.right_click",
        ActionVerb::Type => "guidance.hint.type",
        ActionVerb::Locate => "guidance.hint.locate",
    };
    i18n::t(key, lang, &[])
}

/// Color/resaltado cue aligned with overlay `ACTION_COLORS`.
pub fn overlay_cue(lang: Lang, action: ActionVerb) -> String {
    let key = match action {
        ActionVerb::Click => "guidance.overlay_cue.click",
        ActionVerb::DoubleClick => "guidance.overlay_cue.double_click",
        ActionVerb::RightClick => "guidance.overlay_cue.right_click",
        ActionVerb::Type => "guidance.overlay_cue.type",
        ActionVerb::Locate => "guidance.overlay_cue.locate",
    };
    i18n::t(key, lang, &[])
}

/// Deterministic instruction when we have an anchor (or as LLM fallback).
pub fn canonical_instruction(lang: Lang, step: &GuideStep, has_anchor: bool) -> String {
    if !has_anchor {
        return i18n::t(
            "guidance.hud_no_target",
            lang,
            &[("target", &step.target_text)],
        );
    }
    let key = match step.action {
        ActionVerb::Click => "guidance.instruction.click_with_anchor",
        ActionVerb::DoubleClick => "guidance.instruction.double_click_with_anchor",
        ActionVerb::RightClick => "guidance.instruction.right_click_with_anchor",
        ActionVerb::Type => "guidance.instruction.type_with_anchor",
        ActionVerb::Locate => "guidance.instruction.locate_with_anchor",
    };
    i18n::t(
        key,
        lang,
        &[
            ("target", &step.target_text),
            ("cue", &overlay_cue(lang, step.action)),
        ],
    )
}

/// Where the target sits relative to the active window (for the LLM only).
pub fn spatial_hint(frame: &ScreenFrame, element: Option<&ScreenElement>) -> String {
    let Some(el) = element else {
        return String::new();
    };
    let window = frame
        .window_title(el.window_id)
        .unwrap_or("la ventana activa");
    let (cx, cy) = el.center();
    let (wx, wy) = frame
        .primary_window()
        .map(|w| w.bounds.center())
        .unwrap_or((0, 0));
    let vert = if cy + 40 < wy {
        "parte superior"
    } else if cy > wy + 40 {
        "parte inferior"
    } else {
        "centro"
    };
    let horiz = if cx + 60 < wx {
        "izquierda"
    } else if cx > wx + 60 {
        "derecha"
    } else {
        "centro"
    };
    format!("«{}» en {window}, hacia la {horiz} y {vert} (@{cx},{cy})", el.text)
}

/// True when the instruction mentions the target or a known synonym.
fn target_matches_instruction(line: &str, target: &str) -> bool {
    let lower = line.to_lowercase();
    let t = target.to_lowercase();
    if lower.contains(&t) {
        return true;
    }
    for alias in instruction_target_aliases(&t) {
        if lower.contains(alias) {
            return true;
        }
    }
  // Any significant token from the target (e.g. "Nueva terminal" → "terminal").
    for token in t.split_whitespace() {
        if token.chars().count() >= 4 && lower.contains(token) {
            return true;
        }
    }
    false
}

fn instruction_target_aliases(target: &str) -> &'static [&'static str] {
    match target {
        "terminal" => &["terminal", "consola", "powershell"],
        "nueva terminal" => &["terminal", "nueva terminal", "new terminal"],
        "configuración" | "configuracion" => &["configuración", "configuracion", "settings"],
        "cursor" => &["cursor"],
        _ => &[],
    }
}

/// Accept LLM text only if it matches the step contract.
pub fn accept_llm_instruction(text: &str, step: &GuideStep, click_hint: &str) -> Option<String> {
    let line = text.trim().lines().next()?.trim();
    if line.len() < 12 || line.len() > 220 {
        return None;
    }
    let target = step.target_text.trim();
    if !target.is_empty() && !target_matches_instruction(line, target) {
        return None;
    }
    // Reject if model outputs meta/explanations.
    let bad = ["{", "}", "como asistente", "json", "nota:", "elementos visibles"];
    if bad.iter().any(|b| line.to_lowercase().contains(b)) {
        return None;
    }
    // Gesture should align with overlay hint or action label fragments.
    let hint = click_hint.to_lowercase();
    let gesture_ok = hint
        .split_whitespace()
        .filter(|w| w.len() > 3)
        .any(|w| line.to_lowercase().contains(w))
        || match step.action {
            ActionVerb::DoubleClick => {
                line.to_lowercase().contains("doble")
            }
            ActionVerb::RightClick => line.to_lowercase().contains("derech"),
            ActionVerb::Type => {
                line.to_lowercase().contains("escrib")
            }
            ActionVerb::Locate => {
                line.to_lowercase().contains("busca")
            }
            ActionVerb::Click => {
                line.to_lowercase().contains("clic") || line.to_lowercase().contains("pulsa")
            }
        };
    if !gesture_ok {
        return None;
    }
    Some(line.to_string())
}

/// Build ranked element list with the anchored target marked.
pub fn visible_elements_for_prompt(
    frame: &ScreenFrame,
    limit: usize,
    hints: &[String],
    cursor: crate::input::PhysicalPoint,
    target_query: &str,
) -> String {
    frame.ranked_visible_summary_for_target(limit, hints, cursor, target_query)
}

/// Resolve the element that matches the current step target (for spatial hint).
pub fn target_element<'a>(frame: &'a ScreenFrame, target_text: &str) -> Option<&'a ScreenElement> {
    if target_text.is_empty() {
        return None;
    }
    frame.find(target_text).or_else(|| {
        frame
            .elements
            .iter()
            .find(|e| e.text.eq_ignore_ascii_case(target_text))
    })
}

pub fn action_verb_label(action: ActionVerb, lang: Lang) -> String {
    let key = match action {
        ActionVerb::Click => "action.click",
        ActionVerb::DoubleClick => "action.double_click",
        ActionVerb::RightClick => "action.right_click",
        ActionVerb::Type => "action.type",
        ActionVerb::Locate => "action.locate",
    };
    i18n::t(key, lang, &[])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::templates::default_registry;
    use crate::perception::{
        ElementSource, PerceptionQuality, Rect, ScreenElement, ScreenFrame, WindowId,
        WindowSnapshot,
    };

    fn step(action: ActionVerb, target: &str) -> GuideStep {
        GuideStep {
            index: 1,
            total: 2,
            action,
            target_text: target.into(),
            instruction: String::new(),
            anchor_xy: Some((100, 100)),
            anchor_bounds: Some((80, 80, 40, 40)),
        }
    }

    #[test]
    fn canonical_mentions_target_and_cue() {
        let s = step(ActionVerb::DoubleClick, "Descargas");
        let text = canonical_instruction(Lang::Es, &s, true);
        assert!(text.contains("Descargas"));
        assert!(text.to_lowercase().contains("doble"));
    }

    #[test]
    fn accept_llm_requires_target_and_gesture() {
        let s = step(ActionVerb::Click, "Descargas");
        let hint = click_hint(Lang::Es, ActionVerb::Click);
        assert!(accept_llm_instruction(
            "Haz clic en Descargas; mira el círculo amarillo.",
            &s,
            &hint
        )
        .is_some());
        assert!(accept_llm_instruction("Pulsa aquí", &s, &hint).is_none());
    }

    #[test]
    fn accept_llm_allows_terminal_synonym() {
        let s = step(ActionVerb::Click, "Terminal");
        let hint = click_hint(Lang::Es, ActionVerb::Click);
        assert!(accept_llm_instruction(
            "Haz clic en la consola integrada; mira el círculo amarillo.",
            &s,
            &hint
        )
        .is_some());
    }

    #[test]
    fn goal_summary_uses_confirmation_not_intent_id() {
        let registry = default_registry();
        let t = registry.get("open_folder").unwrap();
        let g = goal_summary(Lang::Es, t, "Descargas");
        assert!(g.contains("Descargas"));
        assert!(!g.contains("open_folder"));
    }

    #[test]
    fn marked_summary_flags_target() {
        let frame = ScreenFrame {
            primary_window_id: WindowId(1),
            windows: vec![WindowSnapshot {
                id: WindowId(1),
                title: "Explorer".into(),
                class_name: String::new(),
                bounds: Rect::new(0, 0, 800, 600),
                is_foreground: true,
                z_order: 0,
                uia_element_count: 1,
            }],
            elements: vec![ScreenElement {
                source: ElementSource::Uia,
                text: "Descargas".into(),
                bounds: Rect::new(100, 200, 120, 32),
                window_id: WindowId(1),
                kind: "Button".into(),
                confidence: 1.0,
                automation_id: None,
            }],
            quality: PerceptionQuality::Full,
            ..ScreenFrame::empty()
        };
        let out = visible_elements_for_prompt(
            &frame,
            10,
            &["descargas".into()],
            crate::input::PhysicalPoint::default(),
            "Descargas",
        );
        assert!(out.contains("OBJETIVO"));
        assert!(out.contains("Descargas"));
    }
}
