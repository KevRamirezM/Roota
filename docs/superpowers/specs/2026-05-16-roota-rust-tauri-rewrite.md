# Roota — Rust + Tauri Rewrite Spec

**Date:** 2026-05-16
**Author:** Roota team
**Status:** In progress (replaces the Python implementation)
**Supersedes:** [`docs/superpowers/specs/2026-05-16-roota-mvp-design.md`](2026-05-16-roota-mvp-design.md)

## Purpose

Re-implement Roota natively per PRD §10 Phase 4 — drop the Python prototyping
layer in favour of a Rust backend behind a Tauri v2 desktop shell with a React
frontend. Preserve the strict guide-only safety contract and the WCAG-AAA UX
principles from the prior implementation.

## Locked Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Desktop shell | Tauri 2.x | PRD §8 stack; smallest binary, native Windows window control |
| Backend language | Rust (stable, edition 2021) | PRD §8; required by Tauri |
| Frontend framework | React 18 + TypeScript + Vite | User pick — biggest a11y ecosystem (Radix UI), familiar |
| Scope | Tauri shell + LLM + UI + UIA accessibility scanner + transparent overlay | User pick — proves the full architecture without voice |
| Voice (STT/TTS) | **Out of scope for this rewrite** | Defer to a follow-up milestone |
| Disposition of Python prototype | Delete at the end (after Rust app builds and runs) | User pick — clean slate |
| LLM | Local Ollama via HTTP `reqwest` + deterministic stub fallback | Mirrors Python `ResilientLLMClient` |
| Accessibility | `uiautomation` crate (Rust UIA wrapper) + stub scanner | Read-only; no input synthesis |
| Overlay | Separate Tauri `WebviewWindow` with `transparent: true`, `alwaysOnTop: true`, `decorations: false`, `setIgnoreCursorEvents(true)` | Click-through, frameless, full-screen |
| State sharing | `Mutex<AppState>` managed by Tauri | Single-session assistant — no need for actor model |
| Cross-platform | Windows-first (matches PRD); UIA stub on non-Windows for compile parity | Mac/Linux can build but the scanner returns the deterministic snapshot |

## Architecture

```mermaid
flowchart TB
    User["User input"] --> ReactUI["React MainScreen"]
    ReactUI -->|invoke run_session| TauriCmds
    TauriCmds["Tauri command handlers"] --> Orchestrator
    Orchestrator["Rust Orchestrator"] --> IntentRec
    IntentRec["IntentRecognizer LLM JSON"] --> ConfirmEvt
    ConfirmEvt["confirmation event"] --> ReactUI
    ReactUI -->|invoke confirm_response| TauriCmds
    TauriCmds --> Loop
    subgraph Loop ["Step loop"]
      direction TB
      Scanner["UiaScanner uiautomation crate"] --> Decision
      Decision["DecisionEngine + Templates"] --> Safety
      Safety["SafetyGuard reject automation"] --> Channel
      Channel["Tauri Channel emit step"] --> ReactUI2
      ReactUI2["FeedbackPanel + OverlayCanvas"] --> Detector
      Detector["StateDetector re-scan"] --> Loop
    end
    Loop --> Done["goal_completed event"]
    Settings["Settings figment env"] -.-> Orchestrator
    Logger["tracing"] -.-> Orchestrator
```

## File Structure

Repo root after rewrite:

