"""
voice — fully offline microphone, speech-to-text and text-to-speech.

Implementations are lazy-imported so a missing optional dependency
(faster-whisper, pyttsx3, sounddevice) only fails at first use and
the rest of the app keeps working.
"""

from app.voice.stt import NullSTT, SpeechToText, WhisperSTT, get_stt
from app.voice.tts import NullTTS, Pyttsx3TTS, TextToSpeech, get_tts

__all__ = [
    "NullSTT",
    "NullTTS",
    "Pyttsx3TTS",
    "SpeechToText",
    "TextToSpeech",
    "WhisperSTT",
    "get_stt",
    "get_tts",
]
