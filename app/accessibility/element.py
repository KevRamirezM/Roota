"""
Plain-data records describing what's on screen.

Keep these dataclasses pure — no platform imports. Both the windows
backend and the stub backend produce these so the rest of the app
stays portable.
"""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass(frozen=True)
class UIElement:
    """One actionable element on screen."""

    type: str
    text: str
    x: int
    y: int
    width: int
    height: int
    automation_id: str | None = None
    window: str = ""

    @property
    def center(self) -> tuple[int, int]:
        return (self.x + self.width // 2, self.y + self.height // 2)

    def matches(self, query: str) -> bool:
        """Loose case-insensitive substring match on text + automation_id."""
        q = query.casefold().strip()
        if not q:
            return False
        haystacks = [self.text.casefold(), (self.automation_id or "").casefold()]
        return any(q in hay for hay in haystacks if hay)


@dataclass(frozen=True)
class UISnapshot:
    """An ordered collection of elements taken at a single point in time."""

    window: str
    elements: tuple[UIElement, ...] = field(default_factory=tuple)

    def find(self, query: str) -> UIElement | None:
        """Return the first element whose text/automation id matches."""
        for el in self.elements:
            if el.matches(query):
                return el
        return None

    def find_all(self, query: str) -> tuple[UIElement, ...]:
        return tuple(el for el in self.elements if el.matches(query))
