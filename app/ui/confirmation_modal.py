"""
Safety Confirmation Gate (PRD §5 step 3).

Massive YES (green) / NO (red) buttons. Keyboard accessible —
Y / Enter for YES, N / Esc for NO. Returns boolean from `exec()`.
"""

from __future__ import annotations

from PySide6.QtCore import Qt
from PySide6.QtGui import QKeyEvent
from PySide6.QtWidgets import (
    QDialog,
    QHBoxLayout,
    QLabel,
    QPushButton,
    QVBoxLayout,
    QWidget,
)

from app.i18n import t


class ConfirmationModal(QDialog):
    """High-visibility modal with massive buttons."""

    def __init__(self, action_message: str, parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self.setWindowTitle(t("confirm.title"))
        self.setModal(True)
        self.setMinimumSize(720, 420)

        title = QLabel(t("confirm.title"))
        title.setObjectName("TitleLabel")
        title.setAlignment(Qt.AlignmentFlag.AlignCenter)

        body_text = t("confirm.body", action=action_message)
        body = QLabel(body_text)
        body.setWordWrap(True)
        body.setAlignment(Qt.AlignmentFlag.AlignCenter)

        self._yes = QPushButton(t("confirm.yes"))
        self._yes.setObjectName("YesButton")
        self._yes.setDefault(True)
        self._yes.setAutoDefault(True)
        self._yes.setAccessibleName(t("confirm.yes"))

        self._no = QPushButton(t("confirm.no"))
        self._no.setObjectName("NoButton")
        self._no.setAccessibleName(t("confirm.no"))

        self._yes.clicked.connect(self.accept)
        self._no.clicked.connect(self.reject)

        button_row = QHBoxLayout()
        button_row.setSpacing(28)
        button_row.addWidget(self._yes, stretch=1)
        button_row.addWidget(self._no, stretch=1)

        layout = QVBoxLayout(self)
        layout.setContentsMargins(40, 40, 40, 40)
        layout.setSpacing(24)
        layout.addWidget(title)
        layout.addWidget(body)
        layout.addStretch(1)
        layout.addLayout(button_row)

    def keyPressEvent(self, event: QKeyEvent) -> None:  # noqa: N802
        key = event.key()
        if key in (Qt.Key.Key_Y, Qt.Key.Key_Enter, Qt.Key.Key_Return):
            self.accept()
            return
        if key in (Qt.Key.Key_N, Qt.Key.Key_Escape):
            self.reject()
            return
        super().keyPressEvent(event)
