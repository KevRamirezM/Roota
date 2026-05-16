"""
config — application settings loaded from environment + .env file.

Exposes a single `Settings` instance via `get_settings()`.
"""

from app.config.settings import Settings, get_settings

__all__ = ["Settings", "get_settings"]
