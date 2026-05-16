"""Tests for the StateDetector heuristics."""

from __future__ import annotations

from app.accessibility.element import UIElement, UISnapshot
from app.orchestration.state_detector import StateDetector
from app.state.session import GuideStep


def _step(target: str = "Descargas") -> GuideStep:
    return GuideStep(
        index=1,
        total=1,
        action="click",
        target_text=target,
        instruction="Haz clic en Descargas",
        anchor_xy=(100, 100),
    )


def _snap_with(*elements: UIElement, window: str = "Explorer") -> UISnapshot:
    return UISnapshot(window=window, elements=tuple(elements))


def test_target_disappeared_means_completed() -> None:
    before = _snap_with(UIElement(type="button", text="Descargas", x=0, y=0, width=10, height=10))
    after = _snap_with()
    out = StateDetector().is_completed(_step(), before, after)
    assert out.completed is True


def test_window_change_means_completed() -> None:
    before = _snap_with(window="Explorer")
    after = _snap_with(UIElement(type="button", text="Algo", x=0, y=0, width=10, height=10), window="Word")
    out = StateDetector().is_completed(_step(), before, after)
    assert out.completed is True


def test_no_change_means_pending() -> None:
    snapshot = _snap_with(UIElement(type="button", text="Descargas", x=0, y=0, width=10, height=10))
    out = StateDetector().is_completed(_step(), snapshot, snapshot)
    assert out.completed is False
