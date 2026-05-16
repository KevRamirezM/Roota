"""
Deterministic in-memory scanner used by tests and non-Windows hosts.

Returning a stable, hand-crafted snapshot lets the orchestration tests
verify the decision logic without any flaky pywinauto dependency.
"""

from __future__ import annotations

from app.accessibility.element import UIElement, UISnapshot

_DEFAULT_ELEMENTS: tuple[UIElement, ...] = (
    UIElement(type="button", text="Descargas", x=120, y=340, width=160, height=32, automation_id="downloads", window="Explorer"),
    UIElement(type="button", text="Documentos", x=120, y=380, width=160, height=32, automation_id="documents", window="Explorer"),
    UIElement(type="button", text="Imágenes", x=120, y=420, width=160, height=32, automation_id="pictures", window="Explorer"),
    UIElement(type="button", text="Escritorio", x=120, y=460, width=160, height=32, automation_id="desktop", window="Explorer"),
    UIElement(type="text", text="Buscar", x=300, y=80, width=400, height=28, automation_id="search_box", window="Explorer"),
    UIElement(type="button", text="Nueva pestaña", x=20, y=20, width=120, height=28, automation_id="new_tab", window="Chrome"),
    UIElement(type="button", text="Redactar", x=40, y=160, width=120, height=40, automation_id="compose", window="Gmail"),
    UIElement(type="button", text="Bandeja de entrada", x=40, y=220, width=200, height=32, automation_id="inbox", window="Gmail"),
    UIElement(type="button", text="Imprimir", x=80, y=140, width=120, height=32, automation_id="print", window="Word"),
)


class StubScanner:
    """Always returns the same fixture snapshot; supports overrides for tests."""

    name = "stub"

    def __init__(self, snapshot: UISnapshot | None = None) -> None:
        self._snapshot = snapshot or UISnapshot(window="Explorer", elements=_DEFAULT_ELEMENTS)

    def set_snapshot(self, snapshot: UISnapshot) -> None:
        self._snapshot = snapshot

    def snapshot(self) -> UISnapshot:
        return self._snapshot
