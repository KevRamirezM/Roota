"""
IntentRecognizer — utterance → `Intent` via the LLM, validated against
the registered template catalog.

The recognizer is intentionally forgiving: unknown intents return a
properly-formed `Intent(intent="unknown", ...)` rather than raising,
because the orchestrator handles that gracefully (friendly fallback
message). Only true contract violations (e.g. malformed JSON the
fallback can't recover) raise `IntentRecognitionError`.
"""

from __future__ import annotations

import json
from typing import Any

from app.llm.client import LLMClient
from app.llm.persona import build_system_prompt
from app.orchestration.templates import TemplateRegistry, default_registry
from app.prompts import render_prompt
from app.state.session import Intent
from app.telemetry import get_logger

logger = get_logger("app.orchestration.intent")


class IntentRecognitionError(RuntimeError):
    """Raised when the LLM returns something we cannot coerce into an Intent."""


class IntentRecognizer:
    """Converts free-text utterances into structured `Intent` records."""

    def __init__(
        self,
        llm: LLMClient,
        templates: TemplateRegistry | None = None,
    ) -> None:
        self._llm = llm
        self._templates = templates or default_registry()

    def recognise(self, utterance: str) -> Intent:
        utterance = utterance.strip()
        if not utterance:
            return Intent(intent="unknown", target="", raw_utterance="")

        prompt = render_prompt("intent_classifier", utterance=utterance)
        system = build_system_prompt()
        try:
            payload = self._llm.complete_json(prompt, system=system)
        except Exception as exc:
            logger.warning(f"LLM JSON failure ({exc!r}); falling back to unknown")
            return Intent(intent="unknown", target="", raw_utterance=utterance)

        return self._coerce(payload, utterance)

    def _coerce(self, payload: Any, utterance: str) -> Intent:
        if isinstance(payload, str):
            try:
                payload = json.loads(payload)
            except json.JSONDecodeError as exc:
                raise IntentRecognitionError(
                    f"Model returned non-JSON string for intent: {payload!r}"
                ) from exc
        if not isinstance(payload, dict):
            raise IntentRecognitionError(f"Intent payload is not an object: {payload!r}")

        raw_intent = str(payload.get("intent", "unknown")).strip().lower()
        target = str(payload.get("target", "")).strip()
        params_raw = payload.get("params", {})
        params: dict[str, str] = {}
        if isinstance(params_raw, dict):
            for k, v in params_raw.items():
                params[str(k)] = str(v)

        if self._templates.get(raw_intent) is None:
            logger.info(f"Intent {raw_intent!r} unknown to registry; marking unknown")
            raw_intent = "unknown"

        return Intent(intent=raw_intent, target=target, params=params, raw_utterance=utterance)
