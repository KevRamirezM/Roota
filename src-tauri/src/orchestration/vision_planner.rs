use serde::Deserialize;

use crate::llm::ollama::OllamaClient;
use crate::orchestration::brief::TaskBrief;
use crate::orchestration::plan::{PlanSource, TaskPlan};
use crate::orchestration::state::ActionVerb;
use crate::orchestration::templates::StepBlueprint;
use crate::perception::frame::Rect;
use crate::perception::vision::capture::{capture_window_bitmap, CaptureOptions};
use crate::perception::vision::coords::map_image_rect_to_screen;
use crate::perception::vision::moondream::rgba_to_png;
use crate::prompts;
use crate::settings::Settings;

const MAX_PLANNED_STEPS: usize = 6;

#[derive(Debug, Deserialize)]
struct VisionPlannedStepJson {
    action: String,
    target: String,
    #[serde(default)]
    x: Option<i32>,
    #[serde(default)]
    y: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct VisionPlanJson {
    #[serde(default)]
    goal_summary: Option<String>,
    #[serde(default)]
    expected_window: Option<String>,
    #[serde(default)]
    steps: Vec<VisionPlannedStepJson>,
}

/// Sends one downscaled PNG to Ollama multimodal when text plans fail.
/// Model availability is probed once at construction, never per plan cycle.
#[derive(Debug, Clone)]
pub struct VisionTaskPlanner {
    client: OllamaClient,
    available: bool,
    max_edge: u32,
    capture_scale: f32,
}

impl VisionTaskPlanner {
    pub fn new(settings: &Settings) -> Self {
        let client = OllamaClient::for_vision_planner(settings);
        let available = client.vision_model_available();
        if available {
            tracing::info!(target = "roota.vision_planner", "vision planner model ready");
        } else {
            tracing::warn!(target = "roota.vision_planner", "vision planner model missing — fallback disabled");
        }
        Self {
            client,
            available,
            max_edge: settings.perception.vision_planner_max_edge,
            capture_scale: settings.perception.capture_scale,
        }
    }

    pub fn is_available(&self) -> bool {
        self.available
    }

    /// Blocking call — the caller must wrap in `spawn_blocking`.
    /// Captures the primary window, sends a downscaled PNG to Ollama,
    /// parses the JSON plan, and maps image coords to screen space
    /// via `hint_xy`.
    pub fn plan_from_brief_blocking(
        &self,
        brief: &TaskBrief,
        primary_window_rect: Rect,
        _lang: crate::settings::Lang,
    ) -> Result<TaskPlan, String> {
        if !self.available {
            return Err("vision planner unavailable".into());
        }

        let opts = CaptureOptions {
            max_edge: self.max_edge,
            scale: self.capture_scale,
            preprocess_ocr: false,
        };

        let bitmap = capture_window_bitmap(primary_window_rect, &opts)
            .map_err(|e| format!("capture failed: {e}"))?;

        if bitmap.is_empty() {
            return Err("captured bitmap is empty".into());
        }

        let png = rgba_to_png(&bitmap).map_err(|e| format!("png encode: {e}"))?;

        let goal_summary = brief
            .object_hints
            .first()
            .cloned()
            .unwrap_or_else(|| brief.goal_summary.clone());

        let prompt =
            prompts::render_vision_task_planner(&goal_summary, bitmap.width, bitmap.height);

        tracing::debug!(
            target = "roota.vision_planner",
            w = bitmap.width,
            h = bitmap.height,
            png_kb = png.len() / 1024,
            "vision planner inference starting"
        );

        let started = std::time::Instant::now();
        let json = self
            .client
            .complete_vision_json_blocking(&prompt, &png)
            .map_err(|e| format!("ollama vision planner: {e}"))?;

        tracing::info!(
            target = "roota.vision_planner",
            ms = started.elapsed().as_millis(),
            "vision planner inference complete"
        );

        let plan = parse_vision_plan_json(&json, &bitmap);
        if plan.steps.is_empty() {
            return Err("vision planner returned empty steps".into());
        }

        Ok(plan)
    }
}

fn parse_vision_plan_json(json: &serde_json::Value, bitmap: &crate::perception::vision::capture::CapturedFrame) -> TaskPlan {
    let parsed: VisionPlanJson = match serde_json::from_value(json.clone()) {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!(
                target = "roota.vision_planner",
                "vision plan json parse failed: {err}"
            );
            return TaskPlan {
                brief: TaskBrief::empty(),
                expected_window: None,
                steps: vec![],
                source: PlanSource::Vision,
            };
        }
    };

