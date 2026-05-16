# Roota

> **Edge AI desktop assistant for senior citizens** — on-device, fully offline, accessibility-first.

Roota translates plain natural-language commands (typed or spoken) into step-by-step
visual guidance directly on the Windows desktop. It **never moves the mouse, types
for you, or reaches the network**. Roota is a calm, patient companion — not an
autopilot.

---

## Tech Stack

| Layer | Technology |
|---|---|
| UI Framework | PySide6 (WCAG AAA palette, 22pt+ typography, giant YES/NO modal) |
| Visual overlay | Frameless, click-through, always-on-top transparent window |
| Windows Accessibility | pywinauto (UIA backend) — read-only |
| Local LLM | Ollama running `qwen2.5:3b` (with deterministic stub fallback) |
| Speech-to-Text | faster-whisper, fully offline (`NullSTT` fallback) |
| Text-to-Speech | pyttsx3 / Windows SAPI (`NullTTS` fallback) |
| Audio capture | sounddevice (push-to-talk) |
| Configuration | pydantic-settings |
| Logging | loguru — local files only |

---

## Prerequisites

- **Python 3.10+** — [python.org](https://www.python.org/downloads/) (tested on 3.13.9)
- **Ollama** — [ollama.com](https://ollama.com/) (must be running locally)
- **`qwen2.5:3b` model** pulled in Ollama (≈1.9 GB RAM at runtime):
  ```powershell
  ollama pull qwen2.5:3b
  ```
- A working microphone (only required for voice input)

---

## Setup (Windows PowerShell)

```powershell
git clone <repo-url>
cd Roota

python -m venv .venv
.venv\Scripts\activate

pip install -r requirements.txt

copy .env.example .env   # adjust if needed

python -m app.ui.main
```

---

## Demo Flow

1. Run `python -m app.ui.main`. A large window opens asking *"¿Qué tarea quieres que haga por ti hoy?"*.
2. Type **"Abre la carpeta de Descargas"** (or press the microphone button and speak it).
3. Roota shows a giant green **SÍ** / red **NO** modal: *"Voy a empezar a abrir la carpeta Descargas. ¿Está bien?"*.
4. Press **SÍ** (or `Y` / Enter on the keyboard).
5. Roota draws a pulsing visual anchor on the Downloads icon in File Explorer and reads aloud *"Haz doble clic en Descargas"*.
6. Open the folder yourself. Roota detects the change and shows a positive *"¡Perfecto!"* state.

Other supported commands: `"abre Chrome"`, `"busca el clima"`, `"escríbele un correo a Elena"`, `"abre mi bandeja de entrada"`, `"abre Word"`, `"imprime el documento"`, `"borra esta foto"`, `"mueve este archivo"`, etc.

---

## Project Structure

```
app/
├── accessibility/   # UIA tree scanners and coordinate parsers (pywinauto + stub)
├── overlay/         # Frameless click-through visual guidance plane
├── llm/             # LLMClient Protocol, OllamaClient, StubLLMClient, ResilientLLMClient
├── voice/           # Pyttsx3TTS, WhisperSTT, MicrophoneRecorder
├── orchestration/   # The brain - intent, decision, state detector, orchestrator
├── prompts/         # System prompt + intent classifier + JSON guidance templates
├── state/           # Session and step tracking
├── ui/              # PySide6 main window, confirmation modal, feedback panel, theme
├── safety/          # SafetyGuard - rejects any automating action
├── i18n/            # Spanish (default) + English string catalogs
├── telemetry/       # Local-only loguru config
└── config/          # Pydantic settings loaded from .env

docs/superpowers/    # Spec + execution log
tests/               # 125 unit + smoke tests
```

---

## Running the Tests

```powershell
$env:QT_QPA_PLATFORM = "offscreen"
.venv\Scripts\python.exe -m pytest tests -q
.venv\Scripts\python.exe -m ruff check app tests
```

Expected: `125 passed` and `All checks passed!`.

---

## Privacy & Safety Contract

- All processing runs **100% on-device**. There is no network sink in the codebase.
- **Roota never automates input.** A `SafetyGuard` runs every emitted action through a
  closed allow-list (`anchor`, `highlight`, `arrow`, `speak`, `show_text`, `scan`).
  Anything else (`click`, `type_text`, `key_press`, `drag`, `file_op`, ...) raises
  `UnsafeActionError`.
- The `WindowsScanner` only **reads** the active window's UIA tree.
- Logs land under `logs/` and are rotated locally; nothing is uploaded.

---

## Troubleshooting

- **"Ollama not responding"** — make sure the daemon is running (`ollama serve`),
  the model is pulled (`ollama list`), and the daemon is reachable at the
  `OLLAMA_HOST` you set in `.env`. Roota will transparently fall back to a
  deterministic stub LLM if Ollama is unreachable, so the UI still works.
- **"model requires more system memory"** — close memory-hungry apps and retry.
  Roota's `ResilientLLMClient` already falls back to the stub for that call.
- **Microphone not detected** — check Windows microphone permissions for Python,
  then click the microphone button. The UI shows
  *"El micrófono no está listo. Puedes escribirlo en su lugar."* if it can't open.
- **Overlay not visible** — Windows sometimes filters always-on-top windows; run
  Roota as a normal user (no need for admin) and ensure no other always-on-top
  app is shadowing it.

---

## Roadmap (PRD §10)

| Phase | Status | Scope |
|---|---|---|
| Phase 1 — MVP | shipped | File Explorer guidance (Python + pywinauto + Ollama) |
| Phase 2 — Voice | shipped | faster-whisper STT + pyttsx3 TTS |
| Phase 3 — Multi-app | shipped | Chrome, Gmail, Office guidance templates |
| Phase 4 — Production rewrite | not started | Native Rust + Tauri replacement |
