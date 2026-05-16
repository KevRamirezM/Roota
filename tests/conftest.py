"""Shared pytest fixtures."""

from __future__ import annotations

from pathlib import Path

import pytest


@pytest.fixture(autouse=True)
def _isolated_logs(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    """Route logs into the test's tmp dir so we never touch the real `logs/`."""
    monkeypatch.setenv("LOG_DIR", str(tmp_path / "logs"))
    monkeypatch.setenv("LOG_LEVEL", "WARNING")
    from app.config import get_settings

    get_settings.cache_clear()


@pytest.fixture
def fake_env(monkeypatch: pytest.MonkeyPatch):
    """Helper to set arbitrary env vars and clear the settings cache."""
    from app.config import get_settings

    def _set(**values: str) -> None:
        for key, value in values.items():
            monkeypatch.setenv(key, value)
        get_settings.cache_clear()

    yield _set
    get_settings.cache_clear()
