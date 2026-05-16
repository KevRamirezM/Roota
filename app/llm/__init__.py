"""
llm — On-device LLM management and inference interfaces.

`get_llm_client()` probes the local Ollama daemon and falls back to a
deterministic stub so the app always has a working brain.
"""

from __future__ import annotations

from app.llm.client import LLMClient
from app.llm.ollama_client import OllamaClient
from app.llm.persona import build_system_prompt
from app.llm.resilient import ResilientLLMClient
from app.llm.stub_client import StubLLMClient
from app.telemetry import get_logger

_logger = get_logger("app.llm")
_cached_client: LLMClient | None = None


def get_llm_client(force: str | None = None) -> LLMClient:
    """Return a cached LLM client, picking Ollama if reachable, else stub.

    The returned client is a ResilientLLMClient that prefers Ollama and
    transparently degrades to the stub on any per-call error.
    """
    global _cached_client
    if force == "stub":
        return StubLLMClient()
    if force == "ollama":
        return OllamaClient()
    if _cached_client is not None:
        return _cached_client
    primary = OllamaClient()
    fallback = StubLLMClient()
    if primary.health_check():
        _logger.info("Using Ollama as primary with stub fallback")
        _cached_client = ResilientLLMClient(primary, fallback)
    else:
        _logger.warning("Ollama unreachable; using deterministic stub")
        _cached_client = fallback
    return _cached_client


def reset_llm_cache() -> None:
    """Test helper: drop the cached client."""
    global _cached_client
    _cached_client = None


__all__ = [
    "LLMClient",
    "OllamaClient",
    "ResilientLLMClient",
    "StubLLMClient",
    "build_system_prompt",
    "get_llm_client",
    "reset_llm_cache",
]
