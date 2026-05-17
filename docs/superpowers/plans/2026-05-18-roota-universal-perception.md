# Universal Windows Perception — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace single-window UIA snapshots with a hybrid **ScreenFrame** pipeline so Roota can guide in any app/region on Windows — read-only, guide-only, PRD §8.9 safe.

**Architecture:** `HybridPerceiver` owns `FusionEngine` and composes `UiaPerceiver` + optional `VisionPerceiver`. Window scoring runs on **all** visible HWNDs before top-K cap. `Perceiver::capture` returns `Result<ScreenFrame, PerceptionError>`; orchestrator calls it via `spawn_blocking` in P1. Orchestrator, decision, detector, and LLM prompts consume `ScreenFrame` directly — **no** production `UiSnapshot` adapter.

**Revision:** 2026-05-18b — aligns with spec review fixes (types, fallible API, ordering, cache, deps).

**Tech Stack:** Rust (Tauri 2), `uiautomation`, `windows` crate (Win32 + optional WinRT OCR), existing `input::InputMonitor`, local Ollama for instructions.

**Design spec:** [`docs/superpowers/specs/2026-05-18-roota-universal-perception-design.md`](../specs/2026-05-18-roota-universal-perception-design.md)

---

## File map (new / modified)

| Path | Responsibility |
|------|----------------|
| `src-tauri/src/perception/mod.rs` | Module root, `get_perceiver()` factory |
| `src-tauri/src/perception/frame.rs` | `ScreenFrame`, `ScreenElement`, `WindowSnapshot`, `Rect` |
| `src-tauri/src/perception/context.rs` | `PerceptionContext` (hints, cursor, settings) |
| `src-tauri/src/perception/window_enum.rs` | `EnumWindows` + HWND metadata (cfg windows) |
| `src-tauri/src/perception/window_score.rs` | Scoring table from design spec |
| `src-tauri/src/perception/uia.rs` | Multi-window UIA walk |
| `src-tauri/src/perception/hybrid.rs` | `HybridPerceiver` — owns `FusionEngine`, wires UIA + vision |
| `src-tauri/src/perception/fusion.rs` | `FusionEngine::fuse` only; called from `hybrid.rs` |
| `src-tauri/src/perception/error.rs` | `PerceptionError`, `PerceptionWarning` |
| `src-tauri/src/perception/vision/mod.rs` | `VisionPerceiver` trait |
| `src-tauri/src/perception/vision/capture.rs` | Monitor/window bitmap capture |
| `src-tauri/src/perception/vision/ocr_windows.rs` | WinRT OCR adapter (cfg windows) |
| `src-tauri/src/perception/stub.rs` | Deterministic multi-window fixture for tests |
| `src-tauri/src/accessibility/windows.rs` | Refactor: HWND helpers reused by `uia.rs` |
| `src-tauri/src/settings.rs` | Perception env vars |
| `src-tauri/src/orchestration/orchestrator.rs` | `capture_frame` loop |
| `src-tauri/src/orchestration/decision.rs` | `&ScreenFrame` |
| `src-tauri/src/orchestration/detector.rs` | Frame delta |
| `src-tauri/prompts/instruction_step.txt` | Multi-window context |
| `src-tauri/Cargo.toml` | `xcap` or `screenshots`, WinRT features |

---

## Phase P1 — Multi-window UIA (foundation)

### Task 1: Perception types + query helpers

**Files:**
- Create: `src-tauri/src/perception/mod.rs`
- Create: `src-tauri/src/perception/frame.rs`
- Create: `src-tauri/src/perception/context.rs`
- Modify: `src-tauri/src/lib.rs` (add `pub mod perception;`)

- [ ] **Step 1: Write failing tests**

```rust
// src-tauri/src/perception/frame.rs (#[cfg(test)])
#[test]
fn find_best_prefers_exact_text_across_windows() {
    let frame = ScreenFrame {
        elements: vec![
            screen_el("Descargas", 100, 100, 80, 24, 1),
            screen_el("Documentos", 200, 100, 80, 24, 2),
        ],
        ..ScreenFrame::empty()
    };
    let q = vec!["descargas".into()];
    let found = frame.find_best_for_action(&q, ActionVerb::Click);
    assert_eq!(found.unwrap().text, "Descargas");
}
```

