"""
Thin façade over the overlay window so orchestrator code never has to
know about Qt geometry. Exposes a tiny imperative API:
`show_anchor(x, y, label)` and `clear()`.
"""

from __future__ import annotations

from app.overlay.shapes import Anchor, AnchorStyle


class OverlayController:
    """Drive an `OverlayWindow` from non-Qt code paths."""

    def __init__(self, window: object | None = None) -> None:
        self._window = window

    def attach(self, window: object) -> None:
        self._window = window

    def show_anchor(
        self,
        x: int,
        y: int,
        label: str = "",
        *,
        style: AnchorStyle = AnchorStyle.PULSE,
        radius: int = 48,
    ) -> None:
        if self._window is None:
            return
        self._window.set_anchors([Anchor(x=x, y=y, label=label, style=style, radius=radius)])  # type: ignore[attr-defined]

    def clear(self) -> None:
        if self._window is None:
            return
        self._window.clear()  # type: ignore[attr-defined]
