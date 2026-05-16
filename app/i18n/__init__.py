"""
i18n — user-facing strings keyed by language code.

All visible text routes through `t(key)` so changing language is a
single config flip. Default language follows `Settings.UI_LANGUAGE`.
"""

from __future__ import annotations

from app.config import get_settings
from app.i18n import en, es

_CATALOGS: dict[str, dict[str, str]] = {
    "es": es.STRINGS,
    "en": en.STRINGS,
}


class TranslationKeyError(KeyError):
    """Raised when a translation key is unknown for the active locale."""


def t(key: str, lang: str | None = None, **fmt: object) -> str:
    """Look up a translated string and apply optional format keywords."""
    locale = lang or get_settings().UI_LANGUAGE
    catalog = _CATALOGS.get(locale)
    if catalog is None:
        catalog = _CATALOGS["es"]
    if key not in catalog:
        fallback = _CATALOGS["es"].get(key)
        if fallback is None:
            raise TranslationKeyError(f"Unknown i18n key: {key!r}")
        text = fallback
    else:
        text = catalog[key]
    if fmt:
        return text.format(**fmt)
    return text


__all__ = ["t", "TranslationKeyError"]