    let steps: Vec<StepBlueprint> = parsed
        .steps
        .into_iter()
        .filter_map(|s| {
            let target = s.target.trim().to_string();
            if target.is_empty() {
                return None;
            }
            let action = parse_vision_action(&s.action);
            let hint_xy = match (s.x, s.y) {
                (Some(x), Some(y)) if x >= 0 && y >= 0 => {
                    let screen_rect = map_image_rect_to_screen(
                        x, y, 1, 1,
                        bitmap.width,
                        bitmap.height,
                        bitmap.source_rect,
                    );
                    Some((screen_rect.x + screen_rect.width / 2, screen_rect.y + screen_rect.height / 2))
                }
                _ => None,
            };
            Some(StepBlueprint {
                action,
                target_query: target,
                instruction_key: instruction_key_for(action),
                fallback_window: None,
                hint_xy,
            })
        })
        .take(MAX_PLANNED_STEPS)
        .collect();

    let brief = TaskBrief {
        goal_summary: parsed.goal_summary.unwrap_or_default(),
        app_hints: vec![],
        object_hints: vec![],
        raw_utterance: String::new(),
        risk_flags: vec![],
    };

    TaskPlan {
        brief,
        expected_window: parsed.expected_window.filter(|s| !s.is_empty()),
        steps,
        source: PlanSource::Vision,
    }
}

fn parse_vision_action(raw: &str) -> ActionVerb {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perception::vision::capture::CapturedFrame;

    fn scaled_dimensions(original_w: u32, original_h: u32, max_edge: u32) -> (u32, u32) {
        let long = original_w.max(original_h);
        if long <= max_edge || max_edge == 0 {
            return (original_w, original_h);
        }
        let ratio = max_edge as f64 / long as f64;
        let nw = ((original_w as f64) * ratio).max(1.0) as u32;
        let nh = ((original_h as f64) * ratio).max(1.0) as u32;
        (nw, nh)
    }

    #[test]
    fn scale_1920x1080_to_max_edge_768() {
        let (w, h) = scaled_dimensions(1920, 1080, 768);
        assert_eq!((w, h), (768, 432));
    }

    #[test]
    fn scale_800x600_to_max_edge_768() {
        let (w, h) = scaled_dimensions(800, 600, 768);
        assert_eq!((w, h), (768, 576));
    }

    #[test]
    fn coord_map_image_to_screen() {
        let bitmap = CapturedFrame {
            width: 768,
            height: 432,
            pixels: vec![],
            source_rect: Rect::new(100, 200, 1920, 1080),
        };
        let screen_rect = map_image_rect_to_screen(400, 300, 1, 1, bitmap.width, bitmap.height, bitmap.source_rect);
        assert!(screen_rect.x >= 100);
        assert!(screen_rect.y >= 200);
    }

    #[test]
    fn parse_vision_json_maps_hint_xy() {
        let json = serde_json::json!({
            "goal_summary": "abrir configuración",
            "steps": [
                {"action": "click", "target": "Inicio", "x": 200, "y": 300}
            ]
        });
        let bitmap = CapturedFrame {
            width: 768,
            height: 432,
            pixels: vec![],
            source_rect: Rect::new(0, 0, 1920, 1080),
        };
        let plan = parse_vision_plan_json(&json, &bitmap);
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.source, PlanSource::Vision);
        assert!(plan.steps[0].hint_xy.is_some());
    }
}
