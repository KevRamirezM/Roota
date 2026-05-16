"""
orchestration — the routing brain that ties LLM, accessibility, state
and overlay together. Pure Python; the Qt-aware bits live in
`app.orchestration.orchestrator`.
"""

from app.orchestration.decision import DecisionEngine
from app.orchestration.intent import IntentRecognitionError, IntentRecognizer
from app.orchestration.state_detector import StateDetector, StepCompletion
from app.orchestration.templates import (
    GuidanceTemplate,
    StepBlueprint,
    TemplateRegistry,
    default_registry,
)

__all__ = [
    "DecisionEngine",
    "GuidanceTemplate",
    "IntentRecognitionError",
    "IntentRecognizer",
    "StateDetector",
    "StepBlueprint",
    "StepCompletion",
    "TemplateRegistry",
    "default_registry",
]
