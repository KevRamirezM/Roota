"""
Floating assistant main window.

Single-action, high-contrast UX:
- Big greeting label.
- Large input box.
- One primary button ("Empezar") and one mic button.
- Feedback panel docked below.
"""

from __future__ import annotations

from PySide6.QtCore import Qt, Signal
from PySide6.QtGui import QKeyEvent
from PySide6.QtWidgets import (
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QMainWindow,
    QPushButton,
    QVBoxLayout,
    QWidget,
)

from app.i18n import t
from app.ui.feedback_panel import FeedbackPanel


class MainWindow(QMainWindow):
    """Top-level Roota window. Emits `command_submitted(str)` and `mic_pressed`."""

    command_submitted = Signal(str)
    mic_pressed = Signal()
    mic_released = Signal()

    def __init__(self, parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self.setWindowTitle(t("app.title"))
        self.resize(960, 640)

        title = QLabel(t("app.title"))
        title.setObjectName("TitleLabel")
        title.setAlignment(Qt.AlignmentFlag.AlignCenter)

        subtitle = QLabel(t("main.greeting"))
        subtitle.setObjectName("SubtitleLabel")
        subtitle.setAlignment(Qt.AlignmentFlag.AlignCenter)
        subtitle.setWordWrap(True)

        self._input = QLineEdit()
        self._input.setPlaceholderText(t("main.input_placeholder"))
        self._input.setAccessibleName(t("main.greeting"))
        self._input.setMinimumHeight(72)
        self._input.returnPressed.connect(self._on_submit)

        self._send = QPushButton(t("main.send_button"))
        self._send.setAccessibleName(t("main.send_button"))
        self._send.setMinimumHeight(72)
        self._send.clicked.connect(self._on_submit)

        self._mic = QPushButton(t("main.mic_button"))
        self._mic.setObjectName("MicButton")
        self._mic.setAccessibleName(t("main.mic_button"))
        self._mic.setMinimumHeight(72)
        self._mic.setCheckable(False)
        self._mic.pressed.connect(self.mic_pressed.emit)
        self._mic.released.connect(self.mic_released.emit)

        action_row = QHBoxLayout()
        action_row.setSpacing(20)
        action_row.addWidget(self._send, stretch=2)
        action_row.addWidget(self._mic, stretch=1)

        self._feedback = FeedbackPanel()

        body = QVBoxLayout()
        body.setContentsMargins(40, 32, 40, 32)
        body.setSpacing(24)
        body.addWidget(title)
        body.addWidget(subtitle)
        body.addWidget(self._input)
        body.addLayout(action_row)
        body.addWidget(self._feedback, stretch=1)

        container = QWidget()
        container.setLayout(body)
        self.setCentralWidget(container)

    @property
    def feedback(self) -> FeedbackPanel:
        return self._feedback

    def set_recording(self, recording: bool) -> None:
        self._mic.setText(t("main.mic_recording") if recording else t("main.mic_button"))

    def show_transcribed(self, text: str) -> None:
        self._input.setText(text)
        self._input.setFocus()

    def lock_input(self, locked: bool) -> None:
        self._input.setReadOnly(locked)
        self._send.setEnabled(not locked)

    def keyPressEvent(self, event: QKeyEvent) -> None:  # noqa: N802
        if event.key() in (Qt.Key.Key_Escape,):
            self._feedback.clear()
            return
        super().keyPressEvent(event)

    def _on_submit(self) -> None:
        text = self._input.text().strip()
        if not text:
            return
        self._input.clear()
        self.command_submitted.emit(text)