- [ ] **Step 2: Run test**

Run: `cd src-tauri; cargo test find_best_prefers_exact_text_across_windows -- --nocapture`  
Expected: FAIL (module/type not found)

- [ ] **Step 3: Implement minimal types**

Implement `Rect`, `WindowId(u64)`, `ScreenElement`, `PerceptionWarning`, `ScreenFrame` with:
- `captured_at_ms: u64` (wall-clock; **not** `Instant`)
- `warnings: Vec<PerceptionWarning>` (default empty)
- `empty()`, `find_best_for_action`, `visible_summary(max_elements: usize)` capped by settings default 40
- Port matching from `accessibility/element.rs` via shared `element_match.rs` or pub(crate) re-export

- [ ] **Step 4: Run test** — Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/perception/ src-tauri/src/lib.rs
git commit -m "feat(perception): add ScreenFrame types and cross-window matching"
```

---

### Task 2: Window enumeration (Win32)

**Files:**
- Create: `src-tauri/src/perception/window_enum.rs`
- Modify: `src-tauri/Cargo.toml` — add `Win32_UI_WindowsAndMessaging` features: `EnumWindows`, `GetWindowTextW`, `GetClassNameW`, `IsWindowVisible`, `GetWindowRect`

- [ ] **Step 1: Write failing test (stub HWND list in non-windows cfg)**

```rust
#[test]
fn visible_windows_filters_roota() {
    let wins = sample_windows_fixture();
    let filtered: Vec<_> = wins.into_iter().filter(|w| !w.is_roota).collect();
    assert!(filtered.iter().all(|w| !w.title.to_lowercase().contains("roota")));
}
```

- [ ] **Step 2: Implement `list_visible_windows() -> Vec<WindowMeta>`**

On Windows: `EnumWindows` callback collecting title, class, rect, `hwnd` as `WindowId`, skip invisible/minimized zero-area, skip Roota markers.

- [ ] **Step 3: Run `cargo test window_enum`** — PASS

- [ ] **Step 4: Commit** — `feat(perception): enumerate visible top-level windows`

---

### Task 3: Window scoring (cursor + hints + foreground)

**Files:**
- Create: `src-tauri/src/perception/window_score.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn cursor_inside_adds_weight() {
    let wins = vec![meta(1, "Explorer", 0, 0, 800, 600)];
    let ranked = rank_windows(&wins, &PerceptionContext {
        cursor: PhysicalPoint { x: 100, y: 100 },
        window_hints: vec!["explorador".into()],
        ..Default::default()
    });
    assert_eq!(ranked[0].id, wins[0].id);
}
```

- [ ] **Step 2: Implement `rank_windows` per design spec weights**

- [ ] **Step 3: Test ordering invariant** — `rank_windows` receives 20 synthetic windows; output len = `min(20, ROOTA_MAX_WINDOWS)` and highest hinted window is first **even when it is not foreground**

```rust
#[test]
fn cap_applied_after_sort_not_before() {
    let ranked = rank_windows(&many_windows_fixture(20), &ctx_with_explorer_hint());
    assert!(ranked.len() <= 8);
    assert!(ranked[0].title.to_lowercase().contains("explorador"));
}
```

- [ ] **Step 4: Run tests** — PASS

- [ ] **Step 5: Commit**

---

### Task 4: Multi-window UIA perceiver

**Files:**
- Create: `src-tauri/src/perception/uia.rs`
- Modify: `src-tauri/src/accessibility/windows.rs` — extract `element_from_hwnd`, `walk_descendants` helpers

- [ ] **Step 1: Write failing integration test using `perception/stub.rs`**

Stub returns 2 windows with known elements; `UiaPerceiver` replaced by stub in test build.

- [ ] **Step 2: Implement `UiaPerceiver::capture`**

For HWNDs returned by `rank_windows` (already capped to top K **after** sort):
1. `ElementFromHandle` / uiautomation equivalent
2. Walk descendants (cap elements per window = `600 / window_count`)
3. Convert bounds to **physical screen coordinates** (add window origin)
4. Return **partial** `UiaCapture { windows, elements }` — do not set `quality` or `warnings` here

- [ ] **Step 3: Count interactable elements in **primary** client rect only** (for later OCR gate)

- [ ] **Step 4: Set `primary_window_id` from rank #1 in `HybridPerceiver`, not in `UiaPerceiver`**

- [ ] **Step 5: Run `cargo test uia`** — PASS

- [ ] **Step 6: Manual smoke**

Run app, log line: `roota.perception uia windows=3 elements=142 primary="Explorador de archivos"`.

- [ ] **Step 7: Commit**

---

### Task 5: HybridPerceiver, FusionEngine, fallible Perceiver trait

**Files:**
- Create: `src-tauri/src/perception/hybrid.rs`
- Create: `src-tauri/src/perception/error.rs`
- Create: `src-tauri/src/perception/fusion.rs` (stub `fuse` for P1: UIA-only passthrough)
- Modify: `src-tauri/src/perception/mod.rs`

```rust
pub trait Perceiver: Send + Sync {
    fn name(&self) -> &str;
    fn capture(&self, ctx: &PerceptionContext) -> Result<ScreenFrame, PerceptionError>;
}

