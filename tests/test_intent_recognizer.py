"""Tests for the IntentRecognizer."""

from __future__ import annotations

from typing import Any

from app.orchestration.intent import IntentRecognizer


class _ScriptedLLM:
    name = "scripted"

    def __init__(self, responses: dict[str, dict[str, Any]]) -> None:
        self._responses = responses

    def health_check(self) -> bool:
        return True

    def complete_text(self, prompt: str, *, system: str | None = None) -> str:
        return ""

    def complete_json(
        self,
        prompt: str,
        *,
        system: str | None = None,
        schema_hint: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        for utterance, payload in self._responses.items():
            if utterance.lower() in prompt.lower():
                return dict(payload)
        return {"intent": "unknown", "target": "", "params": {}}


def test_known_intent_resolves() -> None:
    llm = _ScriptedLLM(
        {
            "Abre Descargas": {"intent": "open_folder", "target": "Descargas", "params": {}},
        }
    )
    rec = IntentRecognizer(llm)

    intent = rec.recognise("Abre Descargas por favor")

    assert intent.intent == "open_folder"
    assert intent.target == "Descargas"
    assert intent.is_known() is True


def test_unregistered_intent_becomes_unknown() -> None:
    llm = _ScriptedLLM({"haz café": {"intent": "make_coffee", "target": "espresso", "params": {}}})
    rec = IntentRecognizer(llm)

    intent = rec.recognise("haz café")

    assert intent.intent == "unknown"


def test_empty_utterance_returns_unknown() -> None:
    llm = _ScriptedLLM({})
    rec = IntentRecognizer(llm)
    assert rec.recognise("   ").intent == "unknown"


def test_llm_failure_falls_back_to_unknown() -> None:
    class _Boom:
        name = "boom"

        def health_check(self) -> bool:
            return False

        def complete_text(self, *_: Any, **__: Any) -> str:
            raise RuntimeError("nope")

        def complete_json(self, *_: Any, **__: Any) -> dict[str, Any]:
            raise RuntimeError("nope")

    rec = IntentRecognizer(_Boom())
    intent = rec.recognise("Abre Descargas")
    assert intent.intent == "unknown"
    assert intent.raw_utterance == "Abre Descargas"


def test_params_coerced_to_strings() -> None:
    llm = _ScriptedLLM(
        {
            "search": {"intent": "search_web", "target": "clima", "params": {"limit": 5}},
        }
    )
    rec = IntentRecognizer(llm)
    intent = rec.recognise("search clima")
    assert intent.params == {"limit": "5"}
