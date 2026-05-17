# Click Guidance Consistency — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or executing-plans.

**Goal:** Make “where to click” and “what to do” consistent across overlay pill, HUD, main panel, and LLM-generated instructions — vision/perception stays as-is.

**Architecture:** Introduce a single `guidance_copy` module that builds canonical Spanish instructions from step + frame + template. LLM prompts receive the same structured facts (human goal, click hint, marked target in element list, spatial hint). LLM output is validated; invalid text falls back to canonical copy. Perception-quality notes stay in the prompt only, not prepended to spoken instructions.

**Tech Stack:** Rust (Tauri), existing `instruction_step.txt`, i18n tables, overlay `click_hint`.

---

## Root causes (from codebase audit)

| Issue | Effect |
|-------|--------|
| LLM `goal` = `intent.intent` (`open_folder`) | Confusing, non-human task description |
| `action` in prompt ≠ overlay `click_hint` | “Clic con botón izquierdo” vs “Haz clic aquí” |
| Perception prefixes on fallback | Long, varying first sentences |
| No target marked in element list | LLM picks wrong control names |
| `anchor_status` says “círculo amarillo” for all actions | Wrong for double-click (orange) / right-click (blue) |
| Three channels (instruction, click_hint, overlay_hint) | Redundant or contradictory text |

---

## Phase 1 — Canonical instruction builder

**Files:**
- Create: `src-tauri/src/orchestration/guidance_copy.rs`
- Modify: `src-tauri/src/orchestration/mod.rs`

- [ ] Add `canonical_instruction(lang, step, has_anchor)` using i18n keys `guidance.instruction.*_with_anchor`
- [ ] Add `goal_summary(lang, template, target)` from `template.confirmation_action_key`
- [ ] Add `accept_llm_instruction(text, step, click_hint)` — must mention target, ≥8 chars, no multi-line

---

## Phase 2 — Prompt contract

**Files:**
- Modify: `src-tauri/prompts/instruction_step.txt`
- Modify: `src-tauri/src/prompts.rs`
- Modify: `src-tauri/src/perception/frame.rs` (`ranked_visible_summary_for_target`)
- Modify: `src-tauri/src/i18n.rs`

- [ ] New placeholders: `{goal_summary}`, `{click_hint}`, `{spatial_hint}`, `{overlay_cue}`
- [ ] Mark target row with `→ OBJETIVO` in element list
- [ ] Strict output rules: one sentence, use `click_hint` gesture, reference overlay cue, exact target name

---

## Phase 3 — Orchestrator wiring

**Files:**
- Modify: `src-tauri/src/orchestration/orchestrator.rs`

- [ ] `instruction_for_step` uses `guidance_copy` for fallback and prompt assembly
- [ ] Remove `perception_limited` / `vision_assisted` prefixes from user-facing strings
- [ ] Pass `goal_summary`, `click_hint`, spatial hint, marked element list to LLM

---

## Phase 4 — Verification

- [ ] `cargo test` in `src-tauri` (run once after all phases)
