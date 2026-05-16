"""
Roota application entry point.

Wires together:
- `Orchestrator` (brain)
- `OrchestratorWorker` on a `QThread`
- `MainWindow` (input + feedback)
- `OverlayWindow` (visual anchors)
- `ConfirmationModal` (safety gate)

Run via `python -m app.ui.main` or the `roota` console script.
"""

from __future__ import annotations

import sys
from typing import Any

from PySide6.QtCore import QObject, Qt, QThread, Signal, Slot
from PySide6.QtGui import QFont
from PySide6.QtWidgets import QApplication

from app.accessibility.scanner import get_scanner
from app.config import get_settings
from app.llm import get_llm_client
from app.orchestration.orchestrator import (
    ConfirmationRequest,
    ErrorEvent,
    GoalCompleteEvent,
    Orchestrator,
)
from app.orchestration.templates import TemplateRegistry
from app.orchestration.worker import OrchestratorWorker
from app.overlay.controller import OverlayController
from app.overlay.window import OverlayWindow
from app.state.session import GuideStep
from app.telemetry import configure_logging, get_logger
from app.ui.confirmation_modal import ConfirmationModal
from app.ui.main_window import MainWindow
from app.ui.theme import base_stylesheet
from app.voice.recorder import MicrophoneRecorder
from app.voice.stt import get_stt
from app.voice.tts import get_tts


def _load_template_registry() -> TemplateRegistry:
    """Load JSON-defined templates from app/prompts/templates if present."""
    from pathlib import Path

    root = Path(__file__).resolve().parent.parent / "prompts" / "templates"
    return TemplateRegistry.from_json_dir(root)


class _UiBridge(QObject):
    """Routes signals between the Qt UI and the orchestrator worker thread."""

    submit_command = Signal(str)
    confirm_response = Signal(bool)


def main() -> int:
    settings = get_settings()
    configure_logging()
    logger = get_logger("app.ui.main")

    app = QApplication.instance() or QApplication(sys.argv)
    app.setApplicationName("Roota")
    app.setStyleSheet(base_stylesheet())
    app.setFont(QFont("Segoe UI", settings.UI_FONT_SIZE))

    overlay = OverlayWindow()
    overlay.show()
    overlay_controller = OverlayController(window=overlay)

    main_window = MainWindow()
    main_window.show()

    llm = get_llm_client()
    scanner = get_scanner()
    templates = _load_template_registry()
    orchestrator = Orchestrator(llm=llm, scanner=scanner, templates=templates)
    worker = OrchestratorWorker(orchestrator)
    thread = QThread()
    worker.moveToThread(thread)
    thread.start()

    tts = get_tts()
    stt = get_stt()
    recorder = MicrophoneRecorder()

    bridge = _UiBridge()

    @Slot(str)
    def on_submit(text: str) -> None:
        main_window.lock_input(True)
        bridge.submit_command.emit(text)

    @Slot(object)
    def on_confirmation(request: ConfirmationRequest) -> None:
        modal = ConfirmationModal(action_message=request.message, parent=main_window)
        accepted = modal.exec() == modal.DialogCode.Accepted
        bridge.confirm_response.emit(accepted)

    @Slot(object)
    def on_step(step: GuideStep) -> None:
        main_window.feedback.show_step(step)
        if step.anchor_xy is not None:
            overlay_controller.show_anchor(step.anchor_xy[0], step.anchor_xy[1], step.target_text)
        else:
            overlay_controller.clear()
        tts.speak(step.instruction)

    @Slot(object)
    def on_complete(_: GoalCompleteEvent) -> None:
        from app.i18n import t

        main_window.feedback.show_completion()
        overlay_controller.clear()
        main_window.lock_input(False)
        tts.speak(t("feedback.completed_body"))

    @Slot(object)
    def on_error(event: ErrorEvent) -> None:
        main_window.feedback.show_error(event.message)
        overlay_controller.clear()
        main_window.lock_input(False)
        tts.speak(event.message)

    @Slot()
    def on_mic_pressed() -> None:
        if recorder.start():
            main_window.set_recording(True)

    @Slot()
    def on_mic_released() -> None:
        audio, sr = recorder.stop()
        main_window.set_recording(False)
        if audio.size == 0:
            return
        text = stt.transcribe(audio, sr)
        if text:
            main_window.show_transcribed(text)

    main_window.mic_pressed.connect(on_mic_pressed)
    main_window.mic_released.connect(on_mic_released)

    main_window.command_submitted.connect(on_submit)
    bridge.submit_command.connect(worker.run, type=Qt.ConnectionType.QueuedConnection)
    bridge.confirm_response.connect(worker.resolve_confirmation, type=Qt.ConnectionType.QueuedConnection)
    worker.confirmation_requested.connect(on_confirmation, type=Qt.ConnectionType.QueuedConnection)
    worker.instruction_ready.connect(on_step, type=Qt.ConnectionType.QueuedConnection)
    worker.goal_completed.connect(on_complete, type=Qt.ConnectionType.QueuedConnection)
    worker.error_raised.connect(on_error, type=Qt.ConnectionType.QueuedConnection)

    logger.info(
        f"Roota started. LLM={getattr(llm, 'name', '?')} scanner={getattr(scanner, 'name', '?')} lang={settings.UI_LANGUAGE}"
    )

    exit_code = app.exec()
    thread.quit()
    thread.wait(2000)
    return exit_code


if __name__ == "__main__":  # pragma: no cover
    sys.exit(main())


def _typing_used(_: Any) -> None:
    """Kept so static checkers see the worker import is intentional."""
    _ = OrchestratorWorker