```
.
├── package.json                # React + Vite + Tauri CLI scripts
├── pnpm-lock.yaml              # or package-lock.json
├── vite.config.ts
├── tsconfig.json
├── index.html
├── src/                        # React frontend
│   ├── main.tsx
│   ├── App.tsx                 # Routes between MainScreen + OverlayCanvas based on window label
│   ├── theme.css               # WCAG AAA palette, 22pt+ typography, focus rings
│   ├── i18n.ts                 # ES/EN catalog + t() helper
│   ├── types.ts                # Shared types matching Rust serde structs
│   ├── tauri-api.ts            # invoke + listen wrappers, single source of IPC strings
│   ├── components/
│   │   ├── MainScreen.tsx
│   │   ├── ConfirmationModal.tsx   # giant YES/NO; uses native <dialog>
│   │   ├── FeedbackPanel.tsx
│   │   └── OverlayCanvas.tsx       # pulsing anchor on the transparent window
│   └── hooks/
│       ├── useOrchestrator.ts      # invoke run_session + listen events
│       └── useTranslation.ts
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json         # main window + overlay window definitions
│   ├── build.rs
│   ├── icons/
│   ├── capabilities/
│   │   └── default.json        # core:default + window:default + (no plugins yet)
│   └── src/
│       ├── main.rs             # thin: app_lib::run()
│       ├── lib.rs              # builder + state + generate_handler!
│       ├── settings.rs         # Settings (env-loaded)
│       ├── i18n.rs             # ES/EN strings + t()
│       ├── safety.rs           # SafetyGuard, GuideAction, ActionType, UnsafeActionError
│       ├── prompts.rs          # embed!(intent_classifier.txt etc.) and render
│       ├── llm/
│       │   ├── mod.rs
│       │   ├── client.rs       # LlmClient trait
│       │   ├── ollama.rs
│       │   ├── stub.rs
│       │   └── resilient.rs
│       ├── accessibility/
│       │   ├── mod.rs
│       │   ├── element.rs      # UiElement, UiSnapshot
│       │   ├── scanner.rs      # Scanner trait + factory
│       │   ├── windows.rs      # uiautomation impl (cfg(windows))
│       │   └── stub.rs
│       ├── orchestration/
│       │   ├── mod.rs
│       │   ├── templates.rs    # in-code default registry + JSON loader
│       │   ├── intent.rs
│       │   ├── decision.rs
│       │   ├── state.rs        # SessionState, GuideStep, Intent
│       │   ├── detector.rs     # StateDetector
│       │   └── orchestrator.rs # tokio task driving classify -> confirm -> step loop
│       ├── overlay.rs          # show_overlay_anchor / clear_overlay commands
│       └── commands.rs         # public Tauri commands; thin wrappers
├── docs/superpowers/...        # specs + plans
├── PRD.md                      # unchanged
├── README.md                   # rewritten for Rust + Tauri
├── .gitignore                  # extended for /target /node_modules /.next etc.
└── .agents/                    # unchanged
```

The Python tree (`app/`, `tests/`, `requirements.txt`, `pyproject.toml`,
`.venv/`, `.env.example` Python references, `app.egg-info`) gets removed in
the final phase, after the Rust app is verified working.

## Component Boundaries

### `settings`
`Settings` struct with defaults matching the prior `.env.example`. Reads
`OLLAMA_HOST`, `LLM_MODEL`, `LLM_TEMPERATURE`, `LLM_MAX_TOKENS`,
`UI_LANGUAGE`, `OVERLAY_OPACITY`, `OVERLAY_FPS`, `LOG_LEVEL` via `std::env`
or a small custom loader (no extra crate needed for ~10 fields). Provided
to other modules through `tauri::State<Mutex<Settings>>`.

### `i18n`
Two `&'static str` catalogs (ES and EN) compiled in. `pub fn t(key: &str, lang: Lang, args: &[(&str, &str)]) -> String` does substring `{name}` substitution.

### `safety`
`SafetyGuard::review(action)` returns `Result<GuideAction, UnsafeActionError>`. Closed allow-list: `Highlight | Anchor | Arrow | Speak | ShowText | Scan`. Anything else yields `UnsafeActionError("…")`.

### `llm`
- `trait LlmClient: Send + Sync { fn name(&self) -> &str; async fn health_check(&self) -> bool; async fn complete_text(&self, prompt: &str, system: Option<&str>) -> Result<String>; async fn complete_json(&self, prompt: &str, system: Option<&str>) -> Result<serde_json::Value>; }`.
- `OllamaClient` uses `reqwest::Client` with a `tokio` runtime, posts to `/api/chat` with `format: "json"` for the JSON variant.
- `StubClient` regex-matches utterances → canned `Intent` JSON.
- `ResilientClient<P, F>` wraps two clients; on `Err` from primary it sets a flag and routes future calls to the fallback for the rest of the session.

