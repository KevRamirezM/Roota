"""
Orchestrator — the Roota brain.

Pure-Python core (`Orchestrator`) is fully testable without Qt; the
Qt-aware adapter (`OrchestratorWorker`) lives below and re-emits the
same events as Qt signals so the UI can subscribe.
"""

from __future__ import annotations

import time
from dataclasses import dataclass
from typing import Callable

from app.accessibility.scanner import AccessibilityScanner
from app.i18n import t
from app.llm.client import LLMClient
from app.orchestration.decision import DecisionEngine, StepResolutionError
from app.orchestration.intent import IntentRecognizer
from app.orchestration.state_detector import StateDetector
from app.orchestration.templates import GuidanceTemplate, TemplateRegistry, default_registry
from app.safety import SafetyGuard
from app.state.session import GuideStep, Intent, SessionState, SessionStore
from app.telemetry import get_logger

logger = get_logger("app.orchestration.orchestrator")


@dataclass
class ConfirmationRequest:
    intent: Intent
    template: GuidanceTemplate
    message: str


@dataclass
class StepEvent:
    step: GuideStep


@dataclass
class GoalCompleteEvent:
    intent: Intent
    steps: int


@dataclass
class ErrorEvent:
    message: str


class Orchestrator:
    """Single-session brain that turns utterances into guide steps."""

    def __init__(
        self,
        *,
        llm: LLMClient,
        scanner: AccessibilityScanner,
        templates: TemplateRegistry | None = None,
        safety: SafetyGuard | None = None,
        store: SessionStore | None = None,
        recognizer: IntentRecognizer | None = None,
        decision: DecisionEngine | None = None,
        detector: StateDetector | None = None,
        sleep: Callable[[float], None] = time.sleep,
    ) -> None:
        self._llm = llm
        self._scanner = scanner
        self._templates = templates or default_registry()
        self._safety = safety or SafetyGuard()
        self._store = store or SessionStore()
        self._recognizer = recognizer or IntentRecognizer(llm, self._templates)
        self._decision = decision or DecisionEngine(safety=self._safety)
        self._detector = detector or StateDetector()
        self._sleep = sleep
        self._cancelled = False

    @property
    def session(self) -> SessionState:
        return self._store.state

    def cancel(self) -> None:
        self._cancelled = True

    def reset(self) -> None:
        self._cancelled = False
        self._store.reset()

    def classify(self, utterance: str) -> tuple[Intent, GuidanceTemplate | None]:
        """Recognise the user's intent and return its template if known."""
        intent = self._recognizer.recognise(utterance)
        template = self._templates.get(intent.intent) if intent.is_known() else None
        return intent, template

    def build_confirmation(
        self, intent: Intent, template: GuidanceTemplate
    ) -> ConfirmationRequest:
        target_label = intent.target or t("intent.no_target")
        message = t(template.confirmation_action_key, target=target_label)
        return ConfirmationRequest(intent=intent, template=template, message=message)

    def begin(self, intent: Intent, template: GuidanceTemplate) -> None:
        self._cancelled = False
        self._store.state.begin(intent, total_steps=len(template.steps))

    def next_step(self, template: GuidanceTemplate, intent: Intent) -> GuideStep:
        snapshot = self._scanner.snapshot()
        return self._decision.next_step(intent, template, snapshot, self._store.state)

    def confirm_step_completed(
        self,
        step: GuideStep,
        intent: Intent,
        template: GuidanceTemplate,
        *,
        max_polls: int = 60,
        poll_interval: float = 0.5,
    ) -> bool:
        """Poll the UI until the step looks completed or polls run out."""
        before = self._scanner.snapshot()
        for _ in range(max_polls):
            if self._cancelled:
                return False
            self._sleep(poll_interval)
            after = self._scanner.snapshot()
            outcome = self._detector.is_completed(step, before, after)
            if outcome.completed:
                logger.info(f"Step {step.index} completed: {outcome.reason}")
                self._store.state.advance()
                return True
            before = after
        return False

    def run(
        self,
        utterance: str,
        *,
        confirm: Callable[[ConfirmationRequest], bool],
        on_step: Callable[[GuideStep], None],
        on_error: Callable[[ErrorEvent], None],
        on_complete: Callable[[GoalCompleteEvent], None],
        max_polls: int = 60,
        poll_interval: float = 0.5,
    ) -> None:
        """Run the full classify → confirm → step-loop pipeline."""
        intent, template = self.classify(utterance)
        if template is None:
            on_error(ErrorEvent(message=t("intent.unknown")))
            return

        confirmation = self.build_confirmation(intent, template)
        if not confirm(confirmation):
            on_error(ErrorEvent(message=t("feedback.error_body", message=t("confirm.no"))))
            return

        self.begin(intent, template)
        while not self._store.state.completed and not self._cancelled:
            try:
                step = self.next_step(template, intent)
            except StepResolutionError as exc:
                on_error(ErrorEvent(message=str(exc)))
                return
            self._store.state.record(step)
            on_step(step)
            ok = self.confirm_step_completed(
                step,
                intent,
                template,
                max_polls=max_polls,
                poll_interval=poll_interval,
            )
            if not ok:
                on_error(ErrorEvent(message=t("guidance.element_not_found", target=step.target_text)))
                return

        on_complete(GoalCompleteEvent(intent=intent, steps=len(self._store.state.history)))
