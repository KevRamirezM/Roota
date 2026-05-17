# Roota — Universal Windows Perception (Design Spec)

**Date:** 2026-05-18  
**Revision:** 2026-05-18b (review fixes: types, API, window ordering, fusion ownership)  
**Status:** Draft — ready for implementation  
**Related:** PRD §8.1 (Accessibility), §8.9 (Safety), Phase 3 roadmap  
**Goal:** Let Roota guide the user in **any visible app, anywhere on Windows** (all monitors, taskbar, dialogs) using a **read-only hybrid perception stack** — UI Automation first, optional vision fallback — while **never** automating clicks or keyboard input.

---

## Problem

Today Roota scans **one foreground window** via UI Automation (`WindowsScanner` → `resolve_target_window` → descendants). That works well for File Explorer when it is focused and exposes a rich accessibility tree, but fails or degrades when:

| Situation | Why current scanner fails |
|-----------|---------------------------|
| Target is in a **background** window | Foreground is another app; hints miss the window |
| User works on a **second monitor** | Window rect / focus logic may not match cursor |
| App has **weak or empty UIA** (games, some Electron, custom paint) | `elements=0`, no anchor |
| Target is in **Start menu / taskbar / system tray** | Not inside a single “app” window title |
| **Modal dialog** on top of app | Dialog tree not merged with parent |
| **Roota panel** steals focus briefly | Mitigated by filters, but perception still single-window |

The user asked for guidance that “sees everything on screen” without Roota **doing** things for them. That implies:

1. **Observe** broadly (multi-window + optional pixels).
2. **Coach** from cursor + clicks + screen state (already started in `input/` + `action_feedback/`).
3. **Never execute** OS actions (PRD §8.9 — unchanged).

Pixel-perfect OCR/VLM is explicitly a **separate layer**, not a replacement for UIA — it fills gaps when the accessibility tree is empty or untrusted.

---

## Success criteria

1. **Multi-app:** With Chrome + Explorer open, a step targeting “Descargas” finds the element in the correct window even if Explorer is not foreground (within configured limits).
2. **Multi-monitor:** Anchors and click feedback use **physical screen coordinates** consistent with the existing overlay converter (`overlay.rs`).
3. **Desktop chrome:** Can perceive and guide to taskbar / Start (via desktop-root UIA walk), documented limitations for secure desktop.
4. **Hybrid fallback:** When UIA returns fewer than `ROOTA_MIN_UIA_ELEMENTS` **interactable elements inside the primary window’s client rect** (not global count across all HWNDs), OCR proposes text boxes; fusion dedupes and ranks targets.
5. **Guide-only:** No new code paths call `SendInput`, `mouse_event`, shell execute for user tasks, or `launch_*` helpers.
6. **Performance:** Full hybrid capture ≤ 800 ms on mid-range laptop at default settings; UIA-only path ≤ 400 ms.
7. **Tests:** Stub + unit tests for window scoring, fusion, and orchestrator integration without a live desktop.

---

## Non-goals (v1 of this feature)

- Cloud vision APIs or uploading screenshots off-device (PRD privacy).
- Automating clicks, typing, or opening apps for the user.
- Perfect perception inside **elevated / secure desktop** (UAC prompts, login screen) — detect and show a calm error.
- Real-time 30 FPS full-screen OCR (batch capture on step boundaries only).

---

## Architecture overview

```text
┌─────────────────────────────────────────────────────────────┐
│                     Orchestrator (existing)                  │
│  perceive → decide → overlay → poll(input + screen delta)   │
└───────────────────────────┬─────────────────────────────────┘
                            │ ScreenFrame
┌───────────────────────────▼─────────────────────────────────┐
│         perception::HybridPerceiver (owns FusionEngine)      │
│   capture(PerceptionContext) -> Result<ScreenFrame, Error>   │
└───────────────┬─────────────────────────────┬───────────────┘
                │                             │
     ┌──────────▼──────────┐       ┌──────────▼──────────┐
     │  UiaPerceiver       │       │  VisionPerceiver     │
     │  multi-window UIA   │       │  screenshot + OCR    │
     │  desktop + modals   │       │  (optional, cfg)     │
     └──────────┬──────────┘       └──────────┬──────────┘
                │                             │
                └──────────┬──────────────────┘
                           ▼
              FusionEngine::fuse (owned by HybridPerceiver)
                           │
                           ▼
                    ScreenFrame { elements[], windows[], cursor, warnings[], ... }
```

**Fusion ownership:** `HybridPerceiver` is the only type that constructs and calls `FusionEngine`. `UiaPerceiver` and `VisionPerceiver` return **partial** element lists; they never write `ScreenFrame.quality` or merge layers. This keeps fusion rules in one place and avoids double-merge when OCR is disabled.

### Core types (new module `src-tauri/src/perception/`)