/// HybridPerceiver owns FusionEngine and is the only Perceiver in production.
pub struct HybridPerceiver {
    uia: UiaPerceiver,
    vision: Option<VisionPerceiver>,
    fusion: FusionEngine,
}
```

- [ ] P1: `fusion.fuse(uia_elements, &[])`; set `quality = Full` or `DegradedUiaOnly` from primary-window element count
- [ ] Push `warnings` (e.g. `WindowCapTruncated` when visible &gt; K)
- [ ] `StubPerceiver` returns `Ok(fixture)` for tests
- [ ] Commit: `feat(perception): fallible Perceiver + HybridPerceiver owns fusion`

---

### Task 6: Orchestrator migration (UiSnapshot → ScreenFrame)

**Files:**
- Modify: `src-tauri/src/orchestration/orchestrator.rs`
- Modify: `src-tauri/src/orchestration/decision.rs`
- Modify: `src-tauri/src/orchestration/detector.rs`
- Modify: `src-tauri/src/orchestration/action_feedback.rs` (signatures if needed)

- [ ] Replace `self.scanner.snapshot_with_context(&scan_ctx)` with:

```rust
let ctx = PerceptionContext::from_scan_ctx(&scan_ctx, input_monitor.poll().cursor);
let frame = tokio::task::spawn_blocking({
    let perceiver = self.perceiver.clone(); // Arc<dyn Perceiver> or dedicated handle
    move || perceiver.capture(&ctx)
})
.await
.map_err(|_| PerceptionError::ThreadJoin)??;

if !frame.warnings.is_empty() {
    tracing::warn!(target: "roota.perception", ?frame.warnings);
}
```

- [ ] On `Err(PerceptionError)`: emit `OrchestratorEvent::Error` and **return** — do not anchor with empty/stale frame
- [ ] Add `perceiver: Arc<dyn Perceiver>` to `Orchestrator` (construct in `lib.rs`)
- [ ] `DecisionEngine::next_step(..., frame: &ScreenFrame, ...)`
- [ ] **Forbidden:** `impl From<&ScreenFrame> for UiSnapshot` in production code. Migrate each test/fixture to `ScreenFrame` or `StubPerceiver` directly.
- [ ] Run full `cargo test` — all green without legacy adapter
- [ ] Commit: `refactor(orchestrator): consume ScreenFrame from Perceiver`

---

### Task 7: Settings + i18n for degraded perception

**Files:**
- Modify: `src-tauri/src/settings.rs`
- Modify: `src-tauri/src/i18n.rs`

- [ ] Add fields: `perception_mode`, `max_windows`, `vision_enabled`, `ocr_language`, `min_uia_elements`, `prompt_max_elements` (40), `prompt_max_windows` (3)
- [ ] Keys: `guidance.perception_limited`, `guidance.secure_desktop_blocked`
- [ ] When `frame.quality == DegradedUiaOnly`, orchestrator prepends limited-mode sentence once
- [ ] Tests in `settings::tests`
- [ ] Commit

---

## Phase P2 — Desktop, modals, multi-monitor hardening

### Task 8: Modal / owned-window attachment

**Files:**
- Modify: `src-tauri/src/perception/window_enum.rs`
- Modify: `src-tauri/src/perception/window_score.rs`

- [ ] Use `GetWindow(GW_OWNER)` / UIA `Parent` to attach `#32770` dialogs to parent app score
- [ ] **Fallback:** on attach failure, rank dialog as standalone + `PerceptionWarning::ModalAttachFailed`
- [ ] Test: fixture with dialog HWND owned by Explorer → elements merged
- [ ] Test: fixture with orphan dialog → warning set, elements still present
- [ ] Commit

