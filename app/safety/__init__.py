"""
safety — runtime gate that enforces Roota's "never automates input" rule.
"""

from app.safety.guard import ActionType, GuideAction, SafetyGuard, UnsafeActionError

__all__ = ["SafetyGuard", "UnsafeActionError", "GuideAction", "ActionType"]
