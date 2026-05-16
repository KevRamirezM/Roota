"""
state.session — Task session model and step tracker.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import Optional


class TaskStatus(str, Enum):
    PENDING = "pending"
    IN_PROGRESS = "in_progress"
    COMPLETED = "completed"
    CANCELLED = "cancelled"
    FAILED = "failed"


@dataclass
class Step:
    index: int
    description: str
    completed: bool = False


@dataclass
class SessionState:
    goal: str
    intent: Optional[str] = None
    target: Optional[str] = None
    status: TaskStatus = TaskStatus.PENDING
    current_step: int = 0
    steps: list[Step] = field(default_factory=list)

    def advance(self) -> None:
        if self.current_step < len(self.steps):
            self.steps[self.current_step].completed = True
            self.current_step += 1
        if self.current_step >= len(self.steps) and self.steps:
            self.status = TaskStatus.COMPLETED

    @property
    def is_complete(self) -> bool:
        return self.status == TaskStatus.COMPLETED
