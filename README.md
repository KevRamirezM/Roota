# Roota

> **Edge AI desktop assistant for senior citizens** — on-device, fully offline, accessibility-first.

Roota translates plain natural-language commands (typed or spoken) into step-by-step visual guidance directly on the Windows desktop, without ever sending data to the cloud.

---

## Tech Stack

| Layer | Technology |
|---|---|
| UI Framework | PySide6 |
| Windows Accessibility | pywinauto + UI Automation API |
| Local LLM Runtime | Ollama (Qwen2.5 3B) |
| Speech-to-Text | faster-whisper (offline) |
| Text-to-Speech | pyttsx3 / Windows SAPI |
| Configuration | pydantic-settings |
| Logging | loguru |

---

## Prerequisites

- **Python 3.10+** — [python.org](https://www.python.org/downloads/)
- **Ollama** — [ollama.com](https://ollama.com/) (must be running locally)
- **Qwen2.5 3B model** pulled in Ollama:
  ```bash
  ollama pull qwen2.5:3b
  ```

---

## Setup

```bash
# 1. Clone the repo
git clone <repo-url>
cd Roota

# 2. Create and activate a virtual environment
python -m venv .venv
.venv\Scripts\activate      # Windows PowerShell

# 3. Install dependencies
pip install -r requirements.txt

# 4. Configure environment
copy .env.example .env
# Edit .env as needed

# 5. Run the application
python -m app.ui.main
```

---

## Project Structure

```
app/
├── accessibility/   # UI Automation tree scanners and coordinate parsers
├── overlay/         # Transparent always-on-top rendering surface
├── llm/             # On-device LLM management (Ollama / Qwen2.5)
├── voice/           # Local STT (whisper) and TTS (pyttsx3) engines
├── orchestration/   # Core routing engine — syncs LLM with OS state
├── prompts/         # Versioned system prompt templates
├── state/           # Session goal and step tracking state machines
├── ui/              # PySide6 accessible user interface
├── telemetry/       # On-device structured logging (loguru)
└── config/          # Settings loaded via pydantic-settings
```

---

## Development Roadmap

| Phase | Scope |
|---|---|
| **MVP** | Windows Explorer navigation guidance (Python + pywinauto + Ollama) |
| **Phase 2** | Local voice pipeline (faster-whisper + Piper TTS) |
| **Phase 3** | Multi-app support (Chrome, Gmail, Office) |
| **Phase 4** | Production rewrite in Rust + Tauri |

---

## Privacy

All processing runs **100% on-device**. No data, voice recordings, or usage telemetry ever leaves the machine.