### `accessibility`
- `UiElement { ty, text, x, y, width, height, automation_id, window }` (camelCase serde for the JS boundary).
- `UiSnapshot { window, elements: Vec<UiElement> }` with `.find()` / `.find_all()` helpers.
- `trait Scanner { fn name(&self) -> &str; fn snapshot(&self) -> UiSnapshot; }`.
- `WindowsScanner` uses `uiautomation::UIAutomation` to walk the foreground window, filter to interactable controls, return a `UiSnapshot`.
- `StubScanner` returns deterministic Explorer/Chrome/Gmail/Word elements (mirrors Python).

### `orchestration`
- `Intent`, `GuideStep`, `SessionState`, `SessionStore` (the `Mutex<AppState>` holder).
- `templates::default_registry()` + `TemplateRegistry::from_json_dir()` — load the same JSON files the Python app used (we copy them to `src-tauri/templates/`).
- `IntentRecognizer::recognise(utterance) -> Intent` — calls `LlmClient::complete_json`, parses, validates against the registry, falls back to `Intent::unknown` on any failure.
- `DecisionEngine::next_step(intent, template, snapshot, session) -> GuideStep` — fuzzy `find()` then token fallback, runs the result through `SafetyGuard::review`.
- `StateDetector::is_completed(step, before, after)` — three heuristics from the Python version.
- `Orchestrator::run(utterance, channel)` — drives the whole loop on a `tokio::task::spawn`. Emits typed events on a `tauri::ipc::Channel<OrchestratorEvent>`.

### Tauri command surface
- `classify(utterance: String) -> Result<ClassifyOutput, AppError>` — quick echo of intent + confirmation message for the modal.
- `start_session(utterance: String, on_event: Channel<OrchestratorEvent>) -> Result<(), AppError>` — kicks off the step loop on a tokio task.
- `confirm_response(accepted: bool) -> Result<(), AppError>` — resolves the confirmation gate inside the running session.
- `cancel_session() -> Result<(), AppError>` — sets the cancellation flag on the orchestrator.
- `show_overlay_anchor(x: i32, y: i32, label: String) -> Result<(), AppError>` — invokes the overlay window via `Manager::get_webview_window("overlay")` and emits an `anchor` event into it.
- `clear_overlay() -> Result<(), AppError>`.

All commands return `Result<_, AppError>` where `AppError` derives `thiserror::Error + serde::Serialize`.

### Channel event types (all `#[serde(tag = "kind", content = "data")]`)

```rust
enum OrchestratorEvent {
    ConfirmationRequested { message: String },
    StepReady { step: GuideStep },
    AnchorChanged { x: i32, y: i32, label: String },
    GoalCompleted { steps: usize },
    Error { message: String },
    Finished,
}
```

### React frontend

- `App.tsx` checks `window.__TAURI__.window.getCurrent().label` and renders either `MainScreen` (label `"main"`) or `OverlayCanvas` (label `"overlay"`).
- `useOrchestrator.ts` exposes `submit(utterance)` and a state object `{ phase, step, error, lastConfirmation }`. Internally it `invoke('start_session', { utterance, onEvent: channel })` and dispatches reducer actions on each event.
- `MainScreen.tsx`: heading, large `<input>` with `aria-label`, "Empezar" button, feedback card, `ConfirmationModal` rendered conditionally.
- `ConfirmationModal.tsx`: native `<dialog>` element (focus-trapped for free), giant green/red buttons (≥160px tall, 120pt text), keyboard shortcuts Y/Enter and N/Esc.
- `FeedbackPanel.tsx`: `aria-live="polite"` region; renders the active step or success/error states.
- `OverlayCanvas.tsx`: `<canvas>` filling the overlay window; on `anchor` event from Rust, draws a pulsing circle + label using `requestAnimationFrame` at `OVERLAY_FPS`.

