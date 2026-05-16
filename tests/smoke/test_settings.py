"""Smoke tests for application settings."""

from __future__ import annotations


def test_settings_defaults_match_env_example(fake_env) -> None:
    from app.config import get_settings

    fake_env()
    s = get_settings()

    assert s.OLLAMA_HOST == "http://localhost:11434"
    assert s.LLM_MODEL == "qwen2.5:3b"
    assert s.UI_LANGUAGE in {"es", "en"}
    assert 0.1 <= s.OVERLAY_OPACITY <= 1.0
    assert s.OVERLAY_FPS >= 10


def test_settings_reads_overrides(fake_env) -> None:
    fake_env(LLM_MODEL="qwen2.5:7b", UI_LANGUAGE="en", OVERLAY_FPS="60")

    from app.config import get_settings

    s = get_settings()

    assert s.LLM_MODEL == "qwen2.5:7b"
    assert s.UI_LANGUAGE == "en"
    assert s.OVERLAY_FPS == 60


def test_i18n_lookup_falls_back_to_spanish(fake_env) -> None:
    fake_env(UI_LANGUAGE="en")
    from app.i18n import t

    assert "Roota" in t("app.title")
    assert "today" in t("main.greeting").lower()


def test_i18n_format_kwargs() -> None:
    from app.i18n import t

    msg = t("feedback.step_label", lang="es", step=1, total=3)
    assert "1" in msg and "3" in msg
