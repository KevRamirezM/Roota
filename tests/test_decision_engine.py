"""Tests for the DecisionEngine."""

from __future__ import annotations

import pytest

from app.accessibility.element import UIElement, UISnapshot
from app.orchestration.decision import DecisionEngine, StepResolutionError
from app.orchestration.templates import default_registry
from app.state.session import Intent, SessionState


def _snapshot_with(*elements: UIElement, window: str = "Explorer") -> UISnapshot:
    return UISnapshot(window=window, elements=tuple(elements))


def _explorer_snapshot() -> UISnapshot:
    return _snapshot_with(
        UIElement(type="button", text="Descargas", x=100, y=300, width=160, height=32, automation_id="downloads", window="Explorer"),
        UIElement(type="button", text="Documentos", x=100, y=340, width=160, height=32, automation_id="documents", window="Explorer"),
    )


def _gmail_snapshot() -> UISnapshot:
    return _snapshot_with(
        UIElement(type="button", text="Redactar", x=40, y=160, width=120, height=40, automation_id="compose", window="Gmail"),
        window="Gmail",
    )


def test_open_folder_targets_descargas() -> None:
    engine = DecisionEngine()
    template = default_registry().get("open_folder")
    assert template is not None
    intent = Intent(intent="open_folder", target="Descargas")
    session = SessionState()
    session.begin(intent, total_steps=len(template.steps))

    step = engine.next_step(intent, template, _explorer_snapshot(), session)

    assert step.action == "double_click"
    assert step.target_text == "Descargas"
    assert step.anchor_xy == (180, 316)
    assert "Descargas" in step.instruction


def test_compose_email_targets_redactar_in_gmail() -> None:
    engine = DecisionEngine()
    template = default_registry().get("compose_email")
    assert template is not None
    intent = Intent(intent="compose_email", target="Elena")
    session = SessionState()
    session.begin(intent, total_steps=len(template.steps))

    step = engine.next_step(intent, template, _gmail_snapshot(), session)

    assert step.target_text == "Redactar"
    assert step.anchor_xy is not None


def test_missing_element_returns_step_without_anchor() -> None:
    engine = DecisionEngine()
    template = default_registry().get("open_folder")
    assert template is not None
    intent = Intent(intent="open_folder", target="Música")  # not in stub
    session = SessionState()
    session.begin(intent, total_steps=len(template.steps))

    step = engine.next_step(intent, template, _explorer_snapshot(), session)

    assert step.target_text == "Música"
    assert step.anchor_xy is None


def test_step_resolution_error_when_index_out_of_range() -> None:
    engine = DecisionEngine()
    template = default_registry().get("open_folder")
    assert template is not None
    intent = Intent(intent="open_folder", target="Descargas")
    session = SessionState()
    session.begin(intent, total_steps=len(template.steps))
    session.step_index = len(template.steps)

    with pytest.raises(StepResolutionError):
        engine.next_step(intent, template, _explorer_snapshot(), session)