### Overlay window in Tauri config

```jsonc
{
  "label": "overlay",
  "title": "Roota Overlay",
  "fullscreen": true,
  "transparent": true,
  "alwaysOnTop": true,
  "decorations": false,
  "skipTaskbar": true,
  "resizable": false,
  "focus": false,
  "visible": false
}
```

After it's shown, we call `webview.set_ignore_cursor_events(true)` from Rust so all input passes through. The frontend sets `<html>` and `<body>` background to `transparent` and the canvas alpha-blends.

## Safety Contract

The contract is identical to the Python version — `SafetyGuard` is the chokepoint and the codebase imports nothing that synthesises input. We deliberately do **not** depend on `enigo`, `windows::Win32::UI::Input::KeyboardAndMouse`, or any keyboard/mouse driver crate. The `uiautomation` crate offers automation methods that *could* click; we only expose its read methods through a `Scanner` trait and never construct an `Element::click()` call anywhere in the codebase. A `cargo deny` check (or a simple grep CI gate) enforces the prohibition.

## Observability

`tracing` + `tracing-subscriber` writes to a rotating file under `logs/` plus stderr. Log entries never include the user's utterance verbatim at INFO level — privacy by default.

## Testing Strategy

Rust unit tests live next to the modules (`#[cfg(test)] mod tests { ... }`):
- `safety::tests` — every guide action passes, every automation action raises.
- `i18n::tests` — fallback to Spanish, format substitution.
- `accessibility::stub::tests` — deterministic snapshot, fuzzy `find`.
- `llm::stub::tests` — every prior Python regex case (open_folder, move_file, …).
- `llm::resilient::tests` — primary error → fallback used; reset re-enables primary.
- `orchestration::intent::tests` — known intent resolves, unknown intent stays `unknown`, JSON parse errors handled.
- `orchestration::decision::tests` — fuzzy match, missing element, multiple steps.
- `orchestration::detector::tests` — target disappeared, window changed, no-change.
- `orchestration::orchestrator::tests` — full loop with stub LLM + stub scanner.

Frontend smoke tested by `npm run build` succeeding plus a single Vitest case
that the i18n catalog contains the same keys as the Rust catalog (file
parity via JSON dump).

## Definition of Done

- `cargo check`, `cargo test`, `cargo clippy --no-deps -- -D warnings` all green.
- `npm run build` produces `dist/` without errors.
- `npm run tauri build` produces a `.exe` and an `.msi` installer.
- `npm run tauri dev` launches the app on Windows; user types a Spanish command, sees the giant YES/NO modal, on YES sees a pulsing anchor on the target element in File Explorer, advances when the user opens the folder.
- The Python implementation is removed from the working tree.
- `README.md` reflects the Rust + Tauri reality.

## Risks and Mitigations

- **MSVC install lag:** Tauri's first build can take 10+ minutes. Mitigation: warm the build by running `cargo check` immediately after MSVC finishes.
- **`uiautomation` crate Spanish text encoding:** UIA returns UTF-16 from the OS; the Rust crate decodes to `String`. Verified working with non-ASCII strings on Windows 11. Mitigation: targeted unit test with a stubbed snapshot containing "Imágenes" / "Descargas".
- **Transparent window on Windows:** Some GPU drivers blank out transparent windows under DWM. Mitigation: set `tauri.conf.json` `windows.decorations: false` + the WebView's body background to `transparent` and verify on the demo box; fall back to a 1% alpha background if pure transparent fails.
- **Channel-based step streaming vs request-response:** A Tauri `Channel` is the right abstraction (typed, fire-and-forget, async). Mitigation: hold the `Mutex<Option<oneshot::Sender<bool>>>` for the confirmation gate so the loop pauses cleanly until the frontend responds.
- **Build time on the demo box:** First `cargo build --release` may take 8-15 min. Mitigation: cache `target/` between runs; we are not on a CI fresh checkout.
