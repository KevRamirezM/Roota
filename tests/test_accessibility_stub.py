"""Tests for the stub accessibility scanner and UI element matching."""

from __future__ import annotations

from app.accessibility import StubScanner, UIElement, UISnapshot, get_scanner


def test_stub_returns_default_explorer_snapshot() -> None:
    snap = StubScanner().snapshot()

    assert snap.window == "Explorer"
    assert any(el.text == "Descargas" for el in snap.elements)


def test_ui_element_matches_case_insensitive() -> None:
    el = UIElement(type="button", text="Descargas", x=0, y=0, width=10, height=10, automation_id="downloads")
    assert el.matches("descargas")
    assert el.matches("DOWN")
    assert not el.matches("documentos")


def test_snapshot_find_uses_first_match() -> None:
    snap = StubScanner().snapshot()
    el = snap.find("Descargas")
    assert el is not None
    assert el.text == "Descargas"


def test_snapshot_find_returns_none_when_missing() -> None:
    snap = UISnapshot(window="x", elements=())
    assert snap.find("anything") is None


def test_get_scanner_factory_force_stub() -> None:
    scanner = get_scanner(force="stub")
    assert scanner.name == "stub"


def test_ui_element_center_is_midpoint() -> None:
    el = UIElement(type="button", text="x", x=100, y=200, width=40, height=20)
    assert el.center == (120, 210)
