"""
Platform-aware scanner factory.

`AccessibilityScanner` is a Protocol; concrete backends live in
`stub_scanner` (always available) and `windows_scanner` (loaded only
on win32 to avoid pywinauto import errors elsewhere).
"""

from __future__ import annotations

import sys
from typing import Protocol, runtime_checkable

from app.accessibility.element import UISnapshot
from app.telemetry import get_logger

logger = get_logger("app.accessibility")


@runtime_checkable
class AccessibilityScanner(Protocol):
    """Returns a snapshot of the current foreground window."""

    name: str

    def snapshot(self) -> UISnapshot:
        ...


def get_scanner(*, force: str | None = None) -> AccessibilityScanner:
    """Return the best scanner for the host OS."""
    from app.accessibility.stub_scanner import StubScanner

    if force == "stub":
        return StubScanner()
    if force == "windows" or (force is None and sys.platform == "win32"):
        try:
            from app.accessibility.windows_scanner import WindowsScanner

            return WindowsScanner()
        except Exception as exc:
            logger.warning(f"WindowsScanner unavailable ({exc!r}); using stub.")
            return StubScanner()
    return StubScanner()
