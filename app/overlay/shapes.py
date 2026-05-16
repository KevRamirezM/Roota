"""
Pure-data overlay shape descriptors.

Kept free of Qt imports so we can unit-test geometry without a
QApplication. The window module reads these and renders them with
QPainter.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum


class AnchorStyle(Enum):
    PULSE = "pulse"
    HIGHLIGHT = "highlight"
    ARROW = "arrow"


@dataclass(frozen=True)
class Anchor:
    """A single visual marker drawn on the overlay."""

    x: int
    y: int
    label: str = ""
    radius: int = 48
    style: AnchorStyle = AnchorStyle.PULSE
