"""
Frameless, click-through, always-on-top overlay window.

Phase 1 renders a single pulsing circle plus optional label at the
target coordinates. Multiple anchors are supported but Roota's UX
principle ("present exactly one action at a time") means we usually
draw just one.
"""

from __future__ import annotations

import math
import sys
from typing import Iterable

from PySide6.QtCore import QPointF, QRectF, Qt, QTimer
from PySide6.QtGui import QColor, QFont, QGuiApplication, QPainter, QPaintEvent, QPen
from PySide6.QtWidgets import QWidget

from app.config import get_settings
from app.overlay.shapes import Anchor, AnchorStyle


class OverlayWindow(QWidget):
    """A transparent fullscreen widget that can never receive input."""

    def __init__(self) -> None:
        flags = (
            Qt.WindowType.FramelessWindowHint
            | Qt.WindowType.WindowStaysOnTopHint
            | Qt.WindowType.Tool
            | Qt.WindowType.WindowTransparentForInput
            | Qt.WindowType.NoDropShadowWindowHint
        )
        super().__init__(None, flags)
        self.setAttribute(Qt.WidgetAttribute.WA_TranslucentBackground, True)
        self.setAttribute(Qt.WidgetAttribute.WA_TransparentForMouseEvents, True)
        self.setAttribute(Qt.WidgetAttribute.WA_ShowWithoutActivating, True)

        screen = QGuiApplication.primaryScreen()
        if screen is not None:
            self.setGeometry(screen.geometry())

        self._anchors: tuple[Anchor, ...] = ()
        self._tick = 0.0
        self._timer = QTimer(self)
        self._timer.timeout.connect(self._on_tick)
        fps = get_settings().OVERLAY_FPS
        self._timer.start(max(1, 1000 // fps))

    def set_anchors(self, anchors: Iterable[Anchor]) -> None:
        self._anchors = tuple(anchors)
        self.update()

    def clear(self) -> None:
        self._anchors = ()
        self.update()

    def _on_tick(self) -> None:
        self._tick = (self._tick + 0.05) % (2 * math.pi)
        if self._anchors:
            self.update()

    def paintEvent(self, event: QPaintEvent) -> None:  # noqa: N802 - Qt API
        if not self._anchors:
            return
        painter = QPainter(self)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing, True)
        opacity = get_settings().OVERLAY_OPACITY
        for anchor in self._anchors:
            self._draw_anchor(painter, anchor, opacity)
        painter.end()

    def _draw_anchor(self, painter: QPainter, anchor: Anchor, opacity: float) -> None:
        pulse = (math.sin(self._tick * 2) + 1) / 2  # 0..1
        radius = anchor.radius + int(pulse * 12)
        if anchor.style is AnchorStyle.PULSE:
            color = QColor("#FFD166")
        elif anchor.style is AnchorStyle.HIGHLIGHT:
            color = QColor("#06D6A0")
        else:
            color = QColor("#118AB2")

        ring = QColor(color)
        ring.setAlphaF(0.85 * opacity)
        pen = QPen(ring, 6)
        painter.setPen(pen)
        painter.setBrush(Qt.BrushStyle.NoBrush)
        painter.drawEllipse(QPointF(anchor.x, anchor.y), radius, radius)

        glow = QColor(color)
        glow.setAlphaF(0.15 * opacity * (1 - pulse))
        painter.setBrush(glow)
        painter.setPen(Qt.PenStyle.NoPen)
        painter.drawEllipse(QPointF(anchor.x, anchor.y), radius + 24, radius + 24)

        if anchor.label:
            painter.setPen(QPen(QColor("#0B1F3A"), 1))
            font = QFont("Segoe UI", 14, QFont.Weight.DemiBold)
            painter.setFont(font)
            text_rect = QRectF(anchor.x - 220, anchor.y + radius + 12, 440, 56)
            bg = QColor("#FFF8E7")
            bg.setAlphaF(0.95 * opacity)
            painter.setBrush(bg)
            painter.drawRoundedRect(text_rect, 12, 12)
            painter.drawText(text_rect, int(Qt.AlignmentFlag.AlignCenter), anchor.label)


def _demo() -> int:  # pragma: no cover - manual entrypoint
    """Run a 5-second smoke demo of the overlay."""
    from PySide6.QtWidgets import QApplication

    app = QApplication.instance() or QApplication(sys.argv)
    overlay = OverlayWindow()
    overlay.show()
    overlay.set_anchors([Anchor(x=500, y=500, label="Demo anchor", radius=64)])
    QTimer.singleShot(5000, app.quit)
    return app.exec()


if __name__ == "__main__":  # pragma: no cover
    sys.exit(_demo())
