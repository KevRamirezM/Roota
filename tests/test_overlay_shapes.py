"""Tests for overlay shape descriptors and the controller façade."""

from __future__ import annotations

from app.overlay.controller import OverlayController
from app.overlay.shapes import Anchor, AnchorStyle


class _FakeWindow:
    def __init__(self) -> None:
        self.anchors: list[Anchor] = []
        self.cleared = 0

    def set_anchors(self, anchors) -> None:  # type: ignore[no-untyped-def]
        self.anchors = list(anchors)

    def clear(self) -> None:
        self.cleared += 1


def test_show_anchor_passes_through() -> None:
    window = _FakeWindow()
    controller = OverlayController(window=window)
    controller.show_anchor(100, 200, "Descargas", style=AnchorStyle.HIGHLIGHT, radius=64)

    assert len(window.anchors) == 1
    a = window.anchors[0]
    assert (a.x, a.y) == (100, 200)
    assert a.label == "Descargas"
    assert a.style is AnchorStyle.HIGHLIGHT
    assert a.radius == 64


def test_clear_dispatched_to_window() -> None:
    window = _FakeWindow()
    OverlayController(window).clear()
    assert window.cleared == 1


def test_no_window_attached_is_safe() -> None:
    controller = OverlayController()
    controller.show_anchor(1, 1, "x")
    controller.clear()


def test_default_anchor_style_is_pulse() -> None:
    a = Anchor(x=0, y=0)
    assert a.style is AnchorStyle.PULSE
