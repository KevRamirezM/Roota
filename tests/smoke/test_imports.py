"""Imports every package to catch syntax / import-time errors quickly."""

from __future__ import annotations

import importlib

import pytest

MODULES = [
    "app",
    "app.config",
    "app.config.settings",
    "app.telemetry",
    "app.telemetry.logger",
    "app.i18n",
    "app.i18n.es",
    "app.i18n.en",
    "app.safety",
    "app.safety.guard",
    "app.llm",
    "app.llm.client",
    "app.llm.persona",
    "app.llm.stub_client",
    "app.llm.ollama_client",
    "app.llm.resilient",
    "app.prompts",
    "app.accessibility",
    "app.accessibility.element",
    "app.accessibility.scanner",
    "app.accessibility.stub_scanner",
    "app.state",
    "app.state.session",
    "app.orchestration",
    "app.orchestration.templates",
    "app.orchestration.intent",
    "app.orchestration.decision",
    "app.orchestration.state_detector",
    "app.orchestration.orchestrator",
    "app.overlay",
    "app.overlay.shapes",
    "app.overlay.controller",
    "app.ui.theme",
    "app.ui.feedback_panel",
    "app.ui.confirmation_modal",
    "app.ui.main_window",
    "app.ui.main",
    "app.orchestration.worker",
    "app.voice",
    "app.voice.tts",
    "app.voice.stt",
    "app.voice.recorder",
]


@pytest.mark.parametrize("name", MODULES)
def test_module_imports(name: str) -> None:
    importlib.import_module(name)
