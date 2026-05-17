# Roota Rust + Tauri Rewrite — Implementation Plan

> **Spec:** [`docs/superpowers/specs/2026-05-16-roota-rust-tauri-rewrite.md`](../specs/2026-05-16-roota-rust-tauri-rewrite.md)

## Goal

Replace the Python prototype with a native Rust + Tauri 2 + React 18 + Vite
app that ships the same guide-only Roota experience: classify → confirm →
guide via transparent overlay, all running on local Ollama.

## Phases

### Phase A — Toolchain + scaffold

- [ ] Verify rustup/rustc/cargo on PATH after `winget install Rustlang.Rustup`.
- [ ] Wait for MSVC C++ Build Tools to finish installing (background).
- [ ] `npm install -D @tauri-apps/cli@^2 @tauri-apps/api@^2 vite typescript @vitejs/plugin-react react react-dom @types/react @types/react-dom`.
- [ ] Author `package.json`, `vite.config.ts`, `tsconfig.json`, `index.html`, `src/main.tsx`, `src/App.tsx` (placeholder).
- [ ] Create `src-tauri/` manually: `Cargo.toml`, `tauri.conf.json` (main window + overlay window), `build.rs`, `capabilities/default.json`, `src/main.rs`, `src/lib.rs` (greet command).
- [ ] Run `cargo check` from `src-tauri/` — expect compile success once MSVC is installed.
- [ ] Run `npm run tauri dev` — expect window with a placeholder React shell.

### Phase B — Domain + Config + Safety

- [ ] `src-tauri/src/settings.rs` with `Settings::from_env()`.
- [ ] `src-tauri/src/i18n.rs` with `Lang`, `t(key, lang, args)`, ES + EN catalogs.
- [ ] `src-tauri/src/safety.rs` with `ActionType` enum (closed allow-list + automation list), `GuideAction`, `SafetyGuard`, `UnsafeActionError`. Unit tests: every guide action passes; every automation action fails; unknown action fails in strict mode.
- [ ] `src-tauri/src/state.rs` (or `orchestration/state.rs`): `Intent`, `GuideStep`, `SessionState`, `SessionStore` (Mutex-wrapped state).
- [ ] Run `cargo test` — expect green.

### Phase C — LLM (Ollama via reqwest, stub fallback)

- [ ] Add `reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }` and `tokio = { version = "1", features = ["rt-multi-thread", "macros", "sync", "time"] }` to `Cargo.toml`.
- [ ] `llm/client.rs` with `LlmClient` async trait (uses `async_trait` or native AFIT — Rust 1.95 has stable AFIT, prefer native).
- [ ] `llm/ollama.rs` — `reqwest::Client` calling `/api/chat` and `/api/tags`. JSON variant uses `format: "json"`.
- [ ] `llm/stub.rs` — regex matchers ported from Python: open_folder, move_file, delete_file, open_browser, search_web, compose_email, etc.
- [ ] `llm/resilient.rs` — wraps two clients; on `Err` from primary, set `primary_healthy = false` (Mutex), route to fallback for the rest of the session; expose `reset()`.
- [ ] Unit tests using a `MockLlm` and a `FailingLlm`.
- [ ] Manual sanity: `cargo run --example llm_smoke` (or a one-shot binary in `bin/`) calls `OllamaClient::complete_text("hola")` and prints the result.

### Phase D — Accessibility scanner

- [ ] Add `uiautomation = "0.20"` (current Windows wrapper) to `[target.'cfg(windows)'.dependencies]`.
- [ ] `accessibility/element.rs` — `UiElement`, `UiSnapshot`, `find`, `find_all`, `matches`.
- [ ] `accessibility/stub.rs` — deterministic snapshot mirroring Python.
- [ ] `accessibility/windows.rs` (cfg(windows)) — `WindowsScanner::snapshot` walks the foreground window via `UIAutomation::create()` + `get_focused_element` + `find_all_descendants`, filters to interactable control types (Button/MenuItem/ListItem/TreeItem/TabItem/Hyperlink/Edit/ComboBox/CheckBox/RadioButton).
- [ ] `accessibility/scanner.rs` — `Scanner` trait + `get_scanner()` factory choosing platform.
- [ ] Tests: stub snapshot deterministic; element matching.

