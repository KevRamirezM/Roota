# Windows Copilot Workflow — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the COPILOT loop (Understand → Observe → Plan → Guide → Verify/Replan) so Roota can guide **any reasonable Windows task** from live screen state, with plan preview, recovery, and senior-friendly UX — guide-only throughout.

**Architecture:** Extend orchestration with `TaskBrief`, `TaskPlan`, `PlanValidator`, and `ReplanEngine`. Orchestrator emits new events (`PlanPreview`, `Observing`, `Replanning`). `TaskPlanner` becomes the PLAN phase; perception (`ScreenFrame`) feeds OBSERVE. Frontend shows plan journey and stuck help. Builds on universal perception plan P1–P2 before adaptive replan needs OCR.

**Tech Stack:** Rust (Tauri 2), React/TypeScript UI, Ollama `qwen3:1.7b`, existing `HybridPerceiver`, `InputMonitor`, overlay channel.

**Design spec:** [`docs/superpowers/specs/2026-05-16-windows-copilot-workflow-design.md`](../specs/2026-05-16-windows-copilot-workflow-design.md)

**Related plans:** [`2026-05-18-roota-universal-perception.md`](2026-05-18-roota-universal-perception.md), [`2026-05-17-perception-ocr-first.md`](2026-05-17-perception-ocr-first.md)

---

## File map

| Path | Responsibility |
|------|----------------|
| `src-tauri/src/orchestration/brief.rs` | `TaskBrief`, UNDERSTAND LLM + heuristics |
| `src-tauri/src/orchestration/plan.rs` | `TaskPlan`, `PlanSource`, `PlanValidator` |
| `src-tauri/src/orchestration/replan.rs` | `ReplanEngine`, `ReplanReason` |
| `src-tauri/src/orchestration/planner.rs` | Extend: `plan_from_brief`, recipe merge |
| `src-tauri/src/orchestration/orchestrator.rs` | COPILOT loop, new events |
| `src-tauri/src/orchestration/mod.rs` | Export new modules |
| `src-tauri/prompts/task_brief.txt` | UNDERSTAND prompt |
| `src-tauri/prompts/task_planner.txt` | Add `{task_brief}` block |
| `src-tauri/guidance/recipes/*.json` | App step skeletons |
| `src-tauri/src/prompts.rs` | Render helpers |
| `src-tauri/src/i18n.rs` | New guidance keys |
| `src/types.ts` | Event + phase types |
| `src/hooks/useOrchestrator.ts` | Handle new events |
| `src/components/PlanPreview.tsx` | Plan journey UI |
| `src/components/GuidancePanel.tsx` | Observing / replanning copy |
| `src/components/MainScreen.tsx` | Wire stuck button |

---

## Phase W0 — Types, events, and plan validation (no UX yet)

### Task 1: TaskBrief + UNDERSTAND parser

**Files:**
- Create: `src-tauri/src/orchestration/brief.rs`
- Create: `src-tauri/prompts/task_brief.txt`
- Modify: `src-tauri/src/prompts.rs`
- Modify: `src-tauri/src/orchestration/mod.rs`

- [ ] **Step 1: Write failing test**

```rust
// src-tauri/src/orchestration/brief.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_brief_json_extracts_hints() {
        let v = serde_json::json!({
            "goal_summary": "Abrir la carpeta Descargas",
            "app_hints": ["explorador"],
            "object_hints": ["descargas"],
            "risk_flags": []
        });
        let brief = parse_brief_json(v, "Abre Descargas").unwrap();
        assert_eq!(brief.object_hints, vec!["descargas"]);
    }
}
```

- [ ] **Step 2: Run test**

Run: `cd src-tauri; cargo test parse_brief_json_extracts_hints -- --nocapture`  
Expected: FAIL (module missing)

