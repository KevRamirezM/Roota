"""Smoke tests for voice fallback behaviour."""

from __future__ import annotations

import numpy as np

from app.voice.stt import NullSTT, _resample, get_stt
from app.voice.tts import NullTTS, get_tts


def test_null_tts_does_not_raise() -> None:
    tts = NullTTS()
    tts.speak("hola")
    tts.stop()


def test_null_stt_returns_empty() -> None:
    out = NullSTT().transcribe(np.zeros(0, dtype=np.float32), 16000)
    assert out == ""


def test_get_tts_force_null() -> None:
    assert get_tts(force="null").name == "null"


def test_get_stt_force_null() -> None:
    assert get_stt(force="null").name == "null"


def test_resample_noop_when_rates_match() -> None:
    audio = np.array([0.1, 0.2, 0.3], dtype=np.float32)
    out = _resample(audio, 16000, 16000)
    assert out is audio


def test_resample_changes_length() -> None:
    audio = np.linspace(-1, 1, num=32000, dtype=np.float32)
    out = _resample(audio, 32000, 16000)
    assert out.shape[0] == 16000


def test_recorder_stop_without_start_returns_empty() -> None:
    from app.voice.recorder import MicrophoneRecorder

    rec = MicrophoneRecorder()
    audio, sr = rec.stop()
    assert audio.size == 0
    assert sr == 16000
