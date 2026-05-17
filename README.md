# Roota

> **Edge AI desktop assistant for senior citizens** — Tauri 2 + Rust + React, fully offline, accessibility-first.

Roota translates plain natural-language commands into single-step visual guidance
on the Windows desktop. It **never moves the mouse, types for you, or reaches the
network**. Roota is a calm, patient companion, not an autopilot.

---

## Tech Stack

| Layer | Technology |
|---|---|
| Desktop shell | Tauri 2 |
| Backend | Rust (edition 2021, stable) |
| Async runtime | tokio (multi-thread) |
| HTTP / LLM | reqwest + llama.cpp `llama-server` (qwen3 1.7b Q4); Ollama optional for vision |
| Windows accessibility | `uiautomation` crate (read-only) |
| Visual overlay | Frameless click-through Tauri webview window |
| Frontend | React 18 + TypeScript + Vite |
| Logging | tracing |

---

## Prerequisites

- **Windows 10/11** (UI Automation backend is Windows-only — non-Windows hosts compile, but the scanner returns a deterministic stub snapshot).
- **Rust toolchain** via [rustup](https://rustup.rs/) (`rustc` 1.77+, `cargo`).
- **Microsoft Visual Studio 2022 Build Tools** with the *Desktop development with C++* workload (required by Tauri to link the Windows binary).
- **Node.js 18+** and **npm** (or pnpm).
- **llama.cpp** `llama-server` for text inference (see [Setup](#setup)).
- **Optional:** **Ollama** + `moondream:1.8b` only if you enable `ROOTA_VISION_VLM=1`.
- **WebView2 Runtime** (already shipped with Windows 10 21H2+ and Windows 11).

---

## Setup

```powershell
git clone <repo-url>
cd Roota

# Install JS deps
npm install

# Sanity check Rust + Cargo
cargo --version

# Terminal 1 — local text LLM (required for non-stub guidance)
.\scripts\start-llama.ps1

# Terminal 2 — app
npm run tauri:dev
```

Download `llama-server.exe` into `bin/` from [llama.cpp releases](https://github.com/ggml-org/llama.cpp/releases), then run `.\scripts\download-model.ps1` for GGUF placement instructions.

### Optional environment overrides

Create a `.env` next to `package.json` (gitignored) to override defaults:

```ini
LLM_BACKEND=llamacpp
LLAMA_HOST=http://127.0.0.1:8080
LLM_TIMEOUT_SECONDS=30
LLM_HEALTH_TIMEOUT_SECONDS=2
ROOTA_PLANNER_PROMPT_ELEMENTS=28
ROOTA_STEP_LLM=0
LLM_TEMPERATURE=0.3
LLM_MAX_TOKENS=512
UI_LANGUAGE=es        # or en
OVERLAY_OPACITY=0.85
OVERLAY_FPS=30
LOG_LEVEL=info
SAFETY_STRICT=true

# Rollback text LLM to Ollama:
# LLM_BACKEND=ollama
# OLLAMA_HOST=http://localhost:11434
# LLM_MODEL=qwen3:1.7b

# Optional vision VLM (requires Ollama):
# ROOTA_VISION_VLM=1
# ROOTA_VISION_MODEL=moondream:1.8b
```

---

## Demo Flow

1. `npm run tauri:dev` launches the main Roota window (and a hidden overlay window that becomes visible only when an anchor is shown).
2. The greeting reads *"¿Qué tarea quieres que haga por ti hoy?"*.
3. Type **"Abre la carpeta de Descargas"** and press **Empezar**.
4. A massive green **SÍ** / red **NO** modal asks: *"Voy a abrir la carpeta Descargas. ¿Está bien?"* (Y/Enter or N/Esc shortcuts work).
5. On YES, the overlay window draws a pulsing yellow anchor over the target element in File Explorer with a label.
6. Open the folder yourself. Roota detects the change and shows the *"¡Listo!"* state.

Other supported intents (with offline stub fallback): `open_folder`, `move_file`, `delete_file`, `open_browser`, `search_web`, `open_url`, `compose_email`, `read_inbox`, `reply_message`, `open_word_document`, `print_document`.

---

## Project Structure

```
.
├── index.html                  # Vite root
├── vite.config.ts
├── tsconfig.json
├── package.json
├── src/                        # React frontend (TypeScript)
│   ├── App.tsx                 # Routes between MainScreen and OverlayCanvas
│   ├── theme.css               # WCAG AAA palette + large typography
│   ├── i18n.ts                 # ES/EN strings, kept in sync with Rust catalog
│   ├── tauri-api.ts            # invoke + listen wrappers
│   ├── components/
│   │   ├── MainScreen.tsx
│   │   ├── ConfirmationModal.tsx   # giant YES/NO native <dialog>
│   │   ├── FeedbackPanel.tsx       # aria-live region
│   │   └── OverlayCanvas.tsx       # pulsing anchor on transparent canvas
│   └── hooks/useOrchestrator.ts
└── src-tauri/                  # Rust backend
    ├── Cargo.toml
    ├── tauri.conf.json         # main + overlay window definitions
    ├── capabilities/default.json
    ├── prompts/                # system + intent classifier + step prompts (embed!)
    ├── templates/              # JSON guidance templates merged at boot
    ├── icons/
    └── src/
        ├── main.rs             # thin passthrough — calls roota_lib::run()
        ├── lib.rs              # builder + state + generate_handler!
        ├── settings.rs         # env-loaded Settings
        ├── i18n.rs             # ES/EN catalog + t()
        ├── safety.rs           # SafetyGuard, GuideAction, ActionType, UnsafeActionError
        ├── prompts.rs          # include_str! prompt loaders
        ├── llm/                # LlmClient trait + Ollama + stub + resilient
        ├── accessibility/      # UiElement, UiSnapshot, Scanner trait, Windows + stub
        ├── orchestration/      # Intent, decision, state detector, templates, orchestrator
        └── commands.rs         # Tauri command handlers
```

---

## Verifying the build

```powershell
# All Rust unit tests (27/27 expected)
cargo test --manifest-path src-tauri/Cargo.toml --lib

# Lint
cargo clippy --manifest-path src-tauri/Cargo.toml --no-deps -- -D warnings

# Frontend production bundle
npm run build

# Full Tauri release build (.exe + .msi installer)
npm run tauri:build
```

---

## Privacy & Safety Contract

- All processing runs **100% on-device**. Text LLM calls go to `localhost:8080` (llama-server); optional vision uses `localhost:11434` (Ollama).
- **Roota never automates input.** A `SafetyGuard` runs every emitted action through a closed allow-list (`Anchor`, `Highlight`, `Arrow`, `Speak`, `ShowText`, `Scan`). Anything else (`Click`, `TypeText`, `KeyPress`, `Drag`, `FileOp`, …) raises `UnsafeActionError`.
- The Windows scanner only **reads** the active foreground window's UIA tree.
- The codebase imports zero input-synthesis crates (no `enigo`, no `Win32::UI::Input::KeyboardAndMouse` callers).
- Logs land under `logs/` and are local-only; nothing is uploaded.

---

## Resilience

- If llama-server is unreachable at boot (2s health probe), Roota uses a deterministic regex-based stub LLM so the UI still works.
- If the text backend is up but a single call fails (timeout, malformed JSON), `ResilientLlmClient` transparently falls back to the stub for the rest of the session — no crash, no broken demo.
- The `WindowsScanner` returns an empty snapshot on any UIA error rather than panicking, and the orchestrator emits a friendly *"No encuentro {target}. ¿Lo ves en pantalla?"* error to the user.

---

## Roadmap (PRD §10)

| Phase | Status | Scope |
|---|---|---|
| Phase 1 — MVP | shipped (Rust) | File Explorer guidance, intent recognizer, overlay |
| Phase 2 — Voice | not started | whisper.cpp STT + Piper/SAPI TTS — out of scope for this rewrite |
| Phase 3 — Multi-app | shipped (Rust) | Chrome / Gmail / Office guidance JSON templates |
| Phase 4 — Production rewrite | **shipped** | Tauri 2 + Rust + React replaces the prior Python prototype |

---

## Documentation

- Spec: [`docs/superpowers/specs/2026-05-16-roota-rust-tauri-rewrite.md`](docs/superpowers/specs/2026-05-16-roota-rust-tauri-rewrite.md)
- Implementation plan: [`docs/superpowers/plans/2026-05-16-roota-rust-tauri-rewrite.md`](docs/superpowers/plans/2026-05-16-roota-rust-tauri-rewrite.md)
- Original PRD: [`PRD.md`](PRD.md)