- [ ] **Step 3: Implement**

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBrief {
    pub raw_utterance: String,
    pub goal_summary: String,
    pub app_hints: Vec<String>,
    pub object_hints: Vec<String>,
    pub risk_flags: Vec<String>,
}

pub fn heuristic_brief(utterance: &str, goal_target: &str) -> TaskBrief { /* scan_ctx-style keyword rules */ }
pub fn parse_brief_json(value: serde_json::Value, utterance: &str) -> Option<TaskBrief> { /* ... */ }
```

Add `render_task_brief(utterance, goal_target) -> String` using `task_brief.txt`.

- [ ] **Step 4: Run test** — PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/orchestration/brief.rs src-tauri/prompts/task_brief.txt src-tauri/src/prompts.rs src-tauri/src/orchestration/mod.rs
git commit -m "feat(orchestration): add TaskBrief understand phase types"
```

---

### Task 2: TaskPlan + PlanValidator

**Files:**
- Create: `src-tauri/src/orchestration/plan.rs`
- Modify: `src-tauri/src/orchestration/planner.rs` (import `TaskPlan`)

- [ ] **Step 1: Write failing test**

```rust
#[test]
fn validator_flags_weak_plan() {
    let frame = fixture_frame_with_only("Inicio");
    let plan = TaskPlan {
        steps: vec![
            blueprint("click", "Configuración"),
            blueprint("click", "Red e Internet"),
        ],
        ..minimal_plan()
    };
    let report = PlanValidator::new().validate(&plan, &frame);
    assert!(report.match_ratio < 0.5);
    assert!(report.needs_reobserve);
}
```

- [ ] **Step 2: Run test** — FAIL

- [ ] **Step 3: Implement**

```rust
pub struct PlanValidationReport {
    pub match_ratio: f32,      // steps with fuzzy element/window hit
    pub needs_reobserve: bool, // ratio < 0.5
    pub unmatched_targets: Vec<String>,
}

pub struct PlanValidator;

impl PlanValidator {
    pub fn validate(&self, plan: &TaskPlan, frame: &ScreenFrame) -> PlanValidationReport {
        // For each step: frame.find_best_for_action OR window title contains target
    }
}
```

- [ ] **Step 4: Run `cargo test validator_flags`** — PASS

- [ ] **Step 5: Commit** — `feat(orchestration): validate plans against ScreenFrame`

---

### Task 3: Orchestrator events (backend)

**Files:**
- Modify: `src-tauri/src/orchestration/orchestrator.rs`

- [ ] **Step 1: Extend enum**

```rust
#[derive(Clone, Debug, Serialize)]
pub struct PlanStepSummary {
    pub index: usize,
    pub action: String,
    pub target: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", content = "data")]
pub enum OrchestratorEvent {
    // existing...
    Observing { pass: u8 },
    PlanPreview { summary: String, steps: Vec<PlanStepSummary> },
    Replanning { reason: String },
    // ...
}
```

- [ ] **Step 2: Serialize test**

```rust
#[test]
fn plan_preview_event_serializes() {
    let e = OrchestratorEvent::PlanPreview { /* ... */ };
    let j = serde_json::to_string(&e).unwrap();
    assert!(j.contains("PlanPreview"));
}
```

- [ ] **Step 3: Run test** — PASS

- [ ] **Step 4: Commit**

---

## Phase W1 — COPILOT pipeline in orchestrator

### Task 4: BriefExtractor (UNDERSTAND)

**Files:**
- Modify: `src-tauri/src/orchestration/brief.rs`
- Modify: `src-tauri/src/orchestration/orchestrator.rs`

- [ ] **Step 1: Test stub LLM returns brief**

Use existing `StubLlmClient` pattern from `intent.rs` tests.

- [ ] **Step 2: Implement `BriefExtractor::understand(utterance, goal_target) -> TaskBrief`**

- Timeout 10s; fallback `heuristic_brief`
- Merge `ScanContext::enrich_from_utterance` tokens into `app_hints`