```rust
/// One capture cycle — replaces bare UiSnapshot at orchestration boundaries.
pub struct ScreenFrame {
    /// Wall-clock millis since UNIX epoch (serde/log friendly). Do NOT use Instant here.
    pub captured_at_ms: u64,
    pub cursor: PhysicalPoint,           // from input::InputMonitor
    pub primary_window_id: WindowId,
    pub windows: Vec<WindowSnapshot>,
    pub elements: Vec<ScreenElement>,    // unified, screen-space bounds
    pub quality: PerceptionQuality,      // Full | DegradedUiaOnly | VisionAssisted
    /// Non-fatal issues (secure desktop, OCR timeout, modal attach failed, etc.)
    pub warnings: Vec<PerceptionWarning>,
}

pub enum PerceptionWarning {
    SecureDesktop,
    OcrUnavailable,
    ModalAttachFailed { dialog_title: String },
    WindowCapTruncated { total_visible: u32, scanned: u32 },
    LowElementCount { window_id: WindowId, count: usize },
}

pub struct WindowSnapshot {
    pub id: WindowId,
    pub title: String,
    pub class_name: String,
    pub bounds: Rect,                    // physical screen rect
    pub is_foreground: bool,
    pub z_order: u32,
    pub uia_element_count: usize,
}

pub struct ScreenElement {
    pub source: ElementSource,           // Uia | Ocr | Fused
    pub text: String,
    pub bounds: Rect,
    pub window_id: WindowId,
    pub kind: String,                    // Button, Text, ...
    pub confidence: f32,                 // 1.0 for UIA; OCR lower
}

pub enum ElementSource { Uia, Ocr, Fused }

pub enum PerceptionQuality {
    Full,              // primary window has enough UIA targets
    DegradedUiaOnly,   // sparse tree; no OCR attempted or OCR skipped
    VisionAssisted,    // OCR/VLM contributed elements
}
```

`ScreenFrame` implements the same **query helpers** now on `UiSnapshot` (`find_best_for_action`, `visible_summary`) on `impl ScreenFrame` directly. **Do not** add a production `From<&ScreenFrame> for UiSnapshot` adapter — it hides missing `window_id` / `warnings` and lets tests pass without real migration.

### Perceiver API (fallible, async-ready)

```rust
pub trait Perceiver: Send + Sync {
    fn name(&self) -> &str;
    /// Blocking capture; orchestrator calls via `tokio::task::spawn_blocking` in P1.
    /// P3 may add `async fn capture_async` once OCR/VLM need await internally.
    fn capture(&self, ctx: &PerceptionContext) -> Result<ScreenFrame, PerceptionError>;
}
```

On failure, orchestrator emits `OrchestratorEvent::Error` with i18n (`guidance.perception_failed` or specific `PerceptionWarning` mapping) and **does not** reuse a stale frame for anchor placement.

### Window selection (replaces single `resolve_target_window`)

**Ordering (critical):** enumerate **all** visible top-level HWNDs → score **every** candidate → sort by score descending → take top `ROOTA_MAX_WINDOWS` (default 8) for UIA walks. Never cap the candidate list before scoring (that drops the correct background window).

| Signal | Weight | Notes |
|--------|--------|-------|
| Foreground HWND | +40 | Still important but not sole signal |
| Cursor inside window rect | +35 | “User is working here” |
| Title matches `ScanContext.window_hints` | +50 | Intent/template driven |
| Not Roota / overlay | hard filter | Existing `ROOTA_TITLE_MARKERS` |
| UIA element count &gt; 0 | +10 | Prefer scannable windows (count from quick probe or last frame) |
| Modal owned by hinted app | +30 | Attach dialog HWND to parent |

Pick **primary** = highest score after sort; **also ingest** elements from all top-K ranked windows into `ScreenFrame.elements` (each tagged with `window_id`).

**Modal fallback:** If `GetWindow(GW_OWNER)` / UIA parent lookup fails, treat the dialog as its own ranked window (do not merge). Push `PerceptionWarning::ModalAttachFailed` and continue; decision may still find controls inside the dialog title.

### Vision layer (Phase 3+)

1. Capture bitmap for union of primary window rect + 32px margin (or monitor under cursor).
2. Run **Windows.Media.Ocr** (WinRT) or embedded `ocrs` / Tesseract — **offline only**.
3. Emit `ScreenElement { source: Ocr, text, bounds, confidence }`.
4. **Fusion:** If UIA box overlaps OCR text (IoU &gt; 0.5), merge into `Fused` with UIA kind + OCR text; if UIA empty in region, keep OCR-only candidates.

Optional later: local **Ollama vision** (`llava`) for icon-only targets — `Could Have`, behind `ROOTA_VISION_LLM=1`.

### Safety & privacy

