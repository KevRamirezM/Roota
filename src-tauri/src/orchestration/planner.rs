//! LLM task planner — turns any Windows utterance + live screen into step blueprints.
//! Uses the same text model as intent classification (e.g. qwen3:1.7b); vision stays in perception.

use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;

use crate::accessibility::scanner::ScanContext;
use crate::llm::client::LlmClient;
use crate::orchestration::brief::TaskBrief;
use crate::orchestration::plan::{PlanSource, TaskPlan};
use crate::orchestration::recipes::RecipeRegistry;
use crate::orchestration::state::ActionVerb;
use crate::orchestration::templates::{GuidanceTemplate, StepBlueprint};
use crate::perception::ScreenFrame;
use crate::prompts;
use crate::settings::PerceptionSettings;

const PLANNER_TIMEOUT_SECS: f32 = 28.0;
const MAX_PLANNED_STEPS: usize = 6;
/// Slightly larger cap for planning so the model sees menus the guide loop may omit.
const PLANNER_PROMPT_ELEMENTS: usize = 60;

#[derive(Debug, Deserialize)]
struct PlannedStepJson {
    action: String,
    target: String,
}

#[derive(Debug, Deserialize)]
struct PlanJson {
    goal_summary: Option<String>,
    expected_window: Option<String>,
    steps: Vec<PlannedStepJson>,
}

pub struct TaskPlanner {
    llm: Arc<dyn LlmClient>,
}

impl TaskPlanner {
    pub fn new(llm: Arc<dyn LlmClient>) -> Self {
        Self { llm }
    }

    /// Build a full guidance template from what is on screen right now.
    pub async fn plan(
        &self,
        utterance: &str,
        goal_target: &str,
        frame: &ScreenFrame,
        scan_ctx: &ScanContext,
        perception: &PerceptionSettings,
    ) -> GuidanceTemplate {
        let brief = crate::orchestration::brief::heuristic_brief(utterance, goal_target);
        self.plan_from_brief(&brief, frame, scan_ctx, perception)
            .await
            .to_guidance_template("windows_task", "confirm.windows_task")
    }

    /// PLAN phase — screen-grounded step list with recipe fallback.
    pub async fn plan_from_brief(
        &self,
        brief: &TaskBrief,
        frame: &ScreenFrame,
        scan_ctx: &ScanContext,
        perception: &PerceptionSettings,
    ) -> TaskPlan {
        let goal_target = brief
            .object_hints
            .first()
            .cloned()
            .unwrap_or_else(|| brief.goal_summary.clone());

        let mut hints = scan_ctx.window_hints.clone();
        hints.extend(brief.app_hints.clone());
        hints.extend(brief.object_hints.clone());

        let element_limit = perception
            .prompt_max_elements
            .max(PLANNER_PROMPT_ELEMENTS);
        let visible = frame.ranked_visible_summary_for_target(
            element_limit,
            &hints,
            frame.cursor,
            &goal_target,
        );
        let window_list = frame.window_list_for_prompt(perception.prompt_max_windows);
        let brief_block = format!(
            "Resumen: {}\nApps: {}\nObjetos: {}",
            brief.goal_summary,
            brief.app_hints.join(", "),
            brief.object_hints.join(", ")
        );
        let prompt = prompts::render_task_planner(prompts::TaskPlannerContext {
            utterance: &brief.raw_utterance,
            goal_target: &goal_target,
            task_brief_block: &brief_block,
            window_list: &window_list,
            visible_elements: &visible,
        });

        let timeout = Duration::from_secs_f32(PLANNER_TIMEOUT_SECS);
        let llm_fut = self
            .llm
            .complete_json(&prompt, Some(prompts::SYSTEM_PROMPT));
        let template = match tokio::time::timeout(timeout, llm_fut).await {
            Ok(Ok(v)) => parse_plan_json(v, &goal_target)
                .unwrap_or_else(|| heuristic_plan(&brief.raw_utterance, &goal_target, Some(frame))),
            Ok(Err(err)) => {
                tracing::warn!(target: "roota.planner", "LLM plan failed: {err}");
                heuristic_plan(&brief.raw_utterance, &goal_target, Some(frame))
            }
            Err(_) => {
                tracing::warn!(target: "roota.planner", "LLM plan timed out");
                heuristic_plan(&brief.raw_utterance, &goal_target, Some(frame))
            }
        };

        let mut steps = template.steps;
        if steps.is_empty() {
            let recipes = RecipeRegistry::load_embedded();
            steps = recipes.suggest_steps(brief, Some(frame));
        }

        TaskPlan {
            brief: brief.clone(),
            expected_window: template.expected_window,
            steps,
            source: PlanSource::Llm,
        }
    }
}