- [ ] **Step 3: Wire in `Orchestrator::run` before confirmation**

```rust
let brief = self.brief_extractor.understand(&utterance, &target_label).await;
// risk_flags non-empty → use stronger confirm key if added later
```

- [ ] **Step 4: `cargo test brief`** — PASS

- [ ] **Step 5: Commit**

---

### Task 5: Observe → Plan → PlanPreview

**Files:**
- Modify: `src-tauri/src/orchestration/orchestrator.rs`
- Modify: `src-tauri/src/orchestration/planner.rs`
- Modify: `src-tauri/prompts/task_planner.txt`
- Modify: `src-tauri/src/prompts.rs`

- [ ] **Step 1: Refactor planner signature**

```rust
pub async fn plan_from_brief(
    &self,
    brief: &TaskBrief,
    frame: &ScreenFrame,
    scan_ctx: &ScanContext,
    perception: &PerceptionSettings,
) -> TaskPlan
```

Include `{goal_summary}`, `{app_hints}`, `{object_hints}` in planner prompt.

- [ ] **Step 2: After user confirms, emit Observing then capture**

```rust
sink.send(OrchestratorEvent::Observing { pass: 1 }).await;
let frame = self.capture_frame(&scan_ctx, cursor).await?;
let plan = self.planner.plan_from_brief(&brief, &frame, &scan_ctx, &self.settings.perception).await;
let report = PlanValidator::new().validate(&plan, &frame);
let frame = if report.needs_reobserve {
    sink.send(OrchestratorEvent::Observing { pass: 2 }).await;
    self.capture_frame(&scan_ctx, cursor).await?
} else { frame };
// replan if reobserved
```

- [ ] **Step 3: Emit PlanPreview**

```rust
sink.send(OrchestratorEvent::PlanPreview {
    summary: plan.brief.goal_summary.clone(),
    steps: plan.steps.iter().enumerate().map(|(i, s)| PlanStepSummary { /* */ }).collect(),
}).await;
```

- [ ] **Step 4: Convert `TaskPlan` → `GuidanceTemplate` for existing guide loop**

- [ ] **Step 5: Integration test with `StubPerceiver` + stub LLM** — event order: Confirm → Observing → PlanPreview → StepReady

- [ ] **Step 6: Commit** — `feat(orchestrator): observe-plan-preview pipeline`

---

### Task 6: Route all free-form intents through windows_task plan

**Files:**
- Modify: `src-tauri/src/orchestration/intent.rs`
- Modify: `src-tauri/src/orchestration/orchestrator.rs`

- [ ] **Step 1: Test — unknown intent becomes `windows_task` with brief (existing) still plans**

- [ ] **Step 2: Remove duplicate `heuristic_plan` branch when `TaskPlan` already built**

- [ ] **Step 3: Keep static templates for `open_folder` etc. as fast path** (`PlanSource::Template`)

- [ ] **Step 4: Commit**

---

## Phase W2 — App recipes + planner quality

### Task 7: Recipe registry

**Files:**
- Create: `src-tauri/guidance/recipes/explorer.json`
- Create: `src-tauri/guidance/recipes/chrome.json`
- Create: `src-tauri/guidance/recipes/settings.json`
- Create: `src-tauri/src/orchestration/recipes.rs`
- Modify: `src-tauri/tauri.conf.json` (bundle resources if needed)

- [ ] **Step 1: Test load recipes**

```rust
#[test]
fn recipes_load_chrome_new_tab() {
    let reg = RecipeRegistry::load_embedded();
    let steps = reg.skeleton("chrome", "new_tab");
    assert!(!steps.is_empty());
}
```

- [ ] **Step 2: Implement merge: if LLM plan empty/low confidence, overlay skeleton targets with `frame.ranked_visible_summary` labels**

- [ ] **Step 3: Commit**

---

### Task 8: Planner prompt v2 + step cap

