"""Smoke tests for the PySide6 UI shells.

We only verify they construct without exceptions and basic signals
fire. This sanity-checks the wiring without standing up a full app.
"""

from __future__ import annotations

import os
import sys

import pytest

# Use the offscreen platform plugin so this works on CI / headless dev boxes.
os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")


pytestmark = pytest.mark.skipif(
    "PYTEST_DISABLE_QT" in os.environ, reason="Qt smoke tests disabled"
)


@pytest.fixture(scope="module")
def qapp():
    from PySide6.QtWidgets import QApplication

    app = QApplication.instance() or QApplication(sys.argv)
    yield app


def test_main_window_constructs(qapp) -> None:
    from app.ui.main_window import MainWindow

    window = MainWindow()
    window.show()
    assert window.windowTitle() == "Roota"
    window.close()


def test_main_window_emits_command(qapp) -> None:
    from app.ui.main_window import MainWindow

    window = MainWindow()
    received: list[str] = []
    window.command_submitted.connect(received.append)
    window._input.setText("Abre Descargas")  # type: ignore[attr-defined]
    window._on_submit()  # type: ignore[attr-defined]
    assert received == ["Abre Descargas"]


def test_confirmation_modal_constructs(qapp) -> None:
    from app.ui.confirmation_modal import ConfirmationModal

    modal = ConfirmationModal(action_message="abrir Descargas")
    assert modal.minimumWidth() >= 400


def test_feedback_panel_displays_step(qapp) -> None:
    from app.state.session import GuideStep
    from app.ui.feedback_panel import FeedbackPanel

    panel = FeedbackPanel()
    panel.show_step(
        GuideStep(
            index=1,
            total=2,
            action="click",
            target_text="Descargas",
            instruction="Haz clic en Descargas.",
            anchor_xy=(10, 20),
        )
    )
    assert "Descargas" in panel._instruction_label.text()  # type: ignore[attr-defined]


def test_overlay_window_constructs(qapp) -> None:
    from app.overlay.shapes import Anchor
    from app.overlay.window import OverlayWindow

    overlay = OverlayWindow()
    overlay.set_anchors([Anchor(x=100, y=100, label="x")])
    overlay.clear()
