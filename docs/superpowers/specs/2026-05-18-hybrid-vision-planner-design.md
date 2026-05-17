# Hybrid Vision Planner — Design Spec

**Date:** 2026-05-18  
**Revision:** 2026-05-18c (review: availability cache, hint_xy only, scale rule, Moondream skip, UX event timing)  
**Status:** Draft — ready for implementation  
**Related:** [`2026-05-18-roota-universal-perception-design.md`](2026-05-18-roota-universal-perception-design.md), [`2026-05-17-llama-cpp-unified-pipeline-design.md`](2026-05-17-llama-cpp-unified-pipeline-design.md)  
**Goal:** Answer “can we send screenshots to the model?” with **yes**, without replacing the fast text path. Use **Option A** (perception) as primary and **Option B** (multimodal plan-from-image) only when text planning is weak.

---

## Problem

Today Roota:

1. **Perceives** via UIA + Windows OCR (+ optional Moondream for element boxes) → `ScreenFrame`.
2. **Plans** via text LLM (`TaskPlanner` + `task_planner.txt`) on element summaries — **no image** reaches the planner.
3. **Resolves anchors** at runtime from `ScreenFrame.find_best_for_action` (`decision.rs`).

Planner JSON includes `x`/`y`, but `parse_plan_json` **drops** them; coordinates only come from perception fusion. When UIA is empty and OCR is sparse, text plans invent labels or fail `PlanValidator` (`match_ratio < 0.5`), triggering a second capture but **the same text planner**.

Moondream already sends PNGs to Ollama for **element detection** (`vision_detect.txt`), not for **step planning**. That is Option A extended.

---

## Decision

| Approach | Role in Roota |
|----------|----------------|
| **A — Screenshot → VLM → elements → text planner** | **Primary perception** (existing `LayeredVisionPerceiver`, Moondream optional). Keep improving OCR/UIA fusion. |
| **B — Screenshot → multimodal LLM → steps** | **Planner fallback only** when `PlanValidator` reports low confidence after pass 2. |
| **Hybrid** | **Ship this.** Happy path stays fast (UIA/OCR + llama.cpp text). Hard screens pay one slow Ollama call (`qwen2.5vl:3b`). |

**Backend split (unchanged from llama.cpp spec):**

- **Text:** `llama-server` / `LlamaCppClient` — bootstrap + planner.
- **Vision:** **Ollama only** (`POST /api/chat` with `images[]`). llama.cpp vision on Windows remains experimental; do not block hackathon on it.

**Privacy:** Screenshots stay on-device (Ollama local). No upload APIs.

---

## Success criteria

1. With `ROOTA_VISION_PLANNER=1` and `qwen2.5vl:3b` (or configured model) in Ollama, a sparse-UIA scenario still produces ≥1 step with a resolvable anchor after fallback.
2. With vision planner **disabled**, behavior is identical to today (no extra Ollama calls on happy path).
3. Fallback runs **at most once** per `observe_and_plan` (after text plan + optional reobserve).
4. `PlanSource::Vision` appears in logs when fallback used.
5. **No `/api/tags` probe on the failure path** — availability is resolved once at startup.
6. Unit tests: JSON parse, image→screen coord mapping, validator trigger, orchestrator gate (no live Ollama).

---

## Non-goals

- Replacing Moondream element detect with qwen2.5-vl for perception (separate code path).
- User-uploaded images from disk (future UI; v1 is live capture only).
- Cloud VLMs.
- llama.cpp multimodal server on Windows in this iteration.
- Injecting synthetic `ScreenElement` rows for planner hints (see Anchor resolution).

---

## Architecture

```text
observe_and_plan
  │
  ├─ capture → ScreenFrame (UIA + OCR [+ Moondream elements])
  │
  ├─ TaskPlanner::plan_from_brief (text, llama.cpp)     ← happy path
  │
  ├─ PlanValidator::validate
  │     match_ratio < 0.5 after pass 2?
  │         yes ─┬─ emit Observing { pass: 3 }  (UI feedback — BEFORE Ollama)
  │              ├─ capture primary window PNG
  │              └─ VisionTaskPlanner::plan_from_image (Ollama, NEW)
  │                    → TaskPlan with StepBlueprint.hint_xy
  │
  └─ decision / overlay (hint_xy when find_best misses)
```

### New types

```rust
// orchestration/vision_planner.rs
pub struct VisionTaskPlanner {
    client: OllamaClient,
    /// Set once in VisionTaskPlanner::new — never re-query /api/tags per plan.
    available: bool,
    timeout_secs: f32,
    max_edge: u32,
}

pub enum PlanSource {
    // existing...
    Vision,
}
```

### Model availability (startup cache)

`VisionTaskPlanner::is_available()` must **not** call `ollama list` or `GET /api/tags` inside `observe_and_plan`.

