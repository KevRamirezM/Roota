# Roota MVP — Design Spec

**Date:** 2026-05-16
**Author:** Roota team
**Status:** Implemented (Phases 1–3)

## Purpose

Build a local-only Windows desktop assistant for senior citizens that
turns natural-language commands into one-step-at-a-time visual
guidance. Roota never automates input. Everything runs offline.

## Locked Decisions

| Decision | Choice | Rationale |
|---|---|---|
| PRD scope | Phases 1–3 in Python; skip Phase 4 (Rust/Tauri rewrite) | Hackathon timebox; PRD explicitly stages this |
| LLM runtime | Local Ollama with `qwen2.5:3b` | Already pulled on the demo box; under 2 GB, fast |
| Safety posture | Strict guide-only — no `pyautogui`, no `SendInput`, no `pywinauto`-driven clicks | PRD §8.9 Safety Layer |
| Test rigor | Pragmatic: unit tests on the orchestration brain; smoke tests for shells | Hackathon-grade coverage with a real safety net |
| Default UI language | Spanish, English available via `UI_LANGUAGE=en` | PRD persona is a 55–75 year old Spanish-speaking user |
| Cross-platform dev | `pywinauto` gated by `sys.platform == "win32"`; stub scanner everywhere else | So tests run on any host |

## Architecture

```mermaid
flowchart TB
    User["User input - voice or text"] --> InputLayer
    InputLayer["app.ui MainWindow + STT"] --> Orchestrator
    Orchestrator["app.orchestration.Orchestrator"] --> IntentRec
    IntentRec["IntentRecognizer LLM JSON"] --> ConfirmGate
    ConfirmGate["app.ui ConfirmationModal"] --> Loop
    subgraph Loop ["Step loop"]
      direction TB
      Scanner["AccessibilityScanner pywinauto"] --> Decision
      Decision["DecisionEngine + Templates"] --> SafetyGuard
      SafetyGuard["SafetyGuard reject automation"] --> InstructionGen
      InstructionGen["LLM single step prompt"] --> Render
      Render["Overlay anchor + Feedback panel + TTS"] --> Detector
      Detector["StateDetector re-scan"] --> Loop
    end
    Loop --> Done["Positive feedback + session reset"]
    Telemetry["app.telemetry loguru"] -.-> Orchestrator
    Telemetry -.-> SafetyGuard
    Settings["app.config Settings .env"] -.-> Orchestrator
```

## Component Boundaries

- `app.config.settings` — Pydantic `Settings`, lazy-loaded.
- `app.telemetry.logger` — loguru file + console sinks; nothing leaves the device.
- `app.i18n` — Spanish/English string catalog with a tiny `t()` lookup.
- `app.safety.guard` — `SafetyGuard.review()` rejects every automating action; a closed allow-list of guide actions.
- `app.state.session` — `Intent`, `GuideStep`, `SessionState`, `SessionStore`.
- `app.accessibility.{element,scanner,stub_scanner,windows_scanner}` — `UIElement` / `UISnapshot`, `AccessibilityScanner` Protocol, `StubScanner` (used in tests + non-Windows), `WindowsScanner` (pywinauto UIA).
- `app.llm.{client,ollama_client,stub_client,resilient,persona}` — `LLMClient` Protocol with two backends and a resilient wrapper that falls back per-call when Ollama runs out of RAM.
- `app.prompts` — system prompt + intent classifier + instruction template + JSON guidance templates.
- `app.orchestration.{templates,intent,decision,state_detector,orchestrator,worker}` — the brain. Orchestrator is pure-Python (testable). `OrchestratorWorker` is the Qt-aware adapter.
- `app.overlay.{shapes,window,controller}` — frameless, click-through, always-on-top overlay with a pulsing anchor.
- `app.ui.{theme,main_window,confirmation_modal,feedback_panel,main}` — WCAG AAA palette, big input, giant YES/NO modal, feedback card, app entry.
- `app.voice.{tts,stt,recorder}` — pyttsx3 (with `NullTTS` fallback), faster-whisper (with `NullSTT` fallback), sounddevice push-to-talk recorder.

## Data Flow

1. User types or speaks. STT (faster-whisper) transcribes if voice.
2. `MainWindow.command_submitted` fires → `OrchestratorWorker.run(text)` (queued to its `QThread`).
3. `IntentRecognizer` calls the LLM with `intent_classifier.txt` and validates the JSON against the registered template catalog.
4. `Orchestrator.build_confirmation` produces a localized phrase. The Qt UI shows the giant YES/NO `ConfirmationModal`.
5. On YES, the orchestrator enters its loop. `DecisionEngine` reads a fresh `UISnapshot` and emits a `GuideStep`. Every step passes through `SafetyGuard.review`.
6. UI updates: `FeedbackPanel.show_step`, `OverlayController.show_anchor`, `TextToSpeech.speak`.
7. `StateDetector` polls subsequent snapshots; when the target disappears or the foreground window changes, the session advances.
8. When the template runs out of steps, `goal_completed` fires and the UI shows the success state.

## Safety Contract

- The single source of truth is `app.safety.guard.SafetyGuard`. Action types fall in two frozen sets: `GUIDE_ACTIONS` and `AUTOMATION_ACTIONS`. Anything in the latter raises `UnsafeActionError`.
- `DecisionEngine.next_step` reviews every emitted action.
- The codebase imports nothing that could synthesise input (`pyautogui`, `keyboard`, `pyinput`, `mouse`).
- The `WindowsScanner` only *reads* UI nodes via UIA; it never clicks.

## Observability

- Single `loguru` logger configured via `Settings`. Output goes to `logs/roota.log` (rotating 5 MB × 5) and stderr.
- No telemetry leaves the device — there's literally no network sink.

## Testing Strategy

- 125 tests, 0 failures, ruff clean.
- Brain modules (safety, session, intent, decision, state detector, orchestrator) have unit tests.
- Voice + LLM resilience have negative-path tests.
- UI shells are smoke-tested (construct + signal fires) with the offscreen Qt platform.

## Definition of Done

- `pytest -q` → all green.
- `ruff check app tests` → all clean.
- `python -m app.ui.main` launches on Windows with Ollama running and `qwen2.5:3b` pulled.
- The user types or speaks a Spanish command; Roota asks YES/NO; on YES it draws a pulsing anchor and reads aloud one calm instruction; advances when the target disappears.
- Roota never moves the mouse, never types for the user, never makes a network call.
