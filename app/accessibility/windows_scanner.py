"""
Windows UI Automation scanner backed by pywinauto.

Only imported on `sys.platform == "win32"`. We deliberately keep the
surface small: scan the *foreground* window's children once and turn
them into `UIElement` records. No mouse, no keystrokes, no clicks.
"""

from __future__ import annotations

import sys
from typing import Any

from app.accessibility.element import UIElement, UISnapshot
from app.telemetry import get_logger

if sys.platform != "win32":  # pragma: no cover - guarded
    raise ImportError("WindowsScanner is only available on Windows.")

logger = get_logger("app.accessibility.windows")


_INTERESTING_TYPES = {
    "Button",
    "MenuItem",
    "ListItem",
    "TreeItem",
    "TabItem",
    "Hyperlink",
    "Edit",
    "Document",
    "ComboBox",
    "CheckBox",
    "RadioButton",
}


class WindowsScanner:
    """Snapshot the active foreground window using pywinauto's UIA backend."""

    name = "windows"

    def __init__(self) -> None:
        try:
            from pywinauto import Desktop  # noqa: F401  -- presence check
        except Exception as exc:  # pragma: no cover - install issue
            raise ImportError(f"pywinauto not available: {exc!r}") from exc

    def snapshot(self) -> UISnapshot:
        try:
            from pywinauto import Desktop
        except Exception as exc:  # pragma: no cover - guarded by __init__
            logger.warning(f"pywinauto import failed: {exc!r}")
            return UISnapshot(window="", elements=())

        try:
            desktop = Desktop(backend="uia")
            window = desktop.windows(active_only=True, visible_only=True, top_level_only=True)
            if not window:
                logger.info("No active foreground window")
                return UISnapshot(window="", elements=())
            top = window[0]
            window_title = top.window_text() or ""
            elements: list[UIElement] = []
            for child in self._safe_descendants(top):
                el = self._to_element(child, window_title)
                if el is not None:
                    elements.append(el)
            return UISnapshot(window=window_title, elements=tuple(elements))
        except Exception as exc:
            logger.warning(f"Windows scan failed: {exc!r}")
            return UISnapshot(window="", elements=())

    @staticmethod
    def _safe_descendants(window: Any) -> list[Any]:
        try:
            return list(window.descendants())
        except Exception as exc:
            logger.debug(f"descendants() failed: {exc!r}")
            return []

    @staticmethod
    def _to_element(node: Any, window_title: str) -> UIElement | None:
        try:
            ctrl_type = node.element_info.control_type or ""
        except Exception:
            ctrl_type = ""
        if ctrl_type and ctrl_type not in _INTERESTING_TYPES:
            return None
        try:
            text = (node.window_text() or "").strip()
        except Exception:
            text = ""
        if not text:
            return None
        try:
            rect = node.rectangle()
        except Exception:
            return None
        try:
            automation_id = node.element_info.automation_id or None
        except Exception:
            automation_id = None
        width = max(0, int(rect.right) - int(rect.left))
        height = max(0, int(rect.bottom) - int(rect.top))
        if width == 0 or height == 0:
            return None
        return UIElement(
            type=ctrl_type or "Control",
            text=text,
            x=int(rect.left),
            y=int(rect.top),
            width=width,
            height=height,
            automation_id=automation_id,
            window=window_title,
        )
