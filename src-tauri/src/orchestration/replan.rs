//! VERIFY / REPLAN — recover when the screen no longer matches the plan.

use std::sync::Arc;

use crate::accessibility::scanner::ScanContext;
use crate::llm::client::LlmClient;
use crate::orchestration::brief::TaskBrief;
use crate::orchestration::plan::{PlanSource, TaskPlan};
use crate::orchestration::planner::TaskPlanner;
use crate::orchestration::templates::{GuidanceTemplate, StepBlueprint};
use crate::perception::ScreenFrame;
use crate::settings::PerceptionSettings;

pub const MAX_REPLANS_PER_SESSION: u32 = 2;

#[derive(Debug, Clone)]
pub enum ReplanReason {
    TargetNotFound { step_index: usize, target: String },
    WrongClick { count: u32 },
    ScreenChanged { detail: String },
    UserAskedHelp,
}

impl ReplanReason {
    pub fn label(&self) -> &'static str {
        match self {
            ReplanReason::TargetNotFound { .. } => "target_not_found",
            ReplanReason::WrongClick { .. } => "wrong_click",
            ReplanReason::ScreenChanged { .. } => "screen_changed",
            ReplanReason::UserAskedHelp => "user_help",
        }
    }
}

pub struct ReplanEngine {
    planner: TaskPlanner,
}

impl ReplanEngine {
    pub fn new(
        llm: Arc<dyn LlmClient>,
        inference_timeout_secs: f32,
        prompt_element_cap: usize,
    ) -> Self {
        Self {
            planner: TaskPlanner::new(llm, inference_timeout_secs, prompt_element_cap),
        }
    }

    pub fn remaining_blueprints(plan: &TaskPlan, completed_step_index: usize) -> Vec<StepBlueprint> {
        plan.steps
            .iter()
            .skip(completed_step_index)
            .cloned()
            .collect()
    }

    pub async fn replan(
        &self,
        brief: &TaskBrief,
        frame: &ScreenFrame,
        scan_ctx: &ScanContext,
        perception: &PerceptionSettings,
        current_steps: &[StepBlueprint],
        from_step_index: usize,
        _reason: &ReplanReason,
    ) -> TaskPlan {
        let mut plan = self
            .planner
            .plan_from_brief(brief, frame, scan_ctx, perception)
            .await;

        if plan.steps.is_empty() && from_step_index < current_steps.len() {
            plan.steps = Self::remaining_blueprints(
                &TaskPlan {
                    brief: brief.clone(),
                    expected_window: None,
                    steps: current_steps.to_vec(),
                    source: PlanSource::Replan,
                },
                from_step_index,
            );
        }
        plan.source = PlanSource::Replan;
        plan
    }

    pub fn apply_replan(template: &mut GuidanceTemplate, plan: &TaskPlan, from_step: usize) {
        if from_step >= template.steps.len() {
            template.steps = plan.steps.clone();
        } else {
            let mut steps = template.steps[..from_step].to_vec();
            steps.extend(plan.steps.iter().cloned());
            template.steps = steps;
        }
        if plan.expected_window.is_some() {
            template.expected_window = plan.expected_window.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::brief::heuristic_brief;
    use crate::orchestration::plan::TaskPlan;
    use crate::orchestration::state::ActionVerb;
    use crate::orchestration::templates::StepBlueprint;

    fn bp(target: &str) -> StepBlueprint {
        StepBlueprint {
            action: ActionVerb::Click,
            target_query: target.into(),
            instruction_key: "guidance.click_target".into(),
            fallback_window: None,
        }
    }

    #[test]
    fn replan_skips_completed_steps() {
        let plan = TaskPlan {
            brief: heuristic_brief("x", "y"),
            expected_window: None,
            steps: vec![bp("a"), bp("b"), bp("c")],
            source: PlanSource::Llm,
        };
        let remaining = ReplanEngine::remaining_blueprints(&plan, 2);
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].target_query, "c");
    }
}
