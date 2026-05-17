//! PLAN phase — validated task plans from screen state.

use crate::orchestration::brief::TaskBrief;
use crate::orchestration::state::ActionVerb;
use crate::orchestration::templates::{GuidanceTemplate, StepBlueprint};
use crate::perception::ScreenFrame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanSource {
    Template,
    Llm,
    Heuristic,
    Replan,
    Vision,
}

#[derive(Debug, Clone)]
pub struct TaskPlan {
    pub brief: TaskBrief,
    pub expected_window: Option<String>,
    pub steps: Vec<StepBlueprint>,
    pub source: PlanSource,
}

impl TaskPlan {
    pub fn to_template(&self) -> GuidanceTemplate {
        GuidanceTemplate {
            intent: if self.source == PlanSource::Template {
                "open_folder".into() // caller should override when using static templates
            } else {
                "windows_task".into()
            },
            confirmation_action_key: "confirm.windows_task".into(),
            expected_window: self.expected_window.clone(),
            steps: self.steps.clone(),
        }
    }

    pub fn to_guidance_template(&self, intent_key: &str, confirm_key: &str) -> GuidanceTemplate {
        GuidanceTemplate {
            intent: intent_key.into(),
            confirmation_action_key: confirm_key.into(),
            expected_window: self.expected_window.clone(),
            steps: self.steps.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlanValidationReport {
    pub match_ratio: f32,
    pub needs_reobserve: bool,
    pub unmatched_targets: Vec<String>,
}

pub struct PlanValidator;

impl PlanValidator {
    pub fn new() -> Self {
        Self
    }

    pub fn validate(&self, plan: &TaskPlan, frame: &ScreenFrame) -> PlanValidationReport {
        if plan.steps.is_empty() {
            return PlanValidationReport {
                match_ratio: 0.0,
                needs_reobserve: true,
                unmatched_targets: vec![],
            };
        }

        let mut matched = 0usize;
        let mut unmatched = Vec::new();

        for step in &plan.steps {
            let target = step.target_query.trim();
            if target.is_empty() {
                unmatched.push(target.to_string());
                continue;
            }
            if self.step_matches(frame, target, step.action) {
                matched += 1;
            } else {
                unmatched.push(target.to_string());
            }
        }

        let match_ratio = matched as f32 / plan.steps.len() as f32;
        PlanValidationReport {
            needs_reobserve: match_ratio < 0.5,
            match_ratio,
            unmatched_targets: unmatched,
        }
    }

    fn step_matches(&self, frame: &ScreenFrame, target: &str, action: ActionVerb) -> bool {
        let queries = vec![target.to_lowercase()];
        if frame.find_best_for_action(&queries, action).is_some() {
            return true;
        }
        let lower = target.to_lowercase();
        frame.windows.iter().any(|w| {
            w.title.to_lowercase().contains(&lower)
                || lower.contains(&w.title.to_lowercase())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::brief::heuristic_brief;
    use crate::perception::{
        ElementSource, PerceptionQuality, Rect, ScreenElement, ScreenFrame, WindowId,
        WindowSnapshot,
    };

    fn fixture_frame_with_only(label: &str) -> ScreenFrame {
        ScreenFrame {
            primary_window_id: WindowId(1),
            windows: vec![WindowSnapshot {
                id: WindowId(1),
                title: "Escritorio".into(),
                class_name: String::new(),
                bounds: Rect::new(0, 0, 1920, 1080),
                is_foreground: true,
                z_order: 0,
                uia_element_count: 1,
            }],
            elements: vec![ScreenElement {
                source: ElementSource::Uia,
                text: label.into(),
                bounds: Rect::new(10, 10, 80, 24),
                window_id: WindowId(1),
                kind: "Button".into(),
                confidence: 1.0,
                automation_id: None,
            }],
            quality: PerceptionQuality::Full,
            ..ScreenFrame::empty()
        }
    }

    fn blueprint(action: ActionVerb, target: &str) -> StepBlueprint {
        StepBlueprint {
            action,
            target_query: target.into(),
            instruction_key: "guidance.click_target".into(),
            fallback_window: None,
            hint_xy: None,
        }
    }

    #[test]
    fn validator_flags_weak_plan() {
        let frame = fixture_frame_with_only("Inicio");
        let plan = TaskPlan {
            brief: heuristic_brief("config", "configuración"),
            expected_window: None,
            steps: vec![
                blueprint(ActionVerb::Click, "Configuración"),
                blueprint(ActionVerb::Click, "Red e Internet"),
            ],
            source: PlanSource::Llm,
        };
        let report = PlanValidator::new().validate(&plan, &frame);
        assert!(report.match_ratio < 0.5);
        assert!(report.needs_reobserve);
    }

    #[test]
    fn validator_accepts_matching_plan() {
        let frame = fixture_frame_with_only("Inicio");
        let plan = TaskPlan {
            brief: heuristic_brief("inicio", "Inicio"),
            expected_window: None,
            steps: vec![blueprint(ActionVerb::Click, "Inicio")],
            source: PlanSource::Llm,
        };
        let report = PlanValidator::new().validate(&plan, &frame);
        assert!(report.match_ratio >= 1.0);
        assert!(!report.needs_reobserve);
    }
}
