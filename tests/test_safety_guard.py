"""Tests for the SafetyGuard — the heart of Roota's safety promise."""

from __future__ import annotations

import pytest

from app.safety import GuideAction, SafetyGuard, UnsafeActionError


@pytest.mark.parametrize("action_type", ["highlight", "anchor", "arrow", "speak", "show_text", "scan"])
def test_guide_actions_pass(action_type: str) -> None:
    guard = SafetyGuard()
    action = GuideAction(type=action_type, target="Downloads")  # type: ignore[arg-type]

    assert guard.review(action) is action
    assert guard.is_safe(action) is True


@pytest.mark.parametrize(
    "action_type",
    ["click", "double_click", "right_click", "type_text", "key_press", "drag", "scroll", "file_op"],
)
def test_automation_actions_rejected(action_type: str) -> None:
    guard = SafetyGuard()
    action = GuideAction(type=action_type, target="Downloads")  # type: ignore[arg-type]

    with pytest.raises(UnsafeActionError):
        guard.review(action)
    assert guard.is_safe(action) is False


def test_unknown_action_rejected_in_strict_mode() -> None:
    guard = SafetyGuard(strict=True)
    bogus = GuideAction(type="hack_the_planet")  # type: ignore[arg-type]

    with pytest.raises(UnsafeActionError):
        guard.review(bogus)


def test_review_message_explains_refusal() -> None:
    guard = SafetyGuard()
    with pytest.raises(UnsafeActionError, match="only guides"):
        guard.review(GuideAction(type="click"))
