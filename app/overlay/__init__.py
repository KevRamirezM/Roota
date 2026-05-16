"""
overlay — frameless, always-on-top, click-through visual guidance plane.
"""

from app.overlay.controller import OverlayController
from app.overlay.shapes import Anchor, AnchorStyle
from app.overlay.window import OverlayWindow

__all__ = ["Anchor", "AnchorStyle", "OverlayController", "OverlayWindow"]
