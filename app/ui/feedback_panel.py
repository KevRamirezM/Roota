"""
Feedback panel — current step instruction and positive completion state.
Lives inside the main window; never blocks input.
"""

from __future__ import annotations

from PySide6.QtCore import Qt, Slot
from PySide6.QtWidgets import QFrame, QLabel, QVBoxLayout, QWidget

from app.i18n import t
from app.state.session import GuideStep


class FeedbackPanel(QFrame):
    """Card showing the current step or the success/error state."""

    def __init__(self, parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self.setObjectName("FeedbackCard")
        self.setFrameShape(QFrame.Shape.NoFrame)

        self._step_label = QLabel("")
        self._step_label.setObjectName("StepLabel")
        self._step_label.setAlignment(Qt.AlignmentFlag.AlignLeft)

        self._instruction_label = QLabel("")
        self._instruction_label.setWordWrap(True)
        self._instruction_label.setAlignment(Qt.AlignmentFlag.AlignLeft)

        layout = QVBoxLayout(self)
        layout.setContentsMargins(28, 24, 28, 24)
        layout.setSpacing(14)
        layout.addWidget(self._step_label)
        layout.addWidget(self._instruction_label)

    @Slot(object)
    def show_step(self, step: GuideStep) -> None:
        self._step_label.setText(t("feedback.step_label", step=step.index, total=step.total))
        self._instruction_label.setText(step.instruction)

    @Slot()
    def show_completion(self) -> None:
        self._step_label.setText(t("feedback.completed_title"))
        self._instruction_label.setText(t("feedback.completed_body"))

    @Slot(str)
    def show_error(self, message: str) -> None:
        self._step_label.setText(t("feedback.error_title"))
        self._instruction_label.setText(t("feedback.error_body", message=message))

    def clear(self) -> None:
        self._step_label.setText("")
        self._instruction_label.setText("")


__all__ = ["FeedbackPanel"]
