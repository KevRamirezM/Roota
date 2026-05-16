"""
StateDetector — heuristic completion check for the current step.

The PRD's State Detection Engine (section 8.8) frames completion as a
delta between two snapshots. We implement three heuristics that work
reliably with pywinauto's UIA snapshots:

- target disappeared → assume the user clicked it and it's now in a
  different state (active item, opened folder).
- foreground window changed → user navigated somewhere else.
- snapshot is otherwise unchanged → step still pending.
"""

from __future__ import annotations

from dataclasses import dataclass

from app.accessibility.element import UISnapshot
from app.state.session import GuideStep


@dataclass(frozen=True)
class StepCompletion:
    completed: bool
    reason: str


class StateDetector:
    """Compares before/after snapshots and decides if the step is done."""

    def is_completed(
        self,
        step: GuideStep,
        before: UISnapshot,
        after: UISnapshot,
    ) -> StepCompletion:
        if before.window != after.window and after.window:
            return StepCompletion(True, f"window changed → {after.window}")

        target_before = before.find(step.target_text) if step.target_text else None
        target_after = after.find(step.target_text) if step.target_text else None

        if target_before is not None and target_after is None:
            return StepCompletion(True, f"target {step.target_text!r} disappeared")

        # New element with same text in a different window also counts.
        if target_after is not None and target_before is not None:
            if target_after.window and target_before.window != target_after.window:
                return StepCompletion(True, "target moved to a new window")

        return StepCompletion(False, "no significant change")
