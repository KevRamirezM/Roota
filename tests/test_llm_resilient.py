"""Tests for the resilient LLM wrapper."""

from __future__ import annotations

from typing import Any

from app.llm.resilient import ResilientLLMClient
from app.llm.stub_client import StubLLMClient


class _FailingClient:
    name = "failing"

    def health_check(self) -> bool:
        return True

    def complete_text(self, prompt: str, *, system: str | None = None) -> str:
        raise RuntimeError("ran out of memory")

    def complete_json(
        self,
        prompt: str,
        *,
        system: str | None = None,
        schema_hint: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        raise RuntimeError("ran out of memory")


class _WorkingClient:
    name = "working"

    def __init__(self) -> None:
        self.calls = 0

    def health_check(self) -> bool:
        return True

    def complete_text(self, prompt: str, *, system: str | None = None) -> str:
        self.calls += 1
        return f"primary:{prompt}"

    def complete_json(
        self,
        prompt: str,
        *,
        system: str | None = None,
        schema_hint: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        self.calls += 1
        return {"intent": "ok", "target": "primary", "params": {}}


def test_uses_primary_when_healthy() -> None:
    primary = _WorkingClient()
    fallback = StubLLMClient()
    client = ResilientLLMClient(primary, fallback)

    out = client.complete_text("hi")
    assert out.startswith("primary:")
    assert primary.calls == 1
    assert client.active_backend == "working"


def test_falls_back_on_primary_error() -> None:
    primary = _FailingClient()
    fallback = StubLLMClient()
    client = ResilientLLMClient(primary, fallback)

    text = client.complete_text("Abre Descargas")
    assert isinstance(text, str)
    assert client.active_backend == "stub"


def test_falls_back_on_json_error() -> None:
    primary = _FailingClient()
    fallback = StubLLMClient()
    client = ResilientLLMClient(primary, fallback)

    payload = client.complete_json("Abre la carpeta de Descargas")
    assert payload["intent"] == "open_folder"
    assert client.active_backend == "stub"


def test_reset_re_enables_primary() -> None:
    primary = _FailingClient()
    fallback = StubLLMClient()
    client = ResilientLLMClient(primary, fallback)

    client.complete_text("trigger fallback")
    assert client.active_backend == "stub"
    client.reset()
    assert client.active_backend == "failing"
