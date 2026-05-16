"""
DecisionEngine — combines an `Intent`, a `GuidanceTemplate` and a fresh
`UISnapshot` into the next `GuideStep` to render.

The engine is pure: same inputs → same step. All side-effects (drawing,
speaking, scanning) live elsewhere.
"""

from __future__ import annotations

from app.accessibility.element import UIElement, UISnapshot
from app.i18n import t
from app.orchestration.templates import GuidanceTemplate, StepBlueprint
from app.safety import GuideAction, SafetyGuard
from app.state.session import GuideStep, Intent, SessionState


class StepResolutionError(RuntimeError):
    """Raised when the orchestrator cannot resolve the next step."""


class DecisionEngine:
    """Compute the next `GuideStep` for the session."""

    def __init__(self, *, safety: SafetyGuard | None = None) -> None:
        self._safety = safety or SafetyGuard()

    def next_step(
        self,
        intent: Intent,
        template: GuidanceTemplate,
        snapshot: UISnapshot,
        session: SessionState,
    ) -> GuideStep:
        if session.step_index >= len(template.steps):
            raise StepResolutionError("No more steps in template")

        blueprint = template.steps[session.step_index]
        target_text = self._materialise_target(blueprint.target_query, intent)

        element = self._find_element(snapshot, target_text, blueprint, template)
        anchor = element.center if element is not None else None

        instruction = t(blueprint.instruction_key, target=target_text)

        # Safety: render-only actions. SafetyGuard rejects automating ones.
        self._safety.review(
            GuideAction(type="anchor", target=target_text, payload=instruction)
        )

        return GuideStep(
            index=session.step_index + 1,
            total=len(template.steps),
            action=blueprint.action,
            target_text=target_text,
            instruction=instruction,
            anchor_xy=anchor,
        )

    @staticmethod
    def _materialise_target(template_query: str, intent: Intent) -> str:
        try:
            return template_query.format(target=intent.target, **intent.params)
        except KeyError:
            return template_query

    @staticmethod
    def _find_element(
        snapshot: UISnapshot,
        target_text: str,
        blueprint: StepBlueprint,
        template: GuidanceTemplate,
    ) -> UIElement | None:
        if not target_text:
            return None
        primary = snapshot.find(target_text)
        if primary is not None:
            return primary
        # Try the literal target_query word too (e.g. "Redactar")
        if blueprint.target_query and blueprint.target_query != target_text:
            secondary = snapshot.find(blueprint.target_query)
            if secondary is not None:
                return secondary
        # Try each token (handles "Mi hija Elena" → match on "Elena")
        for token in target_text.split():
            if len(token) <= 2:
                continue
            match = snapshot.find(token)
            if match is not None:
                return match
        return None
