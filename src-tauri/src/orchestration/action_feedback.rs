//! Interpret user cursor/clicks against the current step (read-only coaching).

use crate::accessibility::element::UiSnapshot;
use crate::i18n;
use crate::input::InputSample;
use crate::orchestration::detector::MIN_POLLS_BEFORE_COMPLETE;
use crate::orchestration::state::{ActionVerb, GuideStep};
use crate::settings::Lang;

pub fn corrective_message(
    step: &GuideStep,
    input: &InputSample,
    snapshot_before: &UiSnapshot,
    snapshot_after: &UiSnapshot,
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
        return assess_unexpected_ui_change(step, snapshot_before, snapshot_after, lang);
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
    before: &UiSnapshot,
    snapshot_after: &UiSnapshot,
    lang: Lang,
) -> Option<String> {
    if before.window.is_empty() || snapshot_after.window.is_empty() {
        return None;
    }
    let b = before.window.to_lowercase();
    let a = snapshot_after.window.to_lowercase();
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
            &[("window", &snapshot_after.window), ("target", &step.target_text)],
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
