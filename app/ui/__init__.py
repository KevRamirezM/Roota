"""
ui — accessible PySide6 user-facing surface.

Exposes the entry point `main()` (also wired via the `roota` console
script in `pyproject.toml`).
"""

from app.ui.main import main

__all__ = ["main"]
