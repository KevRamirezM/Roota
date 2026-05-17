//! Declarative step skeletons merged when LLM plans are weak.

use std::collections::HashMap;

use serde::Deserialize;

use crate::orchestration::brief::TaskBrief;
use crate::orchestration::state::ActionVerb;
use crate::orchestration::templates::StepBlueprint;
use crate::perception::ScreenFrame;

type RecipeFile = HashMap<String, Vec<RecipeStep>>;

#[derive(Debug, Deserialize)]
struct RecipeStep {
    action: String,
    target_query: String,
}

pub struct RecipeRegistry {
    by_app: HashMap<String, HashMap<String, Vec<StepBlueprint>>>,
}

impl RecipeRegistry {
    pub fn load_embedded() -> Self {
        let mut by_app = HashMap::new();
        for (app, raw) in [
            ("chrome", include_str!("../../guidance/recipes/chrome.json")),
            ("explorer", include_str!("../../guidance/recipes/explorer.json")),
            ("settings", include_str!("../../guidance/recipes/settings.json")),
        ] {
            if let Ok(file) = serde_json::from_str::<RecipeFile>(raw) {
                let inner: HashMap<String, Vec<StepBlueprint>> = file
                    .into_iter()
                    .map(|(name, steps)| {
                        (
                            name,
                            steps.into_iter().map(recipe_to_blueprint).collect(),
                        )
                    })
                    .collect();
                by_app.insert(app.to_string(), inner);
            }
        }
        Self { by_app }
    }

    pub fn skeleton(&self, app: &str, recipe: &str) -> Vec<StepBlueprint> {
        self.by_app
            .get(&app.to_lowercase())
            .and_then(|m| m.get(recipe))
            .cloned()
            .unwrap_or_default()
    }

    /// Pick a recipe from brief hints and utterance keywords; refine targets from frame.
    pub fn suggest_steps(&self, brief: &TaskBrief, frame: Option<&ScreenFrame>) -> Vec<StepBlueprint> {
        let u = brief.raw_utterance.to_lowercase();
        let recipe_key = if u.contains("pestaña") || u.contains("tab") {
            Some(("chrome", "new_tab"))
        } else if u.contains("descargas") || u.contains("downloads") {
            Some(("explorer", "open_downloads"))
        } else if u.contains("wifi") || u.contains("wi-fi") || u.contains("wi fi") {
            Some(("settings", "wifi"))
        } else if u.contains("bluetooth") {
            Some(("settings", "bluetooth"))
        } else if u.contains("configuración") || u.contains("configuracion") || u.contains("settings")
        {
            Some(("settings", "open_settings"))
        } else {
            None
        };

        let Some((app, key)) = recipe_key else {
            return Vec::new();
        };

        let mut steps = self.skeleton(app, key);
        if let Some(frame) = frame {
            refine_targets_from_frame(&mut steps, frame);
        }
        steps
    }
}

fn recipe_to_blueprint(s: RecipeStep) -> StepBlueprint {
    let action = parse_recipe_action(&s.action);
    StepBlueprint {
        action,
        target_query: s.target_query,
        instruction_key: instruction_key_for(action),
        fallback_window: None,
    }
}

fn parse_recipe_action(raw: &str) -> ActionVerb {
    match raw.trim().to_lowercase().as_str() {
        "double_click" => ActionVerb::DoubleClick,
        "right_click" => ActionVerb::RightClick,
        "type" => ActionVerb::Type,
        "locate" => ActionVerb::Locate,
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

fn refine_targets_from_frame(steps: &mut [StepBlueprint], frame: &ScreenFrame) {
    for step in steps.iter_mut() {
        let q = vec![step.target_query.to_lowercase()];
        if let Some(el) = frame.find_best_for_action(&q, step.action) {
            if !el.text.trim().is_empty() {
                step.target_query = el.text.trim().to_string();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipes_load_chrome_new_tab() {
        let reg = RecipeRegistry::load_embedded();
        let steps = reg.skeleton("chrome", "new_tab");
        assert!(!steps.is_empty());
    }
}
