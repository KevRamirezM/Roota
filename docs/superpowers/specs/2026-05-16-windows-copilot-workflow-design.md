# Roota — Windows Copilot Workflow (Design Spec)

**Date:** 2026-05-16  
**Status:** Draft — ready for implementation planning  
**Related:** PRD §5–8, [`2026-05-18-roota-universal-perception-design.md`](2026-05-18-roota-universal-perception-design.md)  
**Goal:** Turn Roota from “template + single-window scan” into a **patient Windows copilot** that understands any reasonable user request, **observes the whole desktop**, **plans concrete steps from what is visible**, **guides one gesture at a time**, and **recovers** when the screen changes — while staying **guide-only** (PRD §8.9).

---

## Problem

Today’s pipeline works for curated intents (`open_folder`, etc.) and a first-cut dynamic path (`windows_task` → one-shot `TaskPlanner` → fixed step list). Gaps that block “do anything on Windows”:

| Gap | User impact |
|-----|-------------|
| **Plan is frozen** after confirm | User opens wrong app or UI changes → steps no longer match screen |
| **No plan preview** | Senior sees confirmation text but not *what* will happen step-by-step |
| **Planner sees one snapshot** | Background windows / taskbar targets missed if not in first capture |
| **No structured “understand” phase** | LLM must infer app, goal, and steps in one JSON blob |
| **Weak recovery** | Timeout → generic error; no “let me look again” replan |
| **Template vs dynamic split** | Two mental models; `windows_task` bypasses rich recipes for common apps |
| **Verification is shallow** | Click-on-target fast path is good; UIA delta detection misses many app-specific success signals |

Perception work (multi-window `ScreenFrame`, OCR hybrid) is necessary but **not sufficient** — the **orchestration workflow** must close the loop.

---

## Success criteria

1. **Any desktop task (within guide-only bounds):** User says e.g. “Abre la configuración de Windows” or “Crea una carpeta en el escritorio” → Roota shows a **short plan preview** (2–6 steps) → user confirms → guidance proceeds with overlays.
2. **Screen-grounded targets:** ≥80% of planned step targets appear in the perception summary shown to the planner (measured in manual test script).
3. **Re-plan on failure:** If a step cannot be anchored after N polls or user clicks wrong control twice, Roota emits a **calm replan** (new steps from fresh `ScreenFrame`) without restarting the app.
4. **One action at a time:** UI and TTS continue to show exactly one instruction; plan preview is optional summary, not a wall of text.
5. **Guide-only:** No `SendInput`, shell execute for user tasks, or file mutation automation.
6. **Latency:** Plan preview ≤ 12 s on default Ollama model; per-step instruction ≤ 8 s (existing budget).
7. **Tests:** Stub `ScreenFrame` + stub LLM cover plan parse, replan trigger, and event serialization without a live desktop.

---

## Non-goals (this initiative)

- Full autonomous agent (RPA) or macro recording.
- Cloud LLM / screenshot upload.
- Perfect coverage of elevated UAC, games, or canvas-only UIs (degrade with clear copy).
- Multi-user profiles or cross-session learning.

---

## Recommended architecture: COPILOT loop

Five phases, one session. Perception and LLM roles are explicit.

```text
User utterance
      │
      ▼
┌─────────────┐     Intent + goal sketch (app, object, constraints)
│ UNDERSTAND  │     IntentRecognizer + TaskBrief extractor (LLM JSON)
└──────┬──────┘
       │
       ▼
┌─────────────┐     ScreenFrame (multi-window UIA + optional OCR)
│  OBSERVE    │     May run 2 captures: pre-plan + post-confirm refresh
└──────┬──────┘
       │
       ▼
┌─────────────┐     TaskPlan: steps[], goal_summary, expected_window
│    PLAN     │     TaskPlanner + PlanValidator + AppRecipe merge
└──────┬──────┘
       │
       ▼  PlanPreview event → UI (user already confirmed high-level)
┌─────────────┐     For each step: anchor → overlay → poll input + frame
│    GUIDE    │     Instruction LLM optional; canonical copy preferred
└──────┬──────┘
       │
       ▼
┌─────────────┐     Click feedback + StateDetector + step signals
│   VERIFY    │     On fail → REPLAN (partial) or coach correction
└─────────────┘
```

### New / extended types (`orchestration/`)

