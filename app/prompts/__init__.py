"""
prompts — versioned prompt templates and few-shot examples.

The Python helpers here load text files and apply simple `str.format`
substitution so prompt edits stay pure text and reviewable in Git.
"""

from __future__ import annotations

from pathlib import Path

_PROMPTS_DIR = Path(__file__).parent


def load_prompt(name: str) -> str:
    """Load a `.txt` prompt by name (no extension)."""
    path = _PROMPTS_DIR / f"{name}.txt"
    return path.read_text(encoding="utf-8")


def render_prompt(name: str, **kwargs: object) -> str:
    """Load a prompt and apply `str.format` with the supplied kwargs."""
    return load_prompt(name).format(**kwargs)


__all__ = ["load_prompt", "render_prompt"]