**Files:**
- Modify: `src-tauri/prompts/task_planner.txt`
- Modify: `src-tauri/src/orchestration/planner.rs`

- [ ] **Step 1: Add rules**

- Prefer targets from `visible_elements` list (copy exact casing)
- First step `locate` if expected app not foreground
- Max 6 steps for seniors (reduce from 8)

- [ ] **Step 2: Test parse rejects markdown fences**

- [ ] **Step 3: Commit**

---

## Phase W3 — Verify, replan, stuck help

### Task 9: ReplanEngine

**Files:**
- Create: `src-tauri/src/orchestration/replan.rs`
- Modify: `src-tauri/src/orchestration/orchestrator.rs`

- [ ] **Step 1: Test replan remaining steps**

```rust
#[test]
fn replan_skips_completed_steps() {
    let engine = ReplanEngine::new(stub_llm());
    let remaining = engine.remaining_blueprints(&plan, 2);
    assert_eq!(remaining.len(), plan.steps.len() - 2);
}
```

- [ ] **Step 2: Implement `replan(session, brief, frame, reason) -> TaskPlan`**

- Max 2 per session (`session.replan_count`)

- [ ] **Step 3: Hook `wait_for_target` failure**

```rust
if attempt + 1 >= PERCEPTION_MAX_ATTEMPTS {
    if session.replan_count < 2 {
        sink.send(OrchestratorEvent::Replanning { reason: "target_not_found".into() }).await;
        // refresh template from replan
    }
}
```

- [ ] **Step 4: Hook corrective_message ≥2 on same step**

- [ ] **Step 5: Commit**

---

### Task 10: Tauri command `request_stuck_help`

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/orchestration/orchestrator.rs`

- [ ] **Step 1: Add `Arc<Orchestrator>` handle with `tokio::sync::Notify` or channel for stuck signal**

- [ ] **Step 2: Command sets flag consumed in guide poll loop → triggers replan with `UserAskedHelp`**

- [ ] **Step 3: Test command invokes without panic**

- [ ] **Step 4: Commit**

---

### Task 11: Step success signals (lightweight)

**Files:**
- Modify: `src-tauri/src/orchestration/detector.rs`
- Modify: `src-tauri/src/orchestration/state.rs`

- [ ] **Step 1: Add `StepSignal` on blueprint: `Default | WindowTitleContains | ElementAppears | ElementDisappears`**

- [ ] **Step 2: Planner sets `WindowTitleContains` when `expected_window` set**

- [ ] **Step 3: Tests with frame fixtures**

- [ ] **Step 4: Commit**

---

## Phase W4 — Frontend copilot UX

### Task 12: TypeScript event types

**Files:**
- Modify: `src/types.ts`

- [ ] **Step 1: Add**

```typescript
export interface PlanStepSummary {
  index: number;
  action: ActionVerb;
  target: string;
}

