"""
Resilient LLM client that prefers a primary backend (Ollama) but
transparently falls back to a secondary (stub) on per-call errors.

The hackathon laptop sometimes has too little RAM to load qwen2.5:3b
even when Ollama itself is up, so a one-shot health probe at boot is
not enough — we need to recover *during* a session too.
"""

from __future__ import annotations

from typing import Any

from app.llm.client import LLMClient
from app.telemetry import get_logger

logger = get_logger("app.llm.resilient")


class ResilientLLMClient:
    """Try the primary client, swap to fallback on error for that call."""

    name = "resilient"

    def __init__(self, primary: LLMClient, fallback: LLMClient) -> None:
        self._primary = primary
        self._fallback = fallback
        self._primary_healthy = True

    @property
    def active_backend(self) -> str:
        return self._primary.name if self._primary_healthy else self._fallback.name

    def health_check(self) -> bool:
        return self._primary.health_check() or self._fallback.health_check()

    def complete_text(self, prompt: str, *, system: str | None = None) -> str:
        if self._primary_healthy:
            try:
                return self._primary.complete_text(prompt, system=system)
            except Exception as exc:
                logger.warning(f"Primary LLM failed ({exc!r}); using fallback for text.")
                self._primary_healthy = False
        return self._fallback.complete_text(prompt, system=system)

    def complete_json(
        self,
        prompt: str,
        *,
        system: str | None = None,
        schema_hint: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        if self._primary_healthy:
            try:
                return self._primary.complete_json(prompt, system=system, schema_hint=schema_hint)
            except Exception as exc:
                logger.warning(f"Primary LLM failed ({exc!r}); using fallback for JSON.")
                self._primary_healthy = False
        return self._fallback.complete_json(prompt, system=system, schema_hint=schema_hint)

    def reset(self) -> None:
        """Allow retrying the primary backend (e.g. after RAM is freed)."""
        self._primary_healthy = True