---

### Task 9: Desktop-root / taskbar UIA walk

**Files:**
- Create: `src-tauri/src/perception/desktop.rs`

- [ ] `UIAutomation::get_root_element()` → find `Shell_TrayWnd`, `Start` experience hosts (Windows 10/11 differ — try both class names from spec)
- [ ] Merge taskbar elements into frame when step `expected_window` is `desktop` or intent is `open_start_menu` (new template optional)
- [ ] Document unsupported secure desktop in spec + runtime message
- [ ] Commit

---

### Task 10: Multi-monitor coordinate audit

**Files:**
- Modify: `src-tauri/src/overlay.rs`
- Modify: `src-tauri/src/perception/uia.rs`

- [ ] Test: `overlay::logical_coords` with non-zero `MonitorOrigin` (extend existing tests)
- [ ] Ensure all `ScreenElement.bounds` use physical screen coords from `GetWindowRect` + UIA bounding rect conversion
- [ ] **DPI fusion test (P2, before P3 OCR):** fixture at 125% scale — OCR quad mapped to screen coords must match expected rect ±2px (use stub bitmap + known text box; blocks P3 regressions)
- [ ] Commit: `fix(perception): physical coords across monitors`

---

## Phase P3 — Vision / OCR hybrid fallback

### Task 11: Window bitmap capture

**Files:**
- Create: `src-tauri/src/perception/vision/capture.rs`
- Add dep after spike: `xcap = "0.3"` (pin exact crates.io version in commit — **never** `0.0`)

- [ ] `capture_window_bitmap(hwnd, scale: f32) -> Result<RgbaImage, CaptureError>`
- [ ] Respect `ROOTA_CAPTURE_SCALE`
- [ ] Test: save nothing; assert dimensions &gt; 0 using mock buffer in stub
- [ ] Commit

---

### Task 12: Windows OCR adapter

**Files:**
- Create: `src-tauri/src/perception/vision/ocr_windows.rs`
- Modify: `Cargo.toml` — WinRT `Media_Ocr` or use pure-Rust `ocrs` to avoid WinRT build pain (document choice in commit)

- [ ] `OcrEngine::recognize(&RgbaImage, lang) -> Vec<OcrLine { text, bounds }>`
- [ ] Map OCR quad to screen coords (add window origin)
- [ ] Test with frozen 200×100 PNG in `src-tauri/testdata/ocr_notepad.png` (commit small fixture)
- [ ] Commit

---

### Task 13: Fusion engine (full merge rules)

**Files:**
- Modify: `src-tauri/src/perception/fusion.rs` (expand P1 passthrough)

- [ ] Only `HybridPerceiver` calls `FusionEngine::fuse(uia, ocr) -> Vec<ScreenElement>` with IoU merge threshold 0.5
- [ ] OCR-only kept when no UIA overlap
- [ ] Set `ScreenFrame.quality = VisionAssisted` when OCR contributed
- [ ] Unit tests: overlap merge, disjoint keep both, dedupe duplicate text
- [ ] Commit

