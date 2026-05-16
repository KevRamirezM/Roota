"""Tests for the session state machine."""

from __future__ import annotations

from app.state import GuideStep, Intent, SessionState, SessionStore


def _intent(name: str = "open_folder", target: str = "Descargas") -> Intent:
    return Intent(intent=name, target=target, raw_utterance="abre Descargas")


def test_begin_resets_session() -> None:
    state = SessionState()
    state.begin(_intent(), total_steps=3)

    assert state.intent is not None
    assert state.intent.intent == "open_folder"
    assert state.total_steps == 3
    assert state.step_index == 0
    assert state.completed is False


def test_advance_marks_completed_when_done() -> None:
    state = SessionState()
    state.begin(_intent(), total_steps=2)

    state.advance()
    assert state.step_index == 1
    assert state.completed is False

    state.advance()
    assert state.step_index == 2
    assert state.completed is True


def test_record_appends_history_and_updates_index() -> None:
    state = SessionState()
    state.begin(_intent(), total_steps=2)

    step = GuideStep(
        index=1,
        total=2,
        action="click",
        target_text="Descargas",
        instruction="Haz clic en Descargas.",
        anchor_xy=(200, 360),
    )
    state.record(step)

    assert state.history[-1] is step
    assert state.step_index == 1


def test_reset_clears_everything() -> None:
    state = SessionState()
    state.begin(_intent(), total_steps=3)
    state.advance()
    state.reset()

    assert state.intent is None
    assert state.history == []
    assert state.step_index == 0
    assert state.completed is False


def test_session_store_default_state_is_empty() -> None:
    store = SessionStore()
    assert store.state.intent is None
    assert store.state.history == []


def test_intent_is_known_predicate() -> None:
    assert _intent().is_known() is True
    assert Intent(intent="unknown", target="").is_known() is False
