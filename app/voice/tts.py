"""
Text-to-speech backends.

`Pyttsx3TTS` uses Windows SAPI under the hood (no extra download
required). `NullTTS` is the silent fallback when pyttsx3 is missing.
"""

from __future__ import annotations

import threading
from typing import Protocol, runtime_checkable

from app.config import get_settings
from app.telemetry import get_logger

logger = get_logger("app.voice.tts")


@runtime_checkable
class TextToSpeech(Protocol):
    name: str

    def speak(self, text: str) -> None:
        ...

    def stop(self) -> None:
        ...


class NullTTS:
    """Silent fallback. Logs the text instead of speaking."""

    name = "null"

    def speak(self, text: str) -> None:
        logger.info(f"[silent TTS] {text}")

    def stop(self) -> None:
        pass


class Pyttsx3TTS:
    """Native Windows / cross-platform TTS via pyttsx3."""

    name = "pyttsx3"

    def __init__(self) -> None:
        import pyttsx3

        self._engine = pyttsx3.init()
        settings = get_settings()
        self._engine.setProperty("rate", settings.TTS_RATE)
        self._engine.setProperty("volume", settings.TTS_VOLUME)
        self._lock = threading.Lock()
        self._thread: threading.Thread | None = None

    def speak(self, text: str) -> None:
        if not text.strip():
            return

        def _run() -> None:
            with self._lock:
                try:
                    self._engine.stop()
                    self._engine.say(text)
                    self._engine.runAndWait()
                except RuntimeError as exc:
                    logger.warning(f"pyttsx3 failure: {exc!r}")

        self._thread = threading.Thread(target=_run, daemon=True)
        self._thread.start()

    def stop(self) -> None:
        try:
            self._engine.stop()
        except Exception as exc:  # pragma: no cover - depends on driver
            logger.debug(f"pyttsx3 stop ignored: {exc!r}")


def get_tts(*, force: str | None = None) -> TextToSpeech:
    """Return the best available TTS implementation."""
    if force == "null":
        return NullTTS()
    if force == "pyttsx3":
        return Pyttsx3TTS()
    try:
        return Pyttsx3TTS()
    except Exception as exc:
        logger.warning(f"pyttsx3 unavailable ({exc!r}); using NullTTS")
        return NullTTS()