---

### Task 14: Vision path in HybridPerceiver

**Files:**
- Modify: `src-tauri/src/perception/hybrid.rs`

- [ ] If `settings.vision_enabled && primary_window_interactable_count < min_uia_elements` → OCR **primary HWND client rect only** (not global element count)
- [ ] OCR runs inside `spawn_blocking` or `capture` already on blocking thread — do not block async runtime
- [ ] `ROOTA_PERCEPTION_MODE=uia` skips OCR entirely
- [ ] Log: `roota.perception quality=VisionAssisted ocr_lines=12`
- [ ] Commit

---

### Task 15: LLM prompt update

**Files:**
- Modify: `src-tauri/prompts/instruction_step.txt`
- Modify: `src-tauri/src/prompts.rs`
- Modify: `src-tauri/src/orchestration/orchestrator.rs` (`instruction_for_step`)

- [ ] Add placeholders: `{window_list}`, `{perception_quality}`, keep `{cursor_line}`
- [ ] `window_list` = `frame.window_list_for_prompt(settings.prompt_max_windows)` (default 3)
- [ ] `visible_elements` = `frame.visible_summary(settings.prompt_max_elements)` (default 40)
- [ ] Commit

---

## Phase P4 — Performance, caching, reliability

### Task 16: Frame cache (guide poll loop)

**Files:**
- Create: `src-tauri/src/perception/cache.rs`

- [ ] `FrameCache::get_or_capture(ttl_ms: 500, invalidate: InvalidateReason)` — return cached frame only when **all** hold:
  - age &lt; ttl
  - cursor moved ≤ 50px since cached frame
  - no click/double-click since cache (`InputSample` flags)
  - foreground HWND unchanged (compare `primary_window_id` or title)
  - no `StepCompleted` since cache
- [ ] **Always bypass cache** when `invalidate` is `UserAction` (click), `ForegroundChanged`, `StepBoundary`, or `PerceptionError` retry
- [ ] Never cache `Err` results
- [ ] Benchmark log under `ROOTA_LOG=debug`
- [ ] Commit

---

### Task 17: Offscreen / junk element filter

**Files:**
- Modify: `src-tauri/src/perception/uia.rs`

- [ ] Skip zero-width/height, `IsOffscreen` when property available
- [ ] Cap total elements 800 across all windows
- [ ] Test: fixture with offscreen nodes excluded
- [ ] Commit

---

### Task 18: End-to-end manual + automated checklist

- [ ] Extend `orchestration/detector` tests for multi-window title change
- [ ] Run `cargo test` — all green
- [ ] Run `npm run build`
- [ ] Manual script from design spec (Explorer + Chrome, mis-click, hybrid Notepad)
- [ ] Commit: `test(perception): e2e fixtures and detector multi-window cases`

---

## Phase P5 — Optional local VLM (Could Have)

### Task 19: Ollama vision icon locator (feature-gated)

**Files:**
- Create: `src-tauri/src/perception/vision/vlm.rs`
- Modify: `src-tauri/src/llm/ollama.rs`

- [ ] Only when `ROOTA_VISION_LLM=1` and `llava` (or configured model) available
- [ ] Use `tokio::time::timeout(8s, llm_vision_request(...))` on async runtime; **do not** call from inside `Perceiver::capture` without nested runtime — split `VisionPerceiver::recognize_icons_async` called from orchestrator before/after `spawn_blocking` UIA pass
- [ ] Send downscaled PNG + prompt: return JSON `{ "label": "...", "x": N, "y": N }` in **screen coords**
- [ ] Merge as low-confidence `ScreenElement` with `source: Ocr` and `confidence: 0.6`
- [ ] Strict timeout 8s; fallback to template text
- [ ] Commit behind feature flag

---

## Dependency checklist (`Cargo.toml`)

```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.61", features = [
  "Win32_Foundation",
  "Win32_UI_WindowsAndMessaging",
  "Win32_UI_Input_KeyboardAndMouse",
  "Win32_Graphics_Gdi",           # if BitBlt fallback
] }
xcap = "0.3"                      # Task 11 — pin after `cargo search xcap`; never 0.0
# Option A: windows Media OCR (WinRT)
# Option B: ocrs = "0.8"          # pure Rust, larger binary
```

