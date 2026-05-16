"""End-to-end orchestrator integration test using stubs."""

from __future__ import annotations

from typing import Any

from app.accessibility.element import UIElement, UISnapshot
from app.accessibility.stub_scanner import StubScanner
from app.orchestration.orchestrator import (
    ConfirmationRequest,
    ErrorEvent,
    GoalCompleteEvent,
    Orchestrator,
)
from app.state.session import GuideStep


class _CannedLLM:
    name = "canned"

    def health_check(self) -> bool:
        return True

    def complete_text(self, prompt: str, *, system: str | None = None) -> str:
        return "ok"

    def complete_json(
        self,
        prompt: str,
        *,
        system: str | None = None,
        schema_hint: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        return {"intent": "open_folder", "target": "Descargas", "params": {}}


def _make_scanner_sequence(snapshots: list[UISnapshot]) -> StubScanner:
    """A scanner whose `snapshot()` walks through `snapshots`."""
    scanner = StubScanner(snapshot=snapshots[0])
    cursor = {"i": 0}

    def stepping() -> UISnapshot:
        i = min(cursor["i"], len(snapshots) - 1)
        cursor["i"] += 1
        return snapshots[i]

    scanner.snapshot = stepping  # type: ignore[assignment]
    return scanner


def test_full_happy_path_emits_step_and_completes() -> None:
    descargas = UIElement(type="button", text="Descargas", x=100, y=300, width=160, height=32, automation_id="downloads", window="Explorer")
    snap_before = UISnapshot(window="Explorer", elements=(descargas,))
    snap_after = UISnapshot(window="Explorer", elements=())  # user "double-clicked", target gone

    scanner = _make_scanner_sequence([snap_before, snap_before, snap_after])
    orchestrator = Orchestrator(
        llm=_CannedLLM(),
        scanner=scanner,
        sleep=lambda _: None,
    )

    captured: dict[str, list[Any]] = {"steps": [], "completed": [], "errors": []}

    orchestrator.run(
        "Abre la carpeta de Descargas",
        confirm=lambda req: True,
        on_step=lambda step: captured["steps"].append(step),
        on_error=lambda err: captured["errors"].append(err),
        on_complete=lambda event: captured["completed"].append(event),
        max_polls=5,
        poll_interval=0,
    )

    assert captured["errors"] == []
    assert len(captured["steps"]) == 1
    step = captured["steps"][0]
    assert isinstance(step, GuideStep)
    assert step.target_text == "Descargas"
    assert step.anchor_xy is not None
    assert len(captured["completed"]) == 1
    assert isinstance(captured["completed"][0], GoalCompleteEvent)


def test_user_cancels_at_confirmation() -> None:
    orchestrator = Orchestrator(
        llm=_CannedLLM(),
        scanner=StubScanner(),
        sleep=lambda _: None,
    )

    errors: list[ErrorEvent] = []
    orchestrator.run(
        "Abre Descargas",
        confirm=lambda req: False,
        on_step=lambda step: None,
        on_error=lambda err: errors.append(err),
        on_complete=lambda event: None,
    )

    assert len(errors) == 1


def test_unknown_intent_emits_friendly_error() -> None:
    class _UnknownLLM:
        name = "unknown"

        def health_check(self) -> bool:
            return True

        def complete_text(self, *_: Any, **__: Any) -> str:
            return ""

        def complete_json(self, *_: Any, **__: Any) -> dict[str, Any]:
            return {"intent": "make_coffee", "target": "", "params": {}}

    orchestrator = Orchestrator(
        llm=_UnknownLLM(),
        scanner=StubScanner(),
        sleep=lambda _: None,
    )
    errors: list[ErrorEvent] = []
    orchestrator.run(
        "haz café",
        confirm=lambda req: True,
        on_step=lambda step: None,
        on_error=lambda err: errors.append(err),
        on_complete=lambda event: None,
    )
    assert len(errors) == 1
    assert "no sé" in errors[0].message.lower() or "don't know" in errors[0].message.lower()


def test_confirmation_request_message_uses_locale(fake_env) -> None:
    fake_env(UI_LANGUAGE="es")
    orchestrator = Orchestrator(
        llm=_CannedLLM(),
        scanner=StubScanner(),
        sleep=lambda _: None,
    )
    intent, template = orchestrator.classify("Abre Descargas")
    assert template is not None
    request = orchestrator.build_confirmation(intent, template)

    assert isinstance(request, ConfirmationRequest)
    assert "Descargas" in request.message
    assert "abrir" in request.message.lower()