- Screenshots stay **in-process RAM**; never written to disk unless `ROOTA_DEBUG_CAPTURE=1` in dev.
- OCR strings fed to local LLM only (existing Ollama path).
- `SafetyGuard` unchanged — no automation actions.
- Telemetry logs counts only (`elements`, `windows`, `quality`), not screenshot bytes.

### Settings (`settings.rs`)

| Env var | Default | Meaning |
|---------|---------|---------|
| `ROOTA_PERCEPTION_MODE` | `hybrid` | `uia` \| `hybrid` \| `vision_only` (dev) |
| `ROOTA_MAX_WINDOWS` | `8` | Cap EnumWindows processing |
| `ROOTA_VISION_ENABLED` | `1` | Enable OCR fallback in hybrid |
| `ROOTA_OCR_LANGUAGE` | `es` | OCR language tag |
| `ROOTA_CAPTURE_SCALE` | `0.75` | Downscale before OCR |
| `ROOTA_MIN_UIA_ELEMENTS` | `3` | Below this **in primary window client rect**, trigger vision for that HWND only |
| `ROOTA_PROMPT_MAX_ELEMENTS` | `40` | Max labels in LLM `visible_elements` |
| `ROOTA_PROMPT_MAX_WINDOWS` | `3` | Max window titles in `{window_list}` |

### Integration points (existing files)

| File | Change |
|------|--------|
| `accessibility/scanner.rs` | Deprecate direct orchestrator use; wrap as `UiaPerceiver` |
| `accessibility/windows.rs` | Split: low-level HWND/UIA helpers |
| `perception/mod.rs` | New: trait, fusion, factory |
| `orchestration/orchestrator.rs` | `capture_frame` instead of `snapshot_with_context` |
| `orchestration/decision.rs` | Accept `&ScreenFrame` (or adapter) |
| `orchestration/detector.rs` | Compare frames across windows |
| `orchestration/action_feedback.rs` | Already uses screen coords — verify with multi-monitor tests |
| `prompts/instruction_step.txt` | `{window_list}` capped at `ROOTA_PROMPT_MAX_WINDOWS` (3); `visible_elements` capped at `ROOTA_PROMPT_MAX_ELEMENTS` (40); include `{perception_quality}` + one-line `warnings` summary when non-empty |
| `commands.rs` / overlay | No change to emit path if `GuideStep` unchanged |

### Frontend

No mandatory UI for v1. Optional dev panel: “Calidad de lectura de pantalla: buena / limitada” when `quality != Full`.

---

## Phased delivery

| Phase | Scope | User-visible outcome |
|-------|--------|----------------------|
| **P1** | Multi-window UIA + cursor-aware window pick | Works across most Win32/WPF/WinUI apps when trees exist |
| **P2** | Desktop/taskbar + modal attachment | Start menu, taskbar pins, app dialogs |
| **P3** | OCR hybrid fallback | Canvas-light / poor-a11y apps get text targets |
| **P4** | Perf + caching + stress tests | Stable polling during guide loop |
| **P5** | Optional local VLM icons | “Click the red trash icon” when no text label |

---

## Risks & mitigations

| Risk | Mitigation |
|------|------------|
| OCR latency | Run only when primary-window UIA sparse; cache with explicit invalidation (see plan Task 16) |
| Wrong window selected | Cursor + hints + user correction messages |
| DPI / mixed scaling | Always physical coords; reuse `overlay.rs` math |
| Antivirus flags screen capture | Document; use PrintWindow before BitBlt fallback |
| UIA offscreen elements | Filter `IsOffscreen` / zero-size bounds |

---

## Acceptance test script (manual)

1. Open Explorer (Downloads) + Chrome side by side on two monitors.
2. Ask: “Abre la carpeta Descargas” → confirm SÍ.
3. **Without focusing Explorer**, follow overlay — anchor should appear on Downloads.
4. Mis-click outside target — corrective Spanish message.
5. Open Notepad (poor tree) with hybrid on — OCR should still find menu text labels.
6. Confirm Roota never launches apps or moves the mouse.

---

## Resolved defaults (formerly open questions)

1. **Full virtual desktop OCR vs primary monitor?** Monitor containing cursor only (faster). Log `PerceptionWarning` if capture rect crosses monitors.
2. **LLM prompt size?** `ROOTA_PROMPT_MAX_ELEMENTS=40`, `ROOTA_PROMPT_MAX_WINDOWS=3` — enforced in `ScreenFrame::visible_summary` / `window_list_for_prompt`, not ad hoc in orchestrator.
3. **Intent-specific templates still required?** Yes — perception is generic; templates/LLM still shape steps.
4. **`Instant` vs wall-clock?** `captured_at_ms: u64` only — `Instant` is not serialized, not logged across threads, and must not appear on `ScreenFrame`.
