use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use crate::orchestration::state::ActionVerb;

#[derive(Debug, Clone)]
pub struct StepBlueprint {
    pub action: ActionVerb,
    pub target_query: String,
    pub instruction_key: String,
    pub fallback_window: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GuidanceTemplate {
    pub intent: String,
    pub confirmation_action_key: String,
    pub steps: Vec<StepBlueprint>,
    pub expected_window: Option<String>,
}

#[derive(Debug, Default)]
pub struct TemplateRegistry {
    by_intent: HashMap<String, GuidanceTemplate>,
}

impl TemplateRegistry {
    pub fn register(&mut self, template: GuidanceTemplate) {
        self.by_intent.insert(template.intent.clone(), template);
    }

    pub fn get(&self, intent: &str) -> Option<&GuidanceTemplate> {
        self.by_intent.get(intent)
    }

    pub fn known_intents(&self) -> Vec<String> {
        let mut out: Vec<String> = self.by_intent.keys().cloned().collect();
        out.sort();
        out
    }

    pub fn merge_json_dir(&mut self, root: &Path) {
        if !root.exists() {
            return;
        }
        let entries = match std::fs::read_dir(root) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let raw = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let parsed: HashMap<String, JsonTemplate> = match serde_json::from_str(&raw) {
                Ok(p) => p,
                Err(err) => {
                    tracing::warn!(target: "roota.templates", "skip {:?}: {err}", path);
                    continue;
                }
            };
            for (intent_name, body) in parsed {
                let steps = body
                    .steps
                    .into_iter()
                    .map(|s| StepBlueprint {
                        action: parse_verb(&s.action),
                        target_query: s.target_query.unwrap_or_else(|| "{target}".into()),
                        instruction_key: s
                            .instruction_key
                            .unwrap_or_else(|| "guidance.click_target".into()),
                        fallback_window: s.fallback_window,
                    })
                    .collect();
                self.register(GuidanceTemplate {
                    intent: intent_name.clone(),
                    confirmation_action_key: body
                        .confirmation_action_key
                        .unwrap_or_else(|| format!("confirm.{intent_name}")),
                    steps,
                    expected_window: body.expected_window,
                });
            }
        }
    }
}

#[derive(Deserialize)]
struct JsonTemplate {
    confirmation_action_key: Option<String>,
    expected_window: Option<String>,
    steps: Vec<JsonStep>,
}

#[derive(Deserialize)]
struct JsonStep {
    action: String,
    target_query: Option<String>,
    instruction_key: Option<String>,
    fallback_window: Option<String>,
}

fn parse_verb(value: &str) -> ActionVerb {
    match value {
        "click" => ActionVerb::Click,
        "double_click" => ActionVerb::DoubleClick,
        "right_click" => ActionVerb::RightClick,
        "type" => ActionVerb::Type,
        _ => ActionVerb::Locate,
    }
}

pub fn default_registry() -> TemplateRegistry {
    let mut r = TemplateRegistry::default();
    let s = |action: ActionVerb, target: &str, key: &str| StepBlueprint {
        action,
        target_query: target.into(),
        instruction_key: key.into(),
        fallback_window: None,
    };
    r.register(GuidanceTemplate {
        intent: "open_folder".into(),
        confirmation_action_key: "confirm.open_folder".into(),
        expected_window: Some("Explorer".into()),
        steps: vec![s(
            ActionVerb::DoubleClick,
            "{target}",
            "guidance.double_click_target",
        )],
    });
    r.register(GuidanceTemplate {
        intent: "move_file".into(),
        confirmation_action_key: "confirm.move_file".into(),
        expected_window: Some("Explorer".into()),
        steps: vec![
            s(ActionVerb::Locate, "{target}", "guidance.locate_target"),
            s(
                ActionVerb::RightClick,
                "{target}",
                "guidance.right_click_target",
            ),
            s(ActionVerb::Click, "Cortar", "guidance.click_target"),
        ],
    });
    r.register(GuidanceTemplate {
        intent: "delete_file".into(),
        confirmation_action_key: "confirm.delete_file".into(),
        expected_window: Some("Explorer".into()),
        steps: vec![
            s(ActionVerb::Click, "{target}", "guidance.click_target"),
            s(ActionVerb::Locate, "Suprimir", "guidance.locate_target"),
        ],
    });
    r.register(GuidanceTemplate {
        intent: "open_browser".into(),
        confirmation_action_key: "confirm.open_browser".into(),
        expected_window: Some("Chrome".into()),
        steps: vec![s(ActionVerb::Locate, "Chrome", "guidance.locate_target")],
    });
    r.register(GuidanceTemplate {
        intent: "search_web".into(),
        confirmation_action_key: "confirm.search_web".into(),
        expected_window: Some("Chrome".into()),
        steps: vec![
            s(ActionVerb::Click, "Nueva pestaña", "guidance.click_target"),
            s(ActionVerb::Type, "Buscar", "guidance.type_in_target"),
        ],
    });
    r.register(GuidanceTemplate {
        intent: "open_url".into(),
        confirmation_action_key: "confirm.open_url".into(),
        expected_window: Some("Chrome".into()),
        steps: vec![
            s(ActionVerb::Click, "Nueva pestaña", "guidance.click_target"),
            s(ActionVerb::Type, "Buscar", "guidance.type_in_target"),
        ],
    });
    r.register(GuidanceTemplate {
        intent: "compose_email".into(),
        confirmation_action_key: "confirm.compose_email".into(),
        expected_window: Some("Gmail".into()),
        steps: vec![s(ActionVerb::Click, "Redactar", "guidance.click_target")],
    });
    r.register(GuidanceTemplate {
        intent: "read_inbox".into(),
        confirmation_action_key: "confirm.read_inbox".into(),
        expected_window: Some("Gmail".into()),
        steps: vec![s(
            ActionVerb::Click,
            "Bandeja de entrada",
            "guidance.click_target",
        )],
    });
    r.register(GuidanceTemplate {
        intent: "reply_message".into(),
        confirmation_action_key: "confirm.reply_message".into(),
        expected_window: Some("Gmail".into()),
        steps: vec![s(ActionVerb::Click, "Responder", "guidance.click_target")],
    });
    r.register(GuidanceTemplate {
        intent: "open_word_document".into(),
        confirmation_action_key: "confirm.open_word_document".into(),
        expected_window: Some("Word".into()),
        steps: vec![s(ActionVerb::Locate, "Word", "guidance.locate_target")],
    });
    r.register(GuidanceTemplate {
        intent: "print_document".into(),
        confirmation_action_key: "confirm.print_document".into(),
        expected_window: Some("Word".into()),
        steps: vec![s(ActionVerb::Click, "Imprimir", "guidance.click_target")],
    });
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_cover_phase3_intents() {
        let r = default_registry();
        let known = r.known_intents();
        for needed in [
            "open_folder",
            "move_file",
            "delete_file",
            "open_browser",
            "search_web",
            "open_url",
            "compose_email",
            "read_inbox",
            "reply_message",
            "open_word_document",
            "print_document",
        ] {
            assert!(
                known.contains(&needed.to_string()),
                "missing template for {needed}"
            );
        }
    }
}
