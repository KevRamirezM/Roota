"""
Speech-to-text backends.

`WhisperSTT` wraps faster-whisper with lazy model loading so import is
cheap. `NullSTT` is the fallback that simply returns an empty string
(the UI shows the "voice unavailable" toast in that case).
"""

from __future__ import annotations

from typing import Protocol, runtime_checkable

import numpy as np

from app.config import get_settings
from app.telemetry import get_logger

logger = get_logger("app.voice.stt")


@runtime_checkable
class SpeechToText(Protocol):
    name: str

    def transcribe(self, audio: np.ndarray, sample_rate: int) -> str:
        ...


class NullSTT:
    """No-op fallback used when faster-whisper isn't available."""

    name = "null"

    def transcribe(self, audio: np.ndarray, sample_rate: int) -> str:
        return ""


class WhisperSTT:
    """Wraps faster-whisper. Loads the model on first transcription."""

    name = "whisper"

    def __init__(self, *, model_size: str | None = None, language: str | None = None) -> None:
        settings = get_settings()
        self._model_size = model_size or settings.WHISPER_MODEL_SIZE
        self._language = language or settings.WHISPER_LANGUAGE
        self._device = settings.STT_DEVICE
        self._model = None  # lazy

    def _ensure_model(self) -> None:
        if self._model is not None:
            return
        from faster_whisper import WhisperModel

        logger.info(f"Loading whisper model {self._model_size!r} on {self._device!r}")
        self._model = WhisperModel(self._model_size, device=self._device, compute_type="int8")

    def transcribe(self, audio: np.ndarray, sample_rate: int) -> str:
        if audio.size == 0:
            return ""
        try:
            self._ensure_model()
        except Exception as exc:
            logger.warning(f"Cannot load whisper model: {exc!r}")
            return ""
        if audio.dtype != np.float32:
            audio = audio.astype(np.float32)
        if sample_rate != 16000:
            audio = _resample(audio, sample_rate, 16000)
        try:
            assert self._model is not None
            segments, _info = self._model.transcribe(audio, language=self._language, vad_filter=True)
            return " ".join(seg.text for seg in segments).strip()
        except Exception as exc:
            logger.warning(f"Whisper transcription failed: {exc!r}")
            return ""


def _resample(audio: np.ndarray, src_rate: int, dst_rate: int) -> np.ndarray:
    """Crude linear resampler for mono PCM. Good enough for STT input."""
    if src_rate == dst_rate or audio.size == 0:
        return audio
    duration = audio.shape[0] / src_rate
    new_length = int(duration * dst_rate)
    if new_length <= 0:
        return np.zeros(0, dtype=np.float32)
    src_indices = np.linspace(0, audio.shape[0] - 1, num=new_length).astype(np.int64)
    return audio[src_indices].astype(np.float32)


def get_stt(*, force: str | None = None) -> SpeechToText:
    if force == "null":
        return NullSTT()
    if force == "whisper":
        return WhisperSTT()
    try:
        return WhisperSTT()
    except Exception as exc:
        logger.warning(f"WhisperSTT unavailable ({exc!r}); using NullSTT")
        return NullSTT()