**Rule:** Probe once when the planner is constructed (Tauri `.setup()` or `Orchestrator::new`, same phase as Moondream warmup). Store result in `VisionTaskPlanner { available: bool }`. Hot path reads the bool only.

If the user pulls the model after startup, fallback stays off until app restart (acceptable for hackathon).

### Trigger conditions (all required)

- `settings.perception.vision_planner_enabled == true`
- `vision_planner.available == true` (cached at startup)
- `PlanValidationReport::match_ratio < settings.perception.vision_planner_min_match` (default `0.5`)
- Not already attempted vision plan this cycle (`observe_and_plan` flag)
- **Moondream skip:** `frame` did **not** already use Moondream/VLM in perception this cycle (`PerceptionQuality::VisionAssisted` or `vision_contributed` on the frame used for validation) — avoids a second Ollama multimodal call when element-detect VLM already ran

### Anchor resolution (hard decision)

**Use `StepBlueprint.hint_xy: Option<(i32, i32)>` only.** Do not push synthetic `ScreenElement` into `ScreenFrame`.

Flow:

1. Vision (or text) planner JSON → `hint_xy` in image space.
2. `VisionTaskPlanner` maps image coords → **physical screen** centers via `map_image_rect_to_screen` (same as Moondream).
3. `decision.rs`: `find_best_for_action` first; if `None` and `hint_xy` is `Some`, set `GuideStep.anchor_xy` from hint.

---

## Screenshot scaling (`ROOTA_VISION_PLANNER_MAX_EDGE`)

Reuse `capture_window_bitmap` + `CaptureOptions` (same as Moondream/OCR):

| Rule | Value |
|------|--------|
| Scale basis | **Long edge** of the captured window bitmap |
| Target | `long_edge ≤ max_edge × capture_scale` |
| Aspect ratio | **Preserved** (uniform scale) |
| Crop | **None** — full window content remains in the image |
| Letterbox | **None** |

**Example:** 1920×1080 window, `max_edge = 768`, `scale = 1.0` → output **768×432**. Prompt `{width}×{height}` must use **bitmap** dimensions after scale. `map_image_rect_to_screen` must use the same `CapturedFrame` width/height and `source_rect` as Moondream.

---

## UX: event while waiting (60–90s)

**Emit before the blocking Ollama call**, not after it returns.

```rust
// observe_and_plan — order is mandatory
sink.send(OrchestratorEvent::Observing { pass: 3 }).await;
// Frontend maps pass == 3 → i18n "guidance.observing_vision"
let plan = tokio::task::spawn_blocking(|| vision_planner.plan_from_brief_blocking(...)).await;
```

| Item | Spec |
|------|------|
| Event | `OrchestratorEvent::Observing { pass: 3 }` |
| Meaning | Vision **planner** running (not a third `ScreenFrame` UIA capture) |
| When | Immediately after `should_run_vision_planner` is true, **before** `spawn_blocking` |
| UI copy | `guidance.observing_vision` (ES/EN in `i18n.rs`) |
| Duration | Shown for entire Ollama call; cleared when `PlanPreview` or `Error` follows |

Pass 1 / 2 remain UIA+OCR capture passes. Pass 3 is vision-planner-only so the frontend can show a distinct slow-path message.

---

## Configuration

| Env | Default | Meaning |
|-----|---------|---------|
| `ROOTA_VISION_PLANNER` | `0` | Enable multimodal planner fallback |
| `ROOTA_VISION_PLANNER_MODEL` | `qwen2.5vl:3b` | Ollama tag (probed at startup) |
| `ROOTA_VISION_PLANNER_TIMEOUT_SECS` | `90` | Single slow call budget |
| `ROOTA_VISION_PLANNER_MIN_MATCH` | `0.5` | Text plan quality threshold |
| `ROOTA_VISION_PLANNER_MAX_EDGE` | `768` | Long-edge downscale cap (aspect preserved) |

Moondream remains `ROOTA_VISION_MODEL` for **perception** only.

---

## Risks

| Risk | Mitigation |
|------|------------|
| 60–90s fallback on CPU | Only on failure; `Observing { pass: 3 }` **before** Ollama call |
| Model not installed | Startup cache `available = false`; log once; no per-cycle `/api/tags` |
| Hallucinated coordinates | `hint_xy` only when `find_best` misses; vision-assisted i18n on overlay |
| Duplicate Ollama calls (Moondream + qwen2.5vl) | Skip vision planner when perception already used VLM this cycle (`VisionAssisted` / `vision_contributed`) |
| Wrong coords after resize | Prompt uses post-scale bitmap size; map with same `CapturedFrame` |

---

## Testing strategy

- Pure Rust: parse fixtures, coord map (768×432 from 1920×1080 fixture), validator gating, `should_run_vision_planner` with `moondream_ran` flag.
- Manual: Chrome not focused + “open Chrome”; weak UIA app; confirm pass-3 message appears **before** long wait ends.
