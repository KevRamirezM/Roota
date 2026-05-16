"""
Centralised application settings loaded via pydantic-settings.

All values have sensible defaults and can be overridden through a .env
file placed at the project root, or via real environment variables.
"""

from pydantic_settings import BaseSettings, SettingsConfigDict
from pydantic import Field


class Settings(BaseSettings):
    model_config = SettingsConfigDict(
        env_file=".env",
        env_file_encoding="utf-8",
        case_sensitive=False,
        extra="ignore",
    )

    # ── LLM ──────────────────────────────────────────────────────────────────
    ollama_host: str = Field("http://localhost:11434", description="Ollama server base URL")
    llm_model: str = Field("qwen2.5:3b", description="Ollama model tag to use")
    llm_temperature: float = Field(0.3, ge=0.0, le=2.0)
    llm_max_tokens: int = Field(512, ge=1)

    # ── Voice / STT ───────────────────────────────────────────────────────────
    whisper_model_size: str = Field(
        "small",
        description="faster-whisper model size: tiny | base | small | medium | large",
    )
    whisper_language: str = Field("es", description="BCP-47 language code for transcription")
    stt_device: str = Field("cpu", description="Inference device: cpu | cuda")

    # ── Voice / TTS ───────────────────────────────────────────────────────────
    tts_rate: int = Field(150, description="Speech rate (words per minute)")
    tts_volume: float = Field(1.0, ge=0.0, le=1.0)

    # ── UI ────────────────────────────────────────────────────────────────────
    ui_font_size: int = Field(22, description="Base font size in points for accessibility")
    ui_language: str = Field("es", description="Display language: es | en")

    # ── Overlay ───────────────────────────────────────────────────────────────
    overlay_opacity: float = Field(0.85, ge=0.1, le=1.0)
    overlay_fps: int = Field(30, description="Target render rate for overlay animations")

    # ── Logging ───────────────────────────────────────────────────────────────
    log_level: str = Field("INFO", description="Loguru log level")
    log_dir: str = Field("logs", description="Directory for rotating log files")


settings = Settings()
