"""
accessibility — read the live UI tree of the active desktop.

The package exposes:
- `UIElement`: simple coordinate + label record.
- `UISnapshot`: ordered collection of elements + active window metadata.
- `AccessibilityScanner`: Protocol implemented by the platform backend.
- `get_scanner()`: factory that returns `WindowsScanner` on win32 and
  `StubScanner` everywhere else (so tests stay platform-agnostic).
"""

from app.accessibility.element import UIElement, UISnapshot
from app.accessibility.scanner import AccessibilityScanner, get_scanner
from app.accessibility.stub_scanner import StubScanner

__all__ = [
    "AccessibilityScanner",
    "StubScanner",
    "UIElement",
    "UISnapshot",
    "get_scanner",
]