```rust
/// Structured goal before planning (from UNDERSTAND).
pub struct TaskBrief {
    pub raw_utterance: String,
    pub goal_summary: String,       // one sentence, user language
    pub app_hints: Vec<String>,     // "chrome", "explorador", "cursor"
    pub object_hints: Vec<String>,  // "descargas", "configuración"
    pub risk_flags: Vec<RiskFlag>,  // e.g. DeleteFiles, SendEmail
}

pub struct TaskPlan {
    pub brief: TaskBrief,
    pub expected_window: Option<String>,
    pub steps: Vec<StepBlueprint>,
    pub source: PlanSource,         // Template | Llm | Heuristic | Replan
}

pub enum ReplanReason {
    TargetNotFound { step_index: usize, target: String },
    WrongClick { count: u32 },
    ScreenChanged { detail: String },
    UserAskedHelp,
}
```

### Orchestrator event extensions

| Event | Purpose |
|-------|---------|
| `PlanPreview { summary, steps: Vec<PlanStepSummary> }` | UI shows numbered journey before first overlay |
| `Observing { pass: u8 }` | “Estoy mirando tu pantalla…” during capture |
| `Replanning { reason }` | Transparent recovery |
| `StuckHelp { message, suggestions }` | User tapped “No encuentro” |

Existing events (`StepReady`, `StepCompleted`, `AnchorChanged`, …) stay.

### Plan validation (`PlanValidator`)

Before guiding, ensure each step’s `target_query` is **plausible** against `ScreenFrame`:

- Fuzzy match against `frame.elements` and window titles.
- If &lt;50% steps match: one **automatic re-observe** (refresh frame, replan once).
- If still weak: plan proceeds but first step is `locate` with best window title; UI shows `guidance.plan_partial` once.

### App recipes (declarative, not code)

`src-tauri/guidance/recipes/*.json` — optional step **skeletons** merged when `TaskBrief.app_hints` matches:

```json
{
  "chrome": {
    "open_new_tab": [
      { "action": "click", "target_query": "Nueva pestaña" }
    ]
  }
}
```

Recipes **never** override screen-visible targets when planner found better labels in `ScreenFrame`.

### Re-plan policy

| Trigger | Action |
|---------|--------|
| `wait_for_target` exhausts attempts | `ReplanReason::TargetNotFound` → replan remaining steps only |
| `corrective_message` fired ≥2 times same step | replan current + remaining |
| User “No encuentro” | `UserAskedHelp` → widen perception (`max_windows+2` once) + replan |
| Max 2 replans per session | then calm error + offer restart |

### Safety

- `TaskBrief.risk_flags` → stronger confirmation copy for destructive intents (delete, format, send).
- `SafetyGuard` unchanged: anchors only.
- Replan cannot add steps that violate guide-only rules.

---

## Usability (senior-first)

| Surface | Change |
|---------|--------|
| **Plan preview card** | After confirm, before step 1: numbered list, goal sentence, “Empezar” implicit when first step emits |
| **Progress** | “Paso 2 de 5 · Abrir Configuración” in panel + overlay |
| **Observing state** | Replace silent `classifying` with explicit observing copy |
| **Stuck affordance** | Large “No lo veo” → replan + spoken shorter instruction |
| **Perception quality badge** | Optional chip when `DegradedUiaOnly` (from universal perception) |

---

## Dependency on perception initiative

| Copilot need | Perception deliverable |
|--------------|------------------------|
| Background window targets | Multi-window UIA + window scoring |
| Taskbar / Start | Desktop-root walk (P2) |
| Weak a11y apps | OCR hybrid (P3) |
| Fast guide loop | Frame cache + input poll (existing + P4) |

**Order:** Finish perception **P1–P2** before copilot **W2–W3**; copilot **W0–W1** can start in parallel (types, events, UI).

---

## Approaches considered

| Approach | Pros | Cons | Verdict |
|----------|------|------|---------|
| **A. COPILOT loop (recommended)** | Clear phases, testable, matches PRD companion metaphor | More events/types | **Ship** |
| **B. Bigger planner prompt only** | Small diff | No recovery, still one-shot | Fallback only |
| **C. Full agent with tool calls** | Flexible | Slow, hard to keep guide-only, poor on 1.7B | Defer |

---

## Open questions (defaults chosen)

1. **Plan preview before or after confirm?** — After intent confirm, before step 1 (user already said yes to goal; preview sets expectations).  
2. **Voice readout of plan?** — Optional TTS of `goal_summary` only (Could Have).  
3. **English UI?** — i18n keys for all new strings; planner prompts stay Spanish-first per product.

---

## Spec self-review

- [x] No TBD sections  
- [x] Guide-only preserved  
- [x] Scoped to workflow + UX; perception remains separate plan  
- [x] Testable success criteria  
