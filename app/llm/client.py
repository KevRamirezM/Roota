"""
LLM client abstraction.

The orchestration layer talks to the LLM exclusively through the
`LLMClient` Protocol so we can swap the backend (real Ollama, offline
stub, future llama.cpp) without touching the business logic.
"""

from __future__ import annotations

from typing import Any, Protocol, runtime_checkable


@runtime_checkable
class LLMClient(Protocol):
    """Minimal surface every Roota LLM backend must implement."""

    name: str

    def health_check(self) -> bool:
        """Return True iff the backend is ready to accept calls."""
        ...

    def complete_text(self, prompt: str, *, system: str | None = None) -> str:
        """Single-turn text completion."""
        ...

    def complete_json(
        self,
        prompt: str,
        *,
        system: str | None = None,
        schema_hint: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        """Single-turn JSON completion. Implementations must always return a dict."""
        ...