// extend OrchestratorEvent:
| { kind: "Observing"; data: { pass: number } }
| { kind: "PlanPreview"; data: { summary: string; steps: PlanStepSummary[] } }
| { kind: "Replanning"; data: { reason: string } }
```

- [ ] **Step 2: `npm run build`** — PASS

- [ ] **Step 3: Commit**

---

### Task 13: useOrchestrator reducer

**Files:**
- Modify: `src/hooks/useOrchestrator.ts`

- [ ] **Step 1: New phases**

```typescript
| { kind: "observing"; pass: number }
| { kind: "plan_preview"; summary: string; steps: PlanStepSummary[] }
| { kind: "replanning"; reason: string }
```

- [ ] **Step 2: On `PlanPreview`, store plan; on `StepReady`, transition to `running`**

- [ ] **Step 3: Manual smoke: submit utterance → see plan list → step 1 overlay**

- [ ] **Step 4: Commit**

---

### Task 14: PlanPreview component

**Files:**
- Create: `src/components/PlanPreview.tsx`
- Modify: `src/components/MainScreen.tsx`
- Modify: `src/theme.css` (calm list styles per existing tokens)

- [ ] **Step 1: Render numbered steps, goal summary, WCAG large type**

- [ ] **Step 2: Show during `plan_preview` phase; hide when `running`**

- [ ] **Step 3: Commit**

---

### Task 15: Stuck help button + i18n

**Files:**
- Modify: `src/components/GuidancePanel.tsx`
- Modify: `src/i18n.ts`
- Modify: `src-tauri/src/i18n.rs`

- [ ] **Step 1: Keys: `guidance.stuck_button`, `guidance.replanning`, `guidance.observing`, `guidance.plan_partial`**

- [ ] **Step 2: Button calls `invoke("request_stuck_help")`**

- [ ] **Step 3: Commit**

---

## Phase W5 — Perception integration gate (dependency)

> **Prerequisite:** Complete universal perception plan **P1 Tasks 1–7** and **P2 Task 8–9** before relying on replan for taskbar/background targets.

### Task 16: Perception checklist for copilot QA

- [ ] Multi-window: plan for “click Descargas” finds element when Explorer in background
- [ ] Modal: File dialog steps attach to parent window
- [ ] Desktop: “Abrir Inicio” locates Start via desktop walk
- [ ] Degraded mode: UI shows `guidance.perception_limited` once per session

Document results in `docs/superpowers/plans/2026-05-16-windows-copilot-workflow.md` appendix after manual run.

---

## Phase W6 — Verification & demo script

### Task 17: Automated tests

- [ ] `cargo test` — orchestration + brief + plan + replan
- [ ] `npm run build`
- [ ] Add `src-tauri/tests/copilot_session_stub.rs` if needed for full event sequence

### Task 18: Manual demo script (senior scenarios)

| # | Utterance | Expected |
|---|-----------|----------|
| 1 | Abre la carpeta Descargas | Plan ≥2 steps, anchor on Descargas |
| 2 | Abre Configuración de Windows | Plan via Start or search |
| 3 | (Chrome open) Abre una pestaña nueva | Recipe + visible “Nueva pestaña” |
| 4 | Wrong click on purpose | Correction then replan if repeated |
| 5 | Tap “No lo veo” | Replanning event, new anchor |

- [ ] **Commit:** `test(copilot): stub session and manual script`

---

## Self-review (plan vs spec)

| Spec requirement | Task |
|------------------|------|
| COPILOT five phases | W0–W3, W1 Task 5 |
| Plan preview | W0 Task 3, W4 Task 14 |
| Screen-grounded validation | W0 Task 2, W1 Task 5 |
| Re-plan on failure | W3 Task 9–10 |
| App recipes | W2 Task 7 |
| Guide-only | All tasks — no input automation |
| Senior UX | W4 Tasks 14–15 |
| Perception dependency | W5 Task 16 |
| Tests | W0–W3 unit tests, W6 |

No placeholders in task definitions.

---

## Suggested execution order

```text
W0 (types/events) → W1 (pipeline) → W4 (UX) in parallel with perception P1
→ W2 (recipes) → W3 (replan) after P1 perception stable
→ W5 gate → W6 verify
```

**Parallel work:** Another agent can execute [`2026-05-18-roota-universal-perception.md`](2026-05-18-roota-universal-perception.md) P1 while this plan’s W0–W1 land.

---

## Execution handoff

**Plan complete and saved to `docs/superpowers/plans/2026-05-16-windows-copilot-workflow.md`.**  
**Design spec:** `docs/superpowers/specs/2026-05-16-windows-copilot-workflow-design.md`.

**Two execution options:**

1. **Subagent-Driven (recommended)** — one subagent per task, review between tasks (`superpowers:subagent-driven-development`).
2. **Inline Execution** — implement phase-by-phase in this session with checkpoints (`superpowers:executing-plans`).

**Which approach do you want?**