pub fn parse_plan_json(value: serde_json::Value, goal_target: &str) -> Option<GuidanceTemplate> {
    let parsed: PlanJson = serde_json::from_value(value).ok()?;
    let steps: Vec<StepBlueprint> = parsed
        .steps
        .into_iter()
        .filter_map(|s| {
            let target = s.target.trim();
            if target.is_empty() {
                return None;
            }
            let action = parse_action(&s.action);
            Some(StepBlueprint {
                action,
                target_query: target.into(),
                instruction_key: instruction_key_for(action),
                fallback_window: None,
            })
        })
        .take(MAX_PLANNED_STEPS)
        .collect();

    if steps.is_empty() {
        return None;
    }

    let _summary = parsed
        .goal_summary
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| goal_target.to_string());
    let expected = parsed
        .expected_window
        .filter(|s| !s.trim().is_empty());

    Some(GuidanceTemplate {
        intent: "windows_task".into(),
        confirmation_action_key: "confirm.windows_task".into(),
        expected_window: expected,
        steps,
    })
}

fn parse_action(raw: &str) -> ActionVerb {
    match raw.trim().to_lowercase().as_str() {
        "double_click" | "doubleclick" | "doble_clic" => ActionVerb::DoubleClick,
        "right_click" | "rightclick" | "clic_derecho" => ActionVerb::RightClick,
        "type" | "escribir" | "typing" => ActionVerb::Type,
        "locate" | "buscar" | "find" => ActionVerb::Locate,
        _ => ActionVerb::Click,
    }
}

fn instruction_key_for(action: ActionVerb) -> String {
    match action {
        ActionVerb::Click => "guidance.click_target",
        ActionVerb::DoubleClick => "guidance.double_click_target",
        ActionVerb::RightClick => "guidance.right_click_target",
        ActionVerb::Type => "guidance.type_in_target",
        ActionVerb::Locate => "guidance.locate_target",
    }
    .into()
}

fn blueprint(action: ActionVerb, target: &str) -> StepBlueprint {
    StepBlueprint {
        action,
        target_query: target.into(),
        instruction_key: instruction_key_for(action),
        fallback_window: None,
    }
}

/// Pick the best on-screen label matching any keyword (short labels preferred).
fn pick_visible_label(frame: &ScreenFrame, keywords: &[&str]) -> Option<String> {
    let mut best: Option<(usize, String)> = None;
    for el in &frame.elements {
        let text = el.text.trim();
        if text.is_empty() || text.chars().count() > 48 {
            continue;
        }
        let lower = text.to_lowercase();
        let hit = keywords.iter().any(|k| lower.contains(k));
        if !hit {
            continue;
        }
        let len = text.chars().count();
        let better = best.as_ref().map(|(l, _)| len < *l).unwrap_or(true);
        if better {
            best = Some((len, text.to_string()));
        }
    }
    best.map(|(_, t)| t)
}

