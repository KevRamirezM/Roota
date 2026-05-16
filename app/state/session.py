"""
Session-level state objects.

`Intent` is the LLM's classification of the user's utterance.
`GuideStep` is one piece of guidance currently being shown.
`SessionState` stitches them together with completion flags.
`SessionStore` is the in-memory holder consumed by the orchestrator.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Literal

ActionVerb = Literal[
    "click",
    "double_click",
    "right_click",
    "type",
    "locate",
]


@dataclass(frozen=True)
class Intent:
    """LLM-classified user objective."""

    intent: str
    target: str
    params: dict[str, str] = field(default_factory=dict)
    raw_utterance: str = ""

    def is_known(self) -> bool:
        return self.intent != "unknown"


@dataclass(frozen=True)
class GuideStep:
    """A single piece of guidance to render to the user."""

    index: int
    total: int
    action: ActionVerb
    target_text: str
    instruction: str
    anchor_xy: tuple[int, int] | None = None


@dataclass
class SessionState:
    """Mutable state for one active guidance task."""

    goal: str = ""
    intent: Intent | None = None
    step_index: int = 0
    total_steps: int = 0
    completed: bool = False
    history: list[GuideStep] = field(default_factory=list)

    def begin(self, intent: Intent, total_steps: int) -> None:
        self.intent = intent
        self.goal = intent.intent
        self.step_index = 0
        self.total_steps = total_steps
        self.completed = False
        self.history.clear()

    def record(self, step: GuideStep) -> None:
        self.history.append(step)
        self.step_index = step.index

    def advance(self) -> None:
        if self.step_index < self.total_steps:
            self.step_index += 1
        if self.step_index >= self.total_steps:
            self.completed = True

    def reset(self) -> None:
        self.goal = ""
        self.intent = None
        self.step_index = 0
        self.total_steps = 0
        self.completed = False
        self.history.clear()


class SessionStore:
    """Single-session in-memory holder. Roota is a single-user assistant."""

    def __init__(self) -> None:
        self._state = SessionState()

    @property
    def state(self) -> SessionState:
        return self._state

    def replace(self, state: SessionState) -> None:
        self._state = state

    def reset(self) -> None:
        self._state.reset()
