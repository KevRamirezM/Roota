# Perception OCR-First Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make hybrid perception fast and reliable by using native Windows OCR as the default pixel layer, gating slow Moondream VLM behind an opt-in flag, and improving primary-window selection for overlay HWNDs.

**Architecture:** `LayeredVisionPerceiver` runs `WindowsOcrPerceiver` first (WinRT `Media.Ocr`, ~100–300 ms). Moondream runs only when `ROOTA_VISION_VLM=1` and OCR returns fewer than 2 elements. Window scoring penalizes system overlay titles (Input Experience). `ScanContext` extracts app hints from utterances (e.g. "cursor").

**Tech Stack:** Rust, `windows` 0.61 WinRT OCR, existing `xcap` capture, Ollama Moondream optional.

---

### Task 1: Settings and vision factory

**Files:**
- Modify: `src-tauri/src/settings.rs`
- Modify: `src-tauri/src/perception/vision/mod.rs`

- [ ] Add `vision_vlm_enabled: bool` (default `false`), env `ROOTA_VISION_VLM`
- [ ] Lower defaults: `vision_max_edge=512`, `vision_timeout_secs=45` (VLM only)
- [ ] Add `LayeredVisionPerceiver` + update `default_vision_perceiver`

### Task 2: Windows.Media.Ocr adapter

**Files:**
- Modify: `src-tauri/Cargo.toml` (WinRT features)
- Modify: `src-tauri/src/perception/vision/ocr_windows.rs`

- [ ] Implement RGBA→BGRA, `SoftwareBitmap`, `RecognizeAsync().get()`
- [ ] Map words to `ScreenElement { source: Ocr }` in screen coords
- [ ] Unit test: coord mapping helper (no live OCR)

### Task 3: Window scoring + scan hints

**Files:**
- Modify: `src-tauri/src/perception/window_score.rs`
- Modify: `src-tauri/src/accessibility/scanner.rs`
- Modify: `src-tauri/src/orchestration/orchestrator.rs`

- [ ] Penalize junk overlay windows (Input Experience, tiny chrome)
- [ ] `ScanContext::enrich_from_utterance` for cursor/chrome/terminal
- [ ] Tests for junk penalty and cursor hint ranking

### Task 4: Verify

- [ ] `cargo test` in `src-tauri`
- [ ] Update `.env` comments via settings defaults (user file optional)
