"""Tests for the deterministic stub LLM client."""

from __future__ import annotations

import pytest

from app.llm.stub_client import StubLLMClient


@pytest.fixture
def stub() -> StubLLMClient:
    return StubLLMClient()


@pytest.mark.parametrize(
    "utterance,expected_intent,expected_target",
    [
        ("Abre la carpeta de Descargas", "open_folder", "Downloads"),
        ("Open my Downloads please", "open_folder", "Downloads"),
        ("Quiero escribir un correo para mi hija Elena", "compose_email", "Elena"),
        ("Borra esta foto vieja", "delete_file", "selected_file"),
        ("Abre Chrome", "open_browser", "Chrome"),
        ("Buscar pronóstico del clima", "search_web", "Chrome"),
        ("Abre Word", "open_word_document", ""),
    ],
)
def test_stub_intents(stub: StubLLMClient, utterance: str, expected_intent: str, expected_target: str) -> None:
    result = stub.complete_json(utterance)

    assert result["intent"] == expected_intent
    assert result["target"] == expected_target


def test_stub_unknown_intent(stub: StubLLMClient) -> None:
    result = stub.complete_json("foo bar baz qux")
    assert result["intent"] == "unknown"


def test_stub_health_check_always_ok(stub: StubLLMClient) -> None:
    assert stub.health_check() is True


def test_stub_complete_text_returns_friendly_string(stub: StubLLMClient) -> None:
    out = stub.complete_text("hola")
    assert isinstance(out, str)
    assert len(out) > 0


def test_factory_force_stub() -> None:
    from app.llm import get_llm_client, reset_llm_cache

    reset_llm_cache()
    client = get_llm_client(force="stub")
    assert client.name == "stub"
    reset_llm_cache()
