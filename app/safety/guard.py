"""
Safety layer — the irrevocable runtime guarantee.

PRD section 8.9 mandates that Roota *never* directly controls input
hardware: no synthetic clicks, no synthetic keystrokes, no background
file automation. The guard is the single chokepoint every emitted
action passes through, so any leak shows up here loudly instead of
silently driving the user's machine.

Treat this module as security-critical. Add new ALLOWED actions only
after a written design review.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Final, Literal

ActionType = Literal[
    "highlight",
    "anchor",
    "arrow",
    "speak",
    "show_text",
    "scan",
    "click",
    "double_click",
    "right_click",
    "type_text",
    "key_press",
    "drag",
    "scroll",
    "file_op",
]

GUIDE_ACTIONS: Final[frozenset[ActionType]] = frozenset(
    {"highlight", "anchor", "arrow", "speak", "show_text", "scan"}
)
AUTOMATION_ACTIONS: Final[frozenset[ActionType]] = frozenset(
    {
        "click",
        "double_click",
        "right_click",
        "type_text",
        "key_press",
        "drag",
        "scroll",
        "file_op",
    }
)


@dataclass(frozen=True)
class GuideAction:
    """An action the orchestrator wants to surface to the user."""

    type: ActionType
    target: str | None = None
    payload: str | None = None


class UnsafeActionError(RuntimeError):
    """Raised when something tries to emit an automating action."""


class SafetyGuard:
    """Reject any action that would automate the user's input."""

    def __init__(self, *, strict: bool = True) -> None:
        self._strict = strict

    def review(self, action: GuideAction) -> GuideAction:
        """Return the action unchanged or raise `UnsafeActionError`."""
        if action.type in AUTOMATION_ACTIONS:
            raise UnsafeActionError(
                f"Refusing to emit automating action {action.type!r}; "
                "Roota only guides, it never executes."
            )
        if action.type not in GUIDE_ACTIONS:
            if self._strict:
                raise UnsafeActionError(
                    f"Unknown action type {action.type!r}; refusing under strict mode."
                )
        return action

    def is_safe(self, action: GuideAction) -> bool:
        """Predicate variant of `review` that does not raise."""
        try:
            self.review(action)
            return True
        except UnsafeActionError:
            return False
