//! Interpret user cursor/clicks against the current step (read-only coaching).

use crate::i18n;
use crate::input::InputSample;
use crate::orchestration::detector::MIN_POLLS_BEFORE_COMPLETE;
use crate::orchestration::state::{ActionVerb, GuideStep};
use crate::perception::ScreenFrame;
use crate::settings::Lang;

/// User performed the expected gesture on the anchored target.
#[derive(Debug, Clone)]
pub struct UserActionCompletion {
    pub reason: String,
}

/// Returns true when the cursor is inside the step anchor (requires bounds).
pub fn click_on_target(step: &GuideStep, input: &InputSample) -> bool {
    let (tx, ty, tw, th) = match step.anchor_bounds {
        Some(b) => b,
        None => return false,
    };
    point_in_rect(input.cursor.x, input.cursor.y, tx, ty, tw, th)
}

/// Fast path: user clicked the highlighted target with the expected gesture.
/// Does not wait for UIA tree changes (Explorer keeps "Descargas" visible after open).
pub fn user_action_completed(
    step: &GuideStep,
    input: &InputSample,
    target_click_count: u32,
) -> Option<UserActionCompletion> {
    if !input.left_click && !input.right_click && !input.double_click && target_click_count == 0 {
        return None;
    }

    let on_target = click_on_target(step, input);
    if !on_target && target_click_count == 0 {
        return None;
    }

    let acted = input.left_click || input.right_click || input.double_click;
    if !acted && target_click_count == 0 {
        return None;
    }

    match step.action {
        ActionVerb::DoubleClick => {
            if input.double_click {
                return Some(UserActionCompletion {
                    reason: "user double-clicked target".into(),
                });
            }
            // Two single clicks on target (handles slow poll missing the double-click edge).
            if target_click_count >= 2 {
                return Some(UserActionCompletion {
                    reason: "user double-clicked target (two clicks)".into(),
                });
            }
            if input.left_click && on_target {
                return None;
            }
        }
        ActionVerb::RightClick => {
            if input.right_click && on_target {
                return Some(UserActionCompletion {
                    reason: "user right-clicked target".into(),
                });
            }
        }
        ActionVerb::Click => {
            if input.left_click && on_target && !input.right_click {
                return Some(UserActionCompletion {
                    reason: "user clicked target".into(),
                });
            }
        }
        ActionVerb::Type | ActionVerb::Locate => {
            if acted && on_target {
                return Some(UserActionCompletion {
                    reason: "user activated target".into(),
                });
            }
        }
    }
    None
}

pub fn corrective_message(
    step: &GuideStep,
    input: &InputSample,
    frame_before: &ScreenFrame,
    frame_after: &ScreenFrame,
    poll_index: u32,
    lang: Lang,
) -> Option<String> {
    if poll_index < MIN_POLLS_BEFORE_COMPLETE {
        return None;
    }

    if let Some(msg) = assess_click_against_target(step, input, lang) {
        return Some(msg);
    }

    if input.left_click || input.right_click || input.double_click {
        return assess_unexpected_ui_change(step, frame_before, frame_after, lang);
    }
    None
}

fn assess_click_against_target(step: &GuideStep, input: &InputSample, lang: Lang) -> Option<String> {
    let (tx, ty, tw, th) = step.anchor_bounds?;
    let clicked = input.left_click || input.right_click || input.double_click;
    if !clicked {
        return None;
    }

    let cx = input.cursor.x;
    let cy = input.cursor.y;

    if !point_in_rect(cx, cy, tx, ty, tw, th) {
        return Some(i18n::t(
            "guidance.wrong_click_location",
            lang,
            &[("target", &step.target_text)],
        ));
    }

    match step.action {
        ActionVerb::DoubleClick if input.left_click && !input.double_click => Some(i18n::t(
            "guidance.wrong_single_click",
            lang,
            &[("target", &step.target_text)],
        )),
        ActionVerb::RightClick if input.left_click && !input.right_click => Some(i18n::t(
            "guidance.wrong_left_click_need_right",
            lang,
            &[("target", &step.target_text)],
        )),
        ActionVerb::Click if input.right_click => Some(i18n::t(
            "guidance.wrong_right_click_need_left",
            lang,
            &[("target", &step.target_text)],
        )),
        _ => None,
    }
}

fn assess_unexpected_ui_change(
    step: &GuideStep,
    before: &ScreenFrame,
    after: &ScreenFrame,
    lang: Lang,
) -> Option<String> {
    let before_title = before.primary_window_title();
    let after_title = after.primary_window_title();
    if before_title.is_empty() || after_title.is_empty() {
        return None;
    }
    let b = before_title.to_lowercase();
    let a = after_title.to_lowercase();
    if b == a {
        return None;
    }
    if is_roota_title(&a) {
        return Some(i18n::t("guidance.wrong_focus_roota", lang, &[]));
    }
    let target = step.target_text.to_lowercase();
    if step.action == ActionVerb::DoubleClick
        && !a.contains(&target)
        && !a.contains("explorador")
        && !a.contains("explorer")
    {
        return Some(i18n::t(
            "guidance.wrong_window",
            lang,
            &[("window", &after_title), ("target", &step.target_text)],
        ));
    }
    None
}

pub fn point_in_rect(px: i32, py: i32, x: i32, y: i32, w: i32, h: i32) -> bool {
    px >= x && px <= x + w && py >= y && py <= y + h
}

fn is_roota_title(title: &str) -> bool {
    title.to_lowercase().contains("roota")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::state::GuideStep;

    #[test]
    fn point_inside_bounds() {
        assert!(point_in_rect(150, 160, 100, 150, 80, 40));
        assert!(!point_in_rect(50, 50, 100, 150, 80, 40));
    }

    #[test]
    fn user_action_completed_on_target_click() {
        let step = GuideStep {
            index: 1,
            total: 1,
            action: ActionVerb::Click,
            target_text: "Descargas".into(),
            instruction: String::new(),
            anchor_xy: Some((140, 170)),
            anchor_bounds: Some((100, 150, 80, 40)),
        };
        let input = InputSample {
            cursor: crate::input::PhysicalPoint { x: 140, y: 170 },
            left_click: true,
            ..Default::default()
        };
        assert!(user_action_completed(&step, &input, 0).is_some());
    }

    #[test]
    fn double_click_accepts_two_target_clicks() {
        let step = GuideStep {
            index: 1,
            total: 1,
            action: ActionVerb::DoubleClick,
            target_text: "Descargas".into(),
            instruction: String::new(),
            anchor_xy: Some((140, 170)),
            anchor_bounds: Some((100, 150, 80, 40)),
        };
        let input = InputSample {
            cursor: crate::input::PhysicalPoint { x: 140, y: 170 },
            left_click: true,
            ..Default::default()
        };
        assert!(user_action_completed(&step, &input, 2).is_some());
    }

    #[test]
    fn wrong_click_outside_target() {
        let step = GuideStep {
            index: 1,
            total: 1,
            action: ActionVerb::Click,
            target_text: "Descargas".into(),
            instruction: String::new(),
            anchor_xy: Some((140, 170)),
            anchor_bounds: Some((100, 150, 80, 40)),
        };
        let input = InputSample {
            cursor: crate::input::PhysicalPoint { x: 10, y: 10 },
            left_click: true,
            ..Default::default()
        };
        let msg = assess_click_against_target(&step, &input, Lang::Es);
        assert!(msg.is_some());
    }
}
