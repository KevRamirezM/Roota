"""
QObject wrapper that drives the pure `Orchestrator` from a `QThread`.

UI components subscribe to Qt signals here so that:
- the long-running guide loop never blocks the event loop, and
- the test suite can keep using the pure `Orchestrator` directly.
"""

from __future__ import annotations

from typing import Any

from PySide6.QtCore import QObject, Signal, Slot

from app.orchestration.orchestrator import (
    ConfirmationRequest,
    ErrorEvent,
    GoalCompleteEvent,
    Orchestrator,
)
from app.state.session import GuideStep
from app.telemetry import get_logger

logger = get_logger("app.orchestration.worker")


class OrchestratorWorker(QObject):
    """Qt-friendly façade over `Orchestrator`."""

    confirmation_requested = Signal(object)  # ConfirmationRequest
    instruction_ready = Signal(object)  # GuideStep
    anchor_changed = Signal(int, int, str)
    goal_completed = Signal(object)  # GoalCompleteEvent
    error_raised = Signal(object)  # ErrorEvent
    finished = Signal()

    def __init__(self, orchestrator: Orchestrator, parent: QObject | None = None) -> None:
        super().__init__(parent)
        self._orchestrator = orchestrator
        self._pending_confirmation: bool | None = None

    @Slot(bool)
    def resolve_confirmation(self, accepted: bool) -> None:
        self._pending_confirmation = accepted

    @Slot(str)
    def run(self, utterance: str) -> None:
        try:
            self._orchestrator.run(
                utterance,
                confirm=self._wait_for_confirmation,
                on_step=self._emit_step,
                on_error=lambda event: self.error_raised.emit(event),
                on_complete=lambda event: self.goal_completed.emit(event),
            )
        except Exception as exc:  # pragma: no cover - defensive
            logger.exception(f"orchestrator crashed: {exc!r}")
            self.error_raised.emit(ErrorEvent(message=str(exc)))
        finally:
            self.finished.emit()

    def _wait_for_confirmation(self, request: ConfirmationRequest) -> bool:
        from PySide6.QtCore import QCoreApplication

        self._pending_confirmation = None
        self.confirmation_requested.emit(request)
        while self._pending_confirmation is None:
            QCoreApplication.processEvents()
        return self._pending_confirmation

    def _emit_step(self, step: GuideStep) -> None:
        self.instruction_ready.emit(step)
        if step.anchor_xy is not None:
            self.anchor_changed.emit(step.anchor_xy[0], step.anchor_xy[1], step.target_text)


__all__ = ["OrchestratorWorker"]


def _ensure_typing(_: Any) -> None:
    """No-op kept so static checkers see GoalCompleteEvent imported."""
    _ = GoalCompleteEvent
