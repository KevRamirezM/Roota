use crate::accessibility::element::UiSnapshot;

use crate::orchestration::state::{ActionVerb, GuideStep};

#[derive(Debug, Clone)]
pub struct StepCompletion {
    pub completed: bool,
    pub reason: String,
}

const ROOTA_MARKERS: &[&str] = &["roota"];

/// Minimum polls before we accept any completion signal (lets the UI render).
pub const MIN_POLLS_BEFORE_COMPLETE: u32 = 2;

#[derive(Default)]
pub struct StateDetector;

impl StateDetector {
    pub fn is_completed(
        &self,
        step: &GuideStep,
        before: &UiSnapshot,
        after: &UiSnapshot,
        poll_index: u32,
    ) -> StepCompletion {
        if poll_index < MIN_POLLS_BEFORE_COMPLETE {
            return StepCompletion {
                completed: false,
                reason: format!("warming up (poll {poll_index})"),
            };
        }

        if is_roota_snapshot(before) || is_roota_snapshot(after) {
            return StepCompletion {
                completed: false,
                reason: "snapshot still on Roota — switch to the app you are guiding".into(),
            };
        }

        if folder_navigation_completed(step, before, after) {
            return StepCompletion {
                completed: true,
                reason: "folder view opened".into(),
            };
        }

        if before.window.is_empty() || after.window.is_empty() {
            return self.target_disappeared(step, before, after);
        }

        let before_norm = normalize_title(&before.window);
        let after_norm = normalize_title(&after.window);

        if before_norm != after_norm && !before_norm.is_empty() && !after_norm.is_empty() {
            if matches!(
                step.action,
                ActionVerb::Click
                    | ActionVerb::DoubleClick
                    | ActionVerb::RightClick
                    | ActionVerb::Type
            ) {
                return StepCompletion {
                    completed: true,
                    reason: format!("window changed: {before_norm} -> {after_norm}"),
                };
            }
        }

        self.target_disappeared(step, before, after)
    }

    fn target_disappeared(
        &self,
        step: &GuideStep,
        before: &UiSnapshot,
        after: &UiSnapshot,
    ) -> StepCompletion {
        if step.target_text.is_empty() {
            return StepCompletion {
                completed: false,
                reason: "no target".into(),
            };
        }

        let target_before = before.find(&step.target_text);
        let target_after = after.find(&step.target_text);

        if target_before.is_some() && target_after.is_none() {
            return StepCompletion {
                completed: true,
                reason: format!("target {} disappeared", step.target_text),
            };
        }

        StepCompletion {
            completed: false,
            reason: "no significant change".into(),
        }
    }
}

/// Explorer: double-clicking a folder opens a window titled with that folder.
fn folder_navigation_completed(step: &GuideStep, before: &UiSnapshot, after: &UiSnapshot) -> bool {
    if !matches!(step.action, ActionVerb::DoubleClick) {
        return false;
    }
    let target = step.target_text.to_lowercase();
    if target.is_empty() {
        return false;
    }
    let after_title = after.window.to_lowercase();
    let before_title = before.window.to_lowercase();

    let aliases = folder_aliases(&target);
    let opened = aliases
        .iter()
        .any(|a| after_title.contains(a) && !before_title.contains(a));
    opened || (after_title.contains(&target) && after_title.contains("explorador") && before_title != after_title)
}

fn folder_aliases(target: &str) -> Vec<String> {
    let mut out = vec![target.to_string()];
    match target {
        "descargas" => out.push("downloads".into()),
        "downloads" => out.push("descargas".into()),
        "documentos" => out.push("documents".into()),
        "documents" => out.push("documentos".into()),
        _ => {}
    }
    out
}

fn is_roota_snapshot(snap: &UiSnapshot) -> bool {
    is_roota_title(&snap.window)
}

fn is_roota_title(title: &str) -> bool {
    let lower = title.to_lowercase();
    ROOTA_MARKERS.iter().any(|m| lower.contains(m))
}

fn normalize_title(title: &str) -> String {
    title.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accessibility::element::UiElement;

    fn snap(elements: Vec<UiElement>, window: &str) -> UiSnapshot {
        UiSnapshot {
            window: window.into(),
            elements,
        }
    }

    fn step(target: &str, action: ActionVerb) -> GuideStep {
        GuideStep {
            index: 1,
            total: 1,
            action,
            target_text: target.into(),
            instruction: "..".into(),
            anchor_xy: Some((0, 0)),
            anchor_bounds: None,
        }
    }

    #[test]
    fn target_disappeared_means_completed_after_warmup() {
        let before = snap(
            vec![UiElement {
                kind: "button".into(),
                text: "Descargas".into(),
                x: 0,
                y: 0,
                width: 10,
                height: 10,
                automation_id: None,
                window: "Explorer".into(),
            }],
            "Explorer",
        );
        let after = snap(vec![], "Explorer");
        let outcome = StateDetector.is_completed(
            &step("Descargas", ActionVerb::Click),
            &before,
            &after,
            MIN_POLLS_BEFORE_COMPLETE,
        );
        assert!(outcome.completed);
    }

    #[test]
    fn window_change_ignored_during_warmup() {
        let before = snap(vec![], "Explorer");
        let after = snap(vec![], "Descargas");
        let outcome = StateDetector.is_completed(
            &step("Descargas", ActionVerb::DoubleClick),
            &before,
            &after,
            0,
        );
        assert!(!outcome.completed);
    }

    #[test]
    fn roota_snapshots_never_complete() {
        let before = snap(vec![], "Roota");
        let after = snap(vec![], "Explorador");
        let outcome = StateDetector.is_completed(
            &step("Descargas", ActionVerb::DoubleClick),
            &before,
            &after,
            MIN_POLLS_BEFORE_COMPLETE,
        );
        assert!(!outcome.completed);
    }

    #[test]
    fn explorer_folder_title_change_completes() {
        let before = snap(vec![], "Inicio - Explorador de archivos");
        let after = snap(vec![], "Descargas - Explorador de archivos");
        let outcome = StateDetector.is_completed(
            &step("Descargas", ActionVerb::DoubleClick),
            &before,
            &after,
            MIN_POLLS_BEFORE_COMPLETE,
        );
        assert!(outcome.completed);
    }

    #[test]
    fn no_change_means_pending() {
        let s = snap(
            vec![UiElement {
                kind: "button".into(),
                text: "Descargas".into(),
                x: 0,
                y: 0,
                width: 10,
                height: 10,
                automation_id: None,
                window: "Explorer".into(),
            }],
            "Explorer",
        );
        let outcome = StateDetector.is_completed(
            &step("Descargas", ActionVerb::Click),
            &s,
            &s,
            MIN_POLLS_BEFORE_COMPLETE,
        );
        assert!(!outcome.completed);
    }
}
