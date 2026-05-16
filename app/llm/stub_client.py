"""
Deterministic offline fallback LLM client.

Picked automatically by `get_llm_client()` whenever Ollama isn't
reachable, so the app keeps working (with reduced fluency) instead
of hard-crashing during a demo.
"""

from __future__ import annotations

import re
from typing import Any

from app.i18n import t

_INTENT_RULES: list[tuple[re.Pattern[str], dict[str, Any]]] = [
    (
        re.compile(r"(abrir|abre|open).*(descarga|download)", re.I),
        {"intent": "open_folder", "target": "Downloads", "params": {}},
    ),
    (
        re.compile(r"(abrir|abre|open).*(documento|document)", re.I),
        {"intent": "open_folder", "target": "Documents", "params": {}},
    ),
    (
        re.compile(r"(mover|move).*(foto|photo|file|archivo)", re.I),
        {"intent": "move_file", "target": "selected_file", "params": {}},
    ),
    (
        re.compile(r"(borr|elimin|delete|remove)", re.I),
        {"intent": "delete_file", "target": "selected_file", "params": {}},
    ),
    (
        re.compile(r"(buscar|search|google).*", re.I),
        {"intent": "search_web", "target": "Chrome", "params": {}},
    ),
    (
        re.compile(r"(escrib|enviar|send|write|email|correo).*(elena|hija|hijo|son|daughter|amig)", re.I),
        {"intent": "compose_email", "target": "Elena", "params": {}},
    ),
    (
        re.compile(r"(correo|email|gmail)", re.I),
        {"intent": "compose_email", "target": "", "params": {}},
    ),
    (
        re.compile(r"(chrome|navegador|browser)", re.I),
        {"intent": "open_browser", "target": "Chrome", "params": {}},
    ),
    (
        re.compile(r"(word|documento|document)", re.I),
        {"intent": "open_word_document", "target": "", "params": {}},
    ),
]


class StubLLMClient:
    """Pattern-match local fallback. Slow to evolve, fast to run."""

    name = "stub"

    def health_check(self) -> bool:
        return True

    def complete_text(self, prompt: str, *, system: str | None = None) -> str:
        lowered = prompt.lower()
        if "hola" in lowered or "hello" in lowered:
            return t("main.greeting")
        if "perfecto" in lowered or "great" in lowered:
            return t("feedback.success_body")
        return t("intent.unknown")

    def complete_json(
        self,
        prompt: str,
        *,
        system: str | None = None,
        schema_hint: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        for pattern, payload in _INTENT_RULES:
            if pattern.search(prompt):
                return dict(payload)
        return {"intent": "unknown", "target": "", "params": {}}