Evaluate `ocrs` vs WinRT in Task 12 spike (30 min) — document decision in spec before merging Task 12.

---

## Migration / backwards compatibility

| Consumer | Migration |
|----------|-----------|
| `Scanner` trait | Deprecate; remove orchestrator usage in Task 6. Tests port to `StubPerceiver` → `ScreenFrame` |
| `UiSnapshot` | Keep struct for serde/logging only until frontend needs it; **no** `From<ScreenFrame>` in hot path |
| Stub scanner tests | Port fixtures to `StubPerceiver` |
| Frontend | No change until optional quality badge |
| PRD “automates tasks” wording | Product copy only; code stays guide-only |

---

## Verification commands (every phase)

```powershell
Set-Location "...\Roota\src-tauri"
cargo test
cargo clippy -- -D warnings
Set-Location ".."
npm run build
npm run tauri dev
```

Expected: zero test failures; logs show `roota.perception` with `windows≥1`, `elements≥1` for Explorer when user opened it.

---

## Self-review (plan vs spec)

| Spec requirement | Task |
|------------------|------|
| Multi-app targeting | P1 Tasks 2–6 |
| Score-before-cap ordering | P1 Task 3 test `cap_applied_after_sort_not_before` |
| `captured_at_ms` + `warnings` | P1 Task 1, Task 5 |
| Fallible `capture` + no stale frame on error | P1 Task 5–6 |
| FusionEngine owned by HybridPerceiver | P1 Task 5, P3 Task 13 |
| Multi-monitor coords + DPI | P2 Task 10 |
| Desktop/taskbar | P2 Task 9 |
| Modal fallback + warning | P2 Task 8 |
| OCR hybrid (primary rect only) | P3 Tasks 11–14 |
| Prompt caps 40 / 3 windows | P1 Task 1, P3 Task 15, settings Task 7 |
| Guide-only safety | All tasks — no `SendInput`; review in Task 6 PR |
| Performance ≤800ms | P4 Task 16 (strict invalidation) |
| Tests without live desktop | P1 stub + P2 DPI fixture + P3 OCR fixture + P4 Task 18 |

No placeholders remain in task definitions above.

---

## Execution handoff

**Plan complete and saved to `docs/superpowers/plans/2026-05-18-roota-universal-perception.md`.**  
**Design spec:** `docs/superpowers/specs/2026-05-18-roota-universal-perception-design.md`.

**Two execution options:**

1. **Subagent-Driven (recommended)** — one subagent per task, review between tasks (superpowers:subagent-driven-development).
2. **Inline Execution** — implement phase-by-phase in this session with checkpoints (superpowers:executing-plans).

**Suggested order:** P1 Tasks 1–7 first (delivers most user value without OCR complexity), then P2, then P3 if poor-a11y apps matter for the hackathon demo.

---

## Review changelog (2026-05-18b)

| Issue | Resolution |
|-------|------------|
| `Instant` on `ScreenFrame` | → `captured_at_ms: u64` |
| Missing `warnings` | → `Vec<PerceptionWarning>` + orchestrator log |
| Infallible `capture` | → `Result<ScreenFrame, PerceptionError>` |
| Cap before score | → score all, then take top K; explicit test |
| Fusion ownership unclear | → `HybridPerceiver` owns `FusionEngine` |
| `MIN_UIA_ELEMENTS` scope | → primary window client rect only |
| Prompt size ambiguous | → settings `prompt_max_*` + helper methods |
| `xcap = "0.0"` | → pin `0.3` after spike |
| Sync trait vs OCR/VLM | → `spawn_blocking` P1; async vision split P5 |
| `From<ScreenFrame> for UiSnapshot` | → forbidden in production |
| Cache too naive | → invalidate on click, foreground, step, errors |
| DPI + fusion untested | → P2 fixture test before P3 OCR |
