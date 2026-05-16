"""Tests for the template registry and JSON loading."""

from __future__ import annotations

from pathlib import Path

from app.orchestration.templates import TemplateRegistry, default_registry

TEMPLATES_DIR = Path(__file__).resolve().parent.parent / "app" / "prompts" / "templates"


def test_default_registry_includes_all_phase3_intents() -> None:
    registry = default_registry()
    expected = {
        "open_folder",
        "move_file",
        "delete_file",
        "open_browser",
        "search_web",
        "open_url",
        "compose_email",
        "read_inbox",
        "reply_message",
        "open_word_document",
        "print_document",
    }
    assert set(registry.known_intents()) >= expected


def test_json_dir_loads_explorer_template() -> None:
    registry = TemplateRegistry.from_json_dir(TEMPLATES_DIR)
    template = registry.get("open_folder")
    assert template is not None
    assert template.expected_window == "Explorer"
    assert template.steps[0].action == "double_click"


def test_json_dir_keeps_defaults_when_root_missing(tmp_path: Path) -> None:
    registry = TemplateRegistry.from_json_dir(tmp_path / "missing")
    assert registry.get("open_folder") is not None


def test_json_loaded_templates_cover_all_apps() -> None:
    registry = TemplateRegistry.from_json_dir(TEMPLATES_DIR)
    assert registry.get("compose_email") is not None
    assert registry.get("search_web") is not None
    assert registry.get("print_document") is not None
