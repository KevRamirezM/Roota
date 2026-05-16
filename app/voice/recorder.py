"""
Microphone recorder.

Uses `sounddevice` to capture mono float32 audio at 16 kHz (whisper's
preferred rate). Press-and-hold semantics: `start()` to begin, `stop()`
to halt and receive the buffer.
"""

from __future__ import annotations

import threading

import numpy as np

from app.telemetry import get_logger

logger = get_logger("app.voice.recorder")

_DEFAULT_SAMPLE_RATE = 16000


class MicrophoneRecorder:
    """Push-to-talk style microphone capture."""

    def __init__(self, sample_rate: int = _DEFAULT_SAMPLE_RATE, channels: int = 1) -> None:
        self._sample_rate = sample_rate
        self._channels = channels
        self._stream = None
        self._buffer: list[np.ndarray] = []
        self._lock = threading.Lock()
        self._recording = False

    @property
    def is_recording(self) -> bool:
        return self._recording

    @property
    def sample_rate(self) -> int:
        return self._sample_rate

    def start(self) -> bool:
        if self._recording:
            return True
        try:
            import sounddevice as sd
        except Exception as exc:
            logger.warning(f"sounddevice unavailable: {exc!r}")
            return False

        self._buffer = []

        def _on_chunk(indata, frames, time_info, status) -> None:  # type: ignore[no-untyped-def]
            if status:
                logger.debug(f"sounddevice status: {status}")
            with self._lock:
                self._buffer.append(indata.copy().reshape(-1))

        try:
            self._stream = sd.InputStream(
                samplerate=self._sample_rate,
                channels=self._channels,
                dtype="float32",
                callback=_on_chunk,
            )
            self._stream.start()
            self._recording = True
            return True
        except Exception as exc:
            logger.warning(f"Failed to open mic stream: {exc!r}")
            self._stream = None
            return False

    def stop(self) -> tuple[np.ndarray, int]:
        if not self._recording:
            return np.zeros(0, dtype=np.float32), self._sample_rate
        if self._stream is not None:
            try:
                self._stream.stop()
                self._stream.close()
            except Exception as exc:
                logger.debug(f"mic stream close error: {exc!r}")
            self._stream = None
        self._recording = False
        with self._lock:
            chunks = list(self._buffer)
            self._buffer = []
        if not chunks:
            return np.zeros(0, dtype=np.float32), self._sample_rate
        return np.concatenate(chunks).astype(np.float32), self._sample_rate