### Phase E — Orchestration brain

- [ ] `orchestration/templates.rs` — `StepBlueprint`, `GuidanceTemplate`, in-code `default_registry()` covering all 11 intents from the Python version. JSON loader reads from `src-tauri/templates/*.json` (we copy the Python JSON files verbatim).
- [ ] `orchestration/intent.rs` — `IntentRecognizer::recognise(utterance)`; calls `LlmClient::complete_json` with the embedded `intent_classifier.txt` prompt.
- [ ] `orchestration/decision.rs` — `DecisionEngine::next_step` (fuzzy + token fallback + safety review).
- [ ] `orchestration/detector.rs` — `StateDetector::is_completed`.
- [ ] `orchestration/orchestrator.rs` — drives `tokio::spawn` task; emits typed events on a `tauri::ipc::Channel<OrchestratorEvent>`; awaits a `oneshot::Receiver<bool>` for the confirmation gate.
- [ ] Tests using the stub LLM + stub scanner that drive the full happy path and the user-cancels path.

### Phase F — Tauri commands and wiring

- [ ] `commands.rs` exposing `classify`, `start_session`, `confirm_response`, `cancel_session`, `show_overlay_anchor`, `clear_overlay`.
- [ ] `lib.rs::run()` constructs the `LlmClient` (Ollama → stub fallback), the `Scanner`, the `Orchestrator`, and registers them in `Builder::default().manage(...)` + `generate_handler![...]`.
- [ ] Update `capabilities/default.json` with `core:default` plus `core:webview:allow-set-ignore-cursor-events` (or whichever permission the v2 API requires for the overlay).

### Phase G — React frontend

- [ ] `src/theme.css` — WCAG AAA palette and large typography (mirrors Python `theme.py`).
- [ ] `src/i18n.ts` — same keys as the Rust catalog.
- [ ] `src/types.ts` — TypeScript types matching Rust serde structs.
- [ ] `src/tauri-api.ts` — single source for command names + event names.
- [ ] `src/hooks/useOrchestrator.ts` — `submit(utterance)`, reducer over event stream, `confirm(accepted)`.
- [ ] `src/components/MainScreen.tsx` — layout per spec.
- [ ] `src/components/ConfirmationModal.tsx` — `<dialog>` element with focus trap; YES/NO buttons; keyboard shortcuts.
- [ ] `src/components/FeedbackPanel.tsx` — `aria-live="polite"` region.
- [ ] `src/App.tsx` — routes between main and overlay views by window label.
- [ ] `npm run build` succeeds.

### Phase H — Overlay window

- [ ] Add `overlay` window to `tauri.conf.json` per spec.
- [ ] `overlay.rs::show_overlay_anchor` calls `app.get_webview_window("overlay")` and emits `roota://anchor` event into it; ensures `set_ignore_cursor_events(true)` and `show()` on first use.
- [ ] `src/components/OverlayCanvas.tsx` listens to the anchor event and draws a pulsing circle on a transparent `<canvas>`.
- [ ] Manual run verifies the overlay is click-through, always-on-top, and renders the anchor.

### Phase I — Verify, delete Python, document

- [ ] `cargo fmt && cargo clippy --no-deps -- -D warnings && cargo test` all green.
- [ ] `npm run build` succeeds.
- [ ] `npm run tauri dev` smoke run on Windows.
- [ ] Delete Python tree: `app/`, `tests/`, `pyproject.toml`, `requirements.txt`, `.venv/`, plus any `*.egg-info`.
- [ ] Rewrite `README.md` for the Rust + Tauri stack (prereqs, setup, demo flow, troubleshooting).
- [ ] Commit history is the user's call (we leave the working tree dirty for them to commit).

## Risks (echoed from spec for execution-time awareness)

- MSVC install latency — block Phase A until done.
- First Rust build may take 8-15 min — start it as a background task and continue editing while it compiles.
- Transparent window may need a fallback strategy — verified in Phase H manual smoke.
- `uiautomation` Windows-only — non-Windows builds use the stub; tested on this Windows 11 box.