/// Screen-aware fallback when the LLM is offline, times out, or returns garbage.
pub fn heuristic_plan(
    utterance: &str,
    goal_target: &str,
    frame: Option<&ScreenFrame>,
) -> GuidanceTemplate {
    let lower = utterance.to_lowercase();
    let goal = goal_target.trim();

    if let Some(frame) = frame {
        if lower.contains("terminal") || lower.contains("consola") || lower.contains("powershell")
        {
            let mut steps = Vec::new();
            if let Some(term) = pick_visible_label(
                frame,
                &[
                    "terminal",
                    "nueva terminal",
                    "new terminal",
                    "consola",
                    "powershell",
                ],
            ) {
                steps.push(blueprint(ActionVerb::Click, &term));
            } else if let Some(menu) =
                pick_visible_label(frame, &["terminal", "ver", "view", "más"])
            {
                steps.push(blueprint(ActionVerb::Click, &menu));
                steps.push(blueprint(ActionVerb::Click, "Nueva terminal"));
            } else {
                steps.push(blueprint(ActionVerb::Click, "Terminal"));
                steps.push(blueprint(ActionVerb::Click, "Nueva terminal"));
            }
            if !steps.is_empty() {
                return GuidanceTemplate {
                    intent: "windows_task".into(),
                    confirmation_action_key: "confirm.windows_task".into(),
                    expected_window: Some("Cursor".into()),
                    steps,
                };
            }
        }

        if lower.contains("configuración")
            || lower.contains("configuracion")
            || lower.contains("settings")
        {
            let mut steps = Vec::new();
            if let Some(win) = pick_visible_label(frame, &["configuración", "settings"]) {
                steps.push(blueprint(ActionVerb::Click, &win));
            } else if let Some(start) = pick_visible_label(frame, &["inicio", "start"]) {
                steps.push(blueprint(ActionVerb::Click, &start));
                steps.push(blueprint(ActionVerb::Click, "Configuración"));
            } else {
                steps.push(blueprint(ActionVerb::Locate, "Inicio"));
                steps.push(blueprint(ActionVerb::Click, "Configuración"));
            }
            return GuidanceTemplate {
                intent: "windows_task".into(),
                confirmation_action_key: "confirm.windows_task".into(),
                expected_window: None,
                steps,
            };
        }

        if !goal.is_empty() {
            if let Some(label) = pick_visible_label(frame, &[&goal.to_lowercase()]) {
                let action = if lower.contains("doble") {
                    ActionVerb::DoubleClick
                } else if lower.contains("escrib") {
                    ActionVerb::Type
                } else if lower.contains("abrir") || lower.contains("carpeta") {
                    ActionVerb::DoubleClick
                } else {
                    ActionVerb::Click
                };
                return GuidanceTemplate {
                    intent: "windows_task".into(),
                    confirmation_action_key: "confirm.windows_task".into(),
                    expected_window: {
                        let t = frame.primary_window_title();
                        if t.is_empty() { None } else { Some(t) }
                    },
                    steps: vec![blueprint(action, &label)],
                };
            }
        }
    }

    let target = if !goal.is_empty() {
        goal.to_string()
    } else {
        utterance.trim().chars().take(48).collect::<String>()
    };
    let action = if lower.contains("doble") {
        ActionVerb::DoubleClick
    } else if lower.contains("escrib") || lower.contains("buscar en") {
        ActionVerb::Type
    } else if lower.contains("abrir") || lower.contains("carpeta") {
        ActionVerb::DoubleClick
    } else {
        ActionVerb::Locate
    };
    GuidanceTemplate {
        intent: "windows_task".into(),
        confirmation_action_key: "confirm.windows_task".into(),
        expected_window: None,
        steps: vec![blueprint(action, &target)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perception::{
        ElementSource, PerceptionQuality, Rect, ScreenElement, ScreenFrame, WindowId,
        WindowSnapshot,
    };

    #[test]
    fn parse_valid_plan_json() {
        let v = serde_json::json!({
            "goal_summary": "abrir configuración",
            "expected_window": "Inicio",
            "steps": [
                {"action": "click", "target": "Inicio"},
                {"action": "click", "target": "Configuración"}
            ]
        });
        let t = parse_plan_json(v, "configuración").unwrap();
        assert_eq!(t.steps.len(), 2);
        assert_eq!(t.steps[0].target_query, "Inicio");
        assert_eq!(t.steps[1].action, ActionVerb::Click);
    }

    #[test]
    fn heuristic_plan_non_empty() {
        let t = heuristic_plan("Abre la configuración de Windows", "configuración", None);
        assert!(!t.steps.is_empty());
    }

    #[test]
    fn heuristic_terminal_uses_visible_label() {
        let frame = ScreenFrame {
            primary_window_id: WindowId(1),
            windows: vec![WindowSnapshot {
                id: WindowId(1),
                title: "Cursor".into(),
                class_name: String::new(),
                bounds: Rect::new(0, 0, 1280, 800),
                is_foreground: true,
                z_order: 0,
                uia_element_count: 2,
            }],
            elements: vec![
                ScreenElement {
                    source: ElementSource::Uia,
                    text: "Terminal".into(),
                    bounds: Rect::new(10, 10, 80, 24),
                    window_id: WindowId(1),
                    kind: "MenuItem".into(),
                    confidence: 1.0,
                    automation_id: None,
                },
                ScreenElement {
                    source: ElementSource::Uia,
                    text: "Nueva terminal".into(),
                    bounds: Rect::new(10, 40, 120, 24),
                    window_id: WindowId(1),
                    kind: "MenuItem".into(),
                    confidence: 1.0,
                    automation_id: None,
                },
            ],
            quality: PerceptionQuality::Full,
            ..ScreenFrame::empty()
        };
        let t = heuristic_plan(
            "como abro una terminal en cursor",
            "Terminal",
            Some(&frame),
        );
        assert!(t.steps.len() >= 1);
        assert!(
            t.steps
                .iter()
                .any(|s| s.target_query.to_lowercase().contains("terminal"))
        );
    }
}
