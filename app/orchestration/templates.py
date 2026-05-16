"""
Guidance templates — versioned, deterministic mappings of intents to
ordered step blueprints.

Phase 3 ships an in-code default registry. Phase 7 layers JSON loading
on top so non-developers can author new flows without touching Python.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path

from app.state.session import ActionVerb


@dataclass(frozen=True)
class StepBlueprint:
    """Recipe for one step of guidance."""

    action: ActionVerb
    target_query: str
    instruction_key: str = "guidance.click_target"
    fallback_window: str | None = None


@dataclass(frozen=True)
class GuidanceTemplate:
    """Recipe for guiding the user through one intent end-to-end."""

    intent: str
    confirmation_action_key: str
    steps: tuple[StepBlueprint, ...]
    expected_window: str | None = None
    aliases: tuple[str, ...] = field(default_factory=tuple)


_DEFAULT_TEMPLATES: tuple[GuidanceTemplate, ...] = (
    GuidanceTemplate(
        intent="open_folder",
        confirmation_action_key="confirm.open_folder",
        expected_window="Explorer",
        steps=(
            StepBlueprint(action="double_click", target_query="{target}", instruction_key="guidance.double_click_target", fallback_window="Explorer"),
        ),
    ),
    GuidanceTemplate(
        intent="move_file",
        confirmation_action_key="confirm.move_file",
        expected_window="Explorer",
        steps=(
            StepBlueprint(action="locate", target_query="{target}", instruction_key="guidance.locate_target"),
            StepBlueprint(action="right_click", target_query="{target}", instruction_key="guidance.right_click_target"),
            StepBlueprint(action="click", target_query="Cortar", instruction_key="guidance.click_target"),
        ),
    ),
    GuidanceTemplate(
        intent="delete_file",
        confirmation_action_key="confirm.delete_file",
        expected_window="Explorer",
        steps=(
            StepBlueprint(action="click", target_query="{target}", instruction_key="guidance.click_target"),
            StepBlueprint(action="locate", target_query="Suprimir", instruction_key="guidance.locate_target"),
        ),
    ),
    GuidanceTemplate(
        intent="open_browser",
        confirmation_action_key="confirm.open_browser",
        expected_window="Chrome",
        steps=(
            StepBlueprint(action="locate", target_query="Chrome", instruction_key="guidance.locate_target"),
        ),
    ),
    GuidanceTemplate(
        intent="search_web",
        confirmation_action_key="confirm.search_web",
        expected_window="Chrome",
        steps=(
            StepBlueprint(action="click", target_query="Nueva pestaña", instruction_key="guidance.click_target"),
            StepBlueprint(action="type", target_query="Buscar", instruction_key="guidance.type_in_target"),
        ),
    ),
    GuidanceTemplate(
        intent="open_url",
        confirmation_action_key="confirm.open_url",
        expected_window="Chrome",
        steps=(
            StepBlueprint(action="click", target_query="Nueva pestaña", instruction_key="guidance.click_target"),
            StepBlueprint(action="type", target_query="Buscar", instruction_key="guidance.type_in_target"),
        ),
    ),
    GuidanceTemplate(
        intent="compose_email",
        confirmation_action_key="confirm.compose_email",
        expected_window="Gmail",
        steps=(
            StepBlueprint(action="click", target_query="Redactar", instruction_key="guidance.click_target"),
        ),
    ),
    GuidanceTemplate(
        intent="read_inbox",
        confirmation_action_key="confirm.read_inbox",
        expected_window="Gmail",
        steps=(
            StepBlueprint(action="click", target_query="Bandeja de entrada", instruction_key="guidance.click_target"),
        ),
    ),
    GuidanceTemplate(
        intent="reply_message",
        confirmation_action_key="confirm.reply_message",
        expected_window="Gmail",
        steps=(
            StepBlueprint(action="click", target_query="Responder", instruction_key="guidance.click_target"),
        ),
    ),
    GuidanceTemplate(
        intent="open_word_document",
        confirmation_action_key="confirm.open_word_document",
        expected_window="Word",
        steps=(
            StepBlueprint(action="locate", target_query="Word", instruction_key="guidance.locate_target"),
        ),
    ),
    GuidanceTemplate(
        intent="print_document",
        confirmation_action_key="confirm.print_document",
        expected_window="Word",
        steps=(
            StepBlueprint(action="click", target_query="Imprimir", instruction_key="guidance.click_target"),
        ),
    ),
)


class TemplateRegistry:
    """Lookup of intent name → GuidanceTemplate."""

    def __init__(self, templates: tuple[GuidanceTemplate, ...] = ()) -> None:
        self._by_intent: dict[str, GuidanceTemplate] = {}
        for tpl in templates:
            self.register(tpl)

    def register(self, template: GuidanceTemplate) -> None:
        self._by_intent[template.intent] = template

    def get(self, intent: str) -> GuidanceTemplate | None:
        return self._by_intent.get(intent)

    def known_intents(self) -> tuple[str, ...]:
        return tuple(sorted(self._by_intent.keys()))

    @classmethod
    def from_json_dir(cls, root: Path) -> "TemplateRegistry":
        """Load every `*.json` template under `root` and merge with defaults."""
        registry = default_registry()
        if not root.exists():
            return registry
        for path in sorted(root.glob("*.json")):
            try:
                payload = json.loads(path.read_text(encoding="utf-8"))
            except json.JSONDecodeError:
                continue
            for intent_name, body in payload.items():
                steps = tuple(
                    StepBlueprint(
                        action=step["action"],
                        target_query=step.get("target_query", "{target}"),
                        instruction_key=step.get("instruction_key", "guidance.click_target"),
                        fallback_window=step.get("fallback_window"),
                    )
                    for step in body.get("steps", [])
                )
                registry.register(
                    GuidanceTemplate(
                        intent=intent_name,
                        confirmation_action_key=body.get("confirmation_action_key", f"confirm.{intent_name}"),
                        expected_window=body.get("expected_window"),
                        steps=steps,
                        aliases=tuple(body.get("aliases", ())),
                    )
                )
        return registry


def default_registry() -> TemplateRegistry:
    """Return a fresh registry pre-populated with the in-code defaults."""
    return TemplateRegistry(_DEFAULT_TEMPLATES)
