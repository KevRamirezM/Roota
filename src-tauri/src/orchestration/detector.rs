use crate::orchestration::state::{ActionVerb, GuideStep};
use crate::perception::ScreenFrame;

#[derive(Debug, Clone)]
pub struct StepCompletion {
    pub completed: bool,
    pub reason: String,
}

const ROOTA_MARKERS: &[&str] = &["roota"];

/// Minimum polls before UIA-based completion (user clicks bypass this).
pub const MIN_POLLS_BEFORE_COMPLETE: u32 = 1;

#[derive(Default)]
pub struct StateDetector;

impl StateDetector {
    pub fn is_completed(
        &self,
        step: &GuideStep,
        before: &ScreenFrame,
        after: &ScreenFrame,
        poll_index: u32,
    ) -> StepCompletion {
        if poll_index < MIN_POLLS_BEFORE_COMPLETE {
            return StepCompletion {
                completed: false,
                reason: format!("warming up (poll {poll_index})"),
            };
        }

        // Only block when the *foreground* app is still Roota (not every window list entry).
        if is_roota_title(&after.primary_window_title()) {
            return StepCompletion {
                completed: false,
                reason: "focus is on Roota — click the app you are learning to use".into(),
            };
        }

        if folder_navigation_completed(step, before, after) {
            return StepCompletion {
                completed: true,
                reason: "folder view opened".into(),
            };
        }

        let before_title = before.primary_window_title();
        let after_title = after.primary_window_title();

        if before_title.is_empty() || after_title.is_empty() {
            return self.target_disappeared(step, before, after);
        }

        let before_norm = normalize_title(&before_title);
        let after_norm = normalize_title(&after_title);

        if before_norm != after_norm
            && !before_norm.is_empty()
            && !after_norm.is_empty()
            && matches!(
                step.action,
                ActionVerb::Click
                    | ActionVerb::DoubleClick
                    | ActionVerb::RightClick
                    | ActionVerb::Type
            )
        {
            return StepCompletion {
                completed: true,
                reason: format!("window changed: {before_norm} -> {after_norm}"),
            };
        }

        // Multi-window: also signal "completed" if a NEW window appeared whose
        // title contains the target. This catches e.g. opening "Descargas"
        // when the Explorer window stays primary but a new top-level child
        // appears.
        if let Some(reason) = new_window_matches_target(step, before, after) {
            return StepCompletion {
                completed: true,
                reason,
            };
        }

        self.target_disappeared(step, before, after)
    }

    fn target_disappeared(
        &self,
        step: &GuideStep,
        before: &ScreenFrame,
        after: &ScreenFrame,
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
fn folder_navigation_completed(
    step: &GuideStep,
    before: &ScreenFrame,
    after: &ScreenFrame,
) -> bool {
    if !matches!(step.action, ActionVerb::DoubleClick) {
        return false;
    }
    let target = step.target_text.to_lowercase();
    if target.is_empty() {
        return false;
    }
    let after_title = after.primary_window_title().to_lowercase();
    let before_title = before.primary_window_title().to_lowercase();

    let aliases = folder_aliases(&target);
    let opened = aliases
        .iter()
        .any(|a| after_title.contains(a) && !before_title.contains(a));
    opened
        || (after_title.contains(&target)
            && after_title.contains("explorador")
            && before_title != after_title)
}

/// Cross-window: a brand-new window whose title contains the target counts
/// as completion (e.g. "Descargas - Explorador" appears).
fn new_window_matches_target(
    step: &GuideStep,
    before: &ScreenFrame,
    after: &ScreenFrame,
) -> Option<String> {
    if step.target_text.is_empty() {
        return None;
    }
    let target = step.target_text.to_lowercase();
    let before_titles: Vec<String> = before
        .windows
        .iter()
        .map(|w| w.title.to_lowercase())
        .collect();
    for w in &after.windows {
        let title = w.title.to_lowercase();
        if title.is_empty() {
            continue;
        }
        if !title.contains(&target) {
            continue;
        }
        if before_titles.iter().any(|t| t == &title) {
            continue;
        }
        return Some(format!("new window matched target: {}", w.title));
    }
    None
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
    use crate::perception::{
        ElementSource, PerceptionQuality, Rect, ScreenElement, ScreenFrame, WindowId,
        WindowSnapshot,
    };

    fn frame(elements: Vec<ScreenElement>, window: &str) -> ScreenFrame {
        let windows = if window.is_empty() {
            Vec::new()
        } else {
            vec![WindowSnapshot {
                id: WindowId(1),
                title: window.into(),
                class_name: "CabinetWClass".into(),
                bounds: Rect::new(0, 0, 1280, 720),
                is_foreground: true,
                z_order: 0,
                uia_element_count: elements.len(),
            }]
        };
        ScreenFrame {
            primary_window_id: WindowId(1),
            windows,
            elements,
            quality: PerceptionQuality::Full,
            ..ScreenFrame::empty()
        }
    }

    fn element(text: &str) -> ScreenElement {
        ScreenElement {
            source: ElementSource::Uia,
            text: text.into(),
            bounds: Rect::new(0, 0, 10, 10),
            window_id: WindowId(1),
            kind: "Button".into(),
            confidence: 1.0,
            automation_id: None,
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
        let before = frame(vec![element("Descargas")], "Explorer");
        let after = frame(vec![], "Explorer");
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
        let before = frame(vec![], "Explorer");
        let after = frame(vec![], "Descargas");
        let outcome = StateDetector.is_completed(
            &step("Descargas", ActionVerb::DoubleClick),
            &before,
            &after,
            0,
        );
        assert!(!outcome.completed);
    }

    #[test]
    fn roota_foreground_never_completes() {
        let before = frame(vec![], "Explorador");
        let after = frame(vec![], "Roota");
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
        let before = frame(vec![], "Inicio - Explorador de archivos");
        let after = frame(vec![], "Descargas - Explorador de archivos");
        let outcome = StateDetector.is_completed(
            &step("Descargas", ActionVerb::DoubleClick),
            &before,
            &after,
            MIN_POLLS_BEFORE_COMPLETE,
        );
        assert!(outcome.completed);
    }

    #[test]
    fn new_window_with_target_in_title_completes() {
        // Same primary window, but a new background window appears matching
        // the target — multi-window detector should still flag completion.
        let mut before = frame(vec![element("Descargas")], "Explorer");
        let mut after = frame(vec![], "Explorer");
        before.windows.push(WindowSnapshot {
            id: WindowId(2),
            title: "Chrome".into(),
            class_name: "Chrome_WidgetWin_1".into(),
            bounds: Rect::new(1280, 0, 1280, 720),
            is_foreground: false,
            z_order: 1,
            uia_element_count: 0,
        });
        after.windows.push(WindowSnapshot {
            id: WindowId(2),
            title: "Chrome".into(),
            class_name: "Chrome_WidgetWin_1".into(),
            bounds: Rect::new(1280, 0, 1280, 720),
            is_foreground: false,
            z_order: 2,
            uia_element_count: 0,
        });
        after.windows.push(WindowSnapshot {
            id: WindowId(3),
            title: "Descargas - Explorador de archivos".into(),
            class_name: "CabinetWClass".into(),
            bounds: Rect::new(0, 0, 1280, 720),
            is_foreground: false,
            z_order: 1,
            uia_element_count: 5,
        });
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
        let s = frame(vec![element("Descargas")], "Explorer");
        let outcome = StateDetector.is_completed(
            &step("Descargas", ActionVerb::Click),
            &s,
            &s,
            MIN_POLLS_BEFORE_COMPLETE,
        );
        assert!(!outcome.completed);
    }
}
