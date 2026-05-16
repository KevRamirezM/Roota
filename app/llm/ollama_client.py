"""
Real Ollama-backed LLM client.

Wraps the official `ollama` python package so the rest of the app
sees the standard `LLMClient` Protocol regardless of backend.
"""

from __future__ import annotations

import json
from typing import Any

from app.config import get_settings
from app.telemetry import get_logger

logger = get_logger("app.llm.ollama")


class OllamaClient:
    """Talk to the local Ollama daemon. Never reaches the network."""

    name = "ollama"

    def __init__(
        self,
        *,
        host: str | None = None,
        model: str | None = None,
        temperature: float | None = None,
        max_tokens: int | None = None,
        timeout: float | None = None,
    ) -> None:
        settings = get_settings()
        self._host = host or settings.OLLAMA_HOST
        self._model = model or settings.LLM_MODEL
        self._temperature = settings.LLM_TEMPERATURE if temperature is None else temperature
        self._max_tokens = settings.LLM_MAX_TOKENS if max_tokens is None else max_tokens
        self._timeout = settings.LLM_TIMEOUT_SECONDS if timeout is None else timeout
        self._client: Any | None = None

    def _get_client(self) -> Any:
        if self._client is None:
            import ollama

            self._client = ollama.Client(host=self._host, timeout=self._timeout)
        return self._client

    def health_check(self) -> bool:
        try:
            client = self._get_client()
            tags = client.list()
            models = tags.get("models", []) if isinstance(tags, dict) else getattr(tags, "models", [])
            available = [m.get("name") if isinstance(m, dict) else getattr(m, "model", None) for m in models]
            ok = any(self._model in (n or "") for n in available)
            if not ok:
                logger.warning(f"Ollama up but model {self._model!r} not pulled. Found: {available}")
            return ok
        except Exception as exc:  # pragma: no cover - depends on local service
            logger.warning(f"Ollama health check failed: {exc!r}")
            return False

    def complete_text(self, prompt: str, *, system: str | None = None) -> str:
        client = self._get_client()
        response = client.chat(
            model=self._model,
            messages=self._build_messages(prompt, system),
            options={
                "temperature": self._temperature,
                "num_predict": self._max_tokens,
            },
        )
        return self._extract_message(response).strip()

    def complete_json(
        self,
        prompt: str,
        *,
        system: str | None = None,
        schema_hint: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        client = self._get_client()
        response = client.chat(
            model=self._model,
            messages=self._build_messages(prompt, system),
            format="json",
            options={
                "temperature": self._temperature,
                "num_predict": self._max_tokens,
            },
        )
        raw = self._extract_message(response).strip()
        try:
            value = json.loads(raw)
        except json.JSONDecodeError as exc:
            logger.warning(f"Ollama returned non-JSON despite format=json: {raw!r}")
            raise ValueError(f"Model returned invalid JSON: {raw!r}") from exc
        if not isinstance(value, dict):
            raise ValueError(f"Model returned non-object JSON: {value!r}")
        return value

    @staticmethod
    def _build_messages(prompt: str, system: str | None) -> list[dict[str, str]]:
        messages: list[dict[str, str]] = []
        if system:
            messages.append({"role": "system", "content": system})
        messages.append({"role": "user", "content": prompt})
        return messages

    @staticmethod
    def _extract_message(response: Any) -> str:
        if isinstance(response, dict):
            msg = response.get("message", {})
            return msg.get("content", "") if isinstance(msg, dict) else ""
        msg = getattr(response, "message", None)
        if msg is not None:
            return getattr(msg, "content", "") or ""
        return ""
