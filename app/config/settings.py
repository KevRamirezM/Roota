"""
Centralised application settings.

Values come from environment variables and `.env` (gitignored). Defaults
mirror `.env.example` so the app runs out of the box on a stock Ollama
install with `qwen2.5:3b` pulled.
"""

from __future__ import annotations

from functools import lru_cache
from typing import Literal

from pydantic import Field
from pydantic_settings import BaseSettings, SettingsConfigDict

Language = Literal["es", "en"]
LogLevel = Literal["DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL"]


class Settings(BaseSettings):
    """All runtime configuration for Roota."""

    model_config = SettingsConfigDict(
        env_file=".env",
        env_file_encoding="utf-8",
        extra="ignore",
        case_sensitive=False,
    )

    OLLAMA_HOST: str = "http://localhost:11434"
    LLM_MODEL: str = "qwen2.5:3b"
    LLM_TEMPERATURE: float = Field(default=0.3, ge=0.0, le=2.0)
    LLM_MAX_TOKENS: int = Field(default=512, ge=16, le=8192)
    LLM_TIMEOUT_SECONDS: float = Field(default=30.0, ge=1.0, le=300.0)

    WHISPER_MODEL_SIZE: str = "small"
    WHISPER_LANGUAGE: Language = "es"
    STT_DEVICE: str = "cpu"

    TTS_RATE: int = Field(default=150, ge=80, le=400)
    TTS_VOLUME: float = Field(default=1.0, ge=0.0, le=1.0)

    UI_FONT_SIZE: int = Field(default=22, ge=14, le=72)
    UI_LANGUAGE: Language = "es"

    OVERLAY_OPACITY: float = Field(default=0.85, ge=0.1, le=1.0)
    OVERLAY_FPS: int = Field(default=30, ge=10, le=120)

    LOG_LEVEL: LogLevel = "INFO"
    LOG_DIR: str = "logs"

    SAFETY_STRICT: bool = True


@lru_cache(maxsize=1)
def get_settings() -> Settings:
    """Return the cached Settings instance."""
    return Settings()
