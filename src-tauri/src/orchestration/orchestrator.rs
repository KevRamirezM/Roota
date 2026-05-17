//! Drives the classify → confirm → perceive → point → verify pipeline (PRD §8).
//!
//! Universal perception version: the scanner has been replaced by a fallible
//! `Perceiver` that returns a `ScreenFrame` (multi-window, screen-space). The
//! decision, detector, action-feedback, and prompt layers all consume
//! `ScreenFrame` directly — no `UiSnapshot` adapter is allowed in production.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tokio::sync::oneshot;
use tokio::sync::Mutex;

use crate::accessibility::scanner::ScanContext;
use crate::i18n;
use crate::input::InputMonitor;
use crate::llm::LlmClient;
use crate::orchestration::action_feedback;
use crate::orchestration::decision::DecisionEngine;
use crate::orchestration::detector::StateDetector;
use crate::orchestration::guidance_copy::{
    self, accept_llm_instruction, canonical_instruction, click_hint, goal_summary,
    spatial_hint, target_element, visible_elements_for_prompt,
};
use crate::orchestration::bootstrap::TaskBootstrapper;
use crate::orchestration::brief::TaskBrief;
use crate::orchestration::plan::{PlanSource, PlanValidator, TaskPlan};
use crate::orchestration::planner::TaskPlanner;
use crate::orchestration::replan::{ReplanEngine, ReplanReason};
use crate::orchestration::state::{ActionVerb, GuideStep, Intent, SessionState};
use crate::orchestration::templates::GuidanceTemplate;
use crate::orchestration::templates::TemplateRegistry;
use crate::perception::{
    cache::{FrameCache, InvalidateReason},
    frame::now_ms,
    PerceptionContext, PerceptionError, Perceiver, ScreenFrame,
};
use crate::prompts::{self, InstructionPromptContext};
use crate::settings::{Lang, Settings};

const PERCEPTION_POLL_MS: u64 = 500;
const PERCEPTION_MAX_ATTEMPTS: u32 = 28;
/// Fast input sampling — catches click edges UIA would miss at 500ms.
const INPUT_POLL_MS: u64 = 80;
/// Full desktop capture only every N input polls unless the user clicked.
const PERCEPTION_EVERY_N_POLLS: u32 = 5;
const GUIDE_MAX_POLLS: u32 = 150;
const POST_CONFIRM_MS: u64 = 400;
const STEP_LLM_TIMEOUT_SECS: f32 = 8.0;
const FRAME_CACHE_TTL_MS: u64 = 450;
const MAX_CORRECTIONS_BEFORE_REPLAN: u32 = 2;

#[derive(Clone, Debug, Serialize)]
pub struct PlanStepSummary {
    pub index: usize,
    pub action: ActionVerb,
    pub target: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", content = "data")]
pub enum OrchestratorEvent {
    ConfirmationRequested { message: String },
    Observing { pass: u8 },
    PlanPreview {
        summary: String,
        steps: Vec<PlanStepSummary>,
    },
    Replanning { reason: String },
    StepReady { step: GuideStep },
    StepCompleted { index: usize },
    AnchorChanged { x: i32, y: i32, label: String },
    GoalCompleted { steps: usize },
    Error { message: String },
    Finished,
}

#[async_trait::async_trait]
pub trait EventSink: Send + Sync {
    async fn send(&self, event: OrchestratorEvent);
}

pub struct Orchestrator {
    llm: Arc<dyn LlmClient>,
    perceiver: Arc<dyn Perceiver>,
    templates: Arc<TemplateRegistry>,
    bootstrapper: TaskBootstrapper,
    planner: TaskPlanner,
    replan_engine: ReplanEngine,
    decision: DecisionEngine,
    detector: StateDetector,
    pending_confirmation: Mutex<Option<oneshot::Sender<bool>>>,
    cancelled: AtomicBool,
    stuck_requested: AtomicBool,
    lang: Lang,
    settings: Settings,
}

impl Orchestrator {
    pub fn new(
        llm: Arc<dyn LlmClient>,
        perceiver: Arc<dyn Perceiver>,
        templates: Arc<TemplateRegistry>,
        settings: Settings,
    ) -> Self {
        let lang = settings.ui_language;
        let bootstrapper = TaskBootstrapper::new(
            llm.clone(),
            templates.clone(),
            lang,
            settings.llm_timeout_seconds,
        );
        let planner = TaskPlanner::new(
            llm.clone(),
            settings.llm_timeout_seconds,
            settings.planner_prompt_max_elements,
        );
        let replan_engine = ReplanEngine::new(
            llm.clone(),
            settings.llm_timeout_seconds,
            settings.planner_prompt_max_elements,
        );
        Self {
            llm,
            perceiver,
            templates,
            bootstrapper,
            planner,
            replan_engine,
            decision: DecisionEngine::new(lang),
            detector: StateDetector,
            pending_confirmation: Mutex::new(None),
            cancelled: AtomicBool::new(false),
            stuck_requested: AtomicBool::new(false),
            lang,
            settings,
        }
    }

    /// User tapped "No lo veo" — guide loop will replan on next poll.
    pub fn request_stuck_help(&self) {
        self.stuck_requested.store(true, Ordering::Relaxed);
    }

    pub async fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
        self.stuck_requested.store(false, Ordering::Relaxed);
        let mut guard = self.pending_confirmation.lock().await;
        if let Some(tx) = guard.take() {
            let _ = tx.send(false);
        }
    }

    pub async fn resolve_confirmation(&self, accepted: bool) {
        let mut guard = self.pending_confirmation.lock().await;
        if let Some(tx) = guard.take() {
            let _ = tx.send(accepted);
        }
    }

    pub async fn run<S: EventSink + ?Sized>(self: Arc<Self>, utterance: String, sink: Arc<S>) {
        self.cancelled.store(false, Ordering::Relaxed);
        self.stuck_requested.store(false, Ordering::Relaxed);
        let (intent, task_brief) = self.bootstrapper.bootstrap(&utterance).await;
        let template_key = self.resolve_template_key(&intent);
        if template_key == "unknown" {
            sink.send(OrchestratorEvent::Error {
                message: i18n::t("intent.unknown", self.lang, &[]),
            })
            .await;
            sink.send(OrchestratorEvent::Finished).await;
            return;
        }

        let template = self.templates.get(&template_key).unwrap().clone();
        let is_dynamic = template_key == "windows_task";
        tracing::info!(
            target: "roota.brief",
            summary = %task_brief.goal_summary,
            apps = ?task_brief.app_hints,
            "task brief ready"
        );

        let confirmation_message = i18n::t(
            &template.confirmation_action_key,
            self.lang,
            &[("target", &task_brief.goal_summary)],
        );

        let (tx, rx) = oneshot::channel();
        {
            let mut guard = self.pending_confirmation.lock().await;
            *guard = Some(tx);
        }
        sink.send(OrchestratorEvent::ConfirmationRequested {
            message: confirmation_message,
        })
        .await;

        let accepted = rx.await.unwrap_or(false);
        if !accepted {
            sink.send(OrchestratorEvent::Finished).await;
            return;
        }

        tokio::time::sleep(Duration::from_millis(POST_CONFIRM_MS)).await;

        let mut template = template;
        let mut scan_ctx =
            ScanContext::from_expected_window(template.expected_window.as_deref());
        task_brief.enrich_scan_context(&mut scan_ctx);

        if is_dynamic {
            match self
                .observe_and_plan(sink.as_ref(), &task_brief, &scan_ctx, &intent)
                .await
            {
                Ok((plan, partial)) => {
                    template = plan.to_guidance_template("windows_task", "confirm.windows_task");
                    self.emit_plan_preview(sink.as_ref(), &plan).await;
                    if partial {
                        tracing::info!(target: "roota.orchestrator", "plan partially grounded");
                    }
                    tracing::info!(
                        target: "roota.orchestrator",
                        steps = template.steps.len(),
                        "dynamic plan ready"
                    );
                }
                Err(msg) => {
                    sink.send(OrchestratorEvent::Error { message: msg }).await;
                    sink.send(OrchestratorEvent::Finished).await;
                    return;
                }
            }
            if template.steps.is_empty() {
                sink.send(OrchestratorEvent::Error {
                    message: i18n::t("intent.unknown", self.lang, &[]),
                })
                .await;
                sink.send(OrchestratorEvent::Finished).await;
                return;
            }
        }

        let mut session = SessionState::default();
        session.begin(intent.clone(), template.steps.len());
        let mut task_brief = task_brief;

        while !session.completed && !self.cancelled.load(Ordering::Relaxed) {
            let Some((frame_before, mut step)) = self
                .wait_for_target(
                    sink.as_ref(),
                    &intent,
                    &mut template,
                    &scan_ctx,
                    &mut session,
                    &mut task_brief,
                )
                .await
            else {
                sink.send(OrchestratorEvent::Error {
                    message: i18n::t("guidance.perception_failed", self.lang, &[]),
                })
                .await;
                sink.send(OrchestratorEvent::Finished).await;
                return;
            };

            let mut input_monitor = InputMonitor::new();
            let frame_cache = FrameCache::with_ttl(FRAME_CACHE_TTL_MS);
            frame_cache.put(frame_before.clone());

            let has_anchor = step.anchor_xy.is_some();
            let use_llm_phrasing = template.intent != "windows_task" || has_anchor;
            step.instruction = self
                .instruction_for_step(
                    &step,
                    &frame_before,
                    &intent,
                    &template,
                    &scan_ctx,
                    &input_monitor.poll(),
                    use_llm_phrasing,
                )
                .await;

            session.record(step.clone());
            self.emit_step(sink.as_ref(), &step).await;

            let mut completed = false;
            let mut last_anchor = step.anchor_xy;
            let mut rolling_frame = frame_before.clone();
            let mut polls_since_capture: u32 = 0;
            let mut target_click_count: u32 = 0;
            session.advance_step_cursor();

            for poll in 0..GUIDE_MAX_POLLS {
                if self.cancelled.load(Ordering::Relaxed) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(INPUT_POLL_MS)).await;

                if self.stuck_requested.swap(false, Ordering::Relaxed) {
                    if self
                        .try_replan(
                            sink.as_ref(),
                            &mut template,
                            &mut session,
                            &task_brief,
                            &scan_ctx,
                            &rolling_frame,
                            ReplanReason::UserAskedHelp,
                        )
                        .await
                    {
                        break;
                    }
                }

                let input = input_monitor.poll();

                if input.left_click && action_feedback::click_on_target(&step, &input) {
                    target_click_count = target_click_count.saturating_add(1);
                }

                // Fast path: correct click on target → advance without waiting for UIA.
                if let Some(user_done) =
                    action_feedback::user_action_completed(&step, &input, target_click_count)
                {
                    tracing::info!(
                        target: "roota.orchestrator",
                        reason = %user_done.reason,
                        "step completed via user action"
                    );
                    completed = true;
                    let finished_index = step.index;
                    session.advance();
                    if !session.completed {
                        sink.send(OrchestratorEvent::StepCompleted {
                            index: finished_index,
                        })
                        .await;
                    }
                    break;
                }

                let user_acted = input.left_click || input.right_click || input.double_click;
                polls_since_capture += 1;
                let need_capture = user_acted || polls_since_capture >= PERCEPTION_EVERY_N_POLLS;

                let frame_after = if need_capture {
                    polls_since_capture = 0;
                    let invalidate = if user_acted {
                        InvalidateReason::UserAction
                    } else {
                        InvalidateReason::Initial
                    };
                    match self
                        .capture_frame_cached(
                            &scan_ctx,
                            input.cursor,
                            &frame_cache,
                            invalidate,
                            true,
                        )
                        .await
                    {
                        Ok(f) => f,
                        Err(err) => {
                            tracing::warn!(
                                target: "roota.orchestrator",
                                "perception failed mid-guide: {err}"
                            );
                            continue;
                        }
                    }
                } else if let Some(cached) = frame_cache.get(
                    now_ms(),
                    input.cursor,
                    Some(rolling_frame.primary_window_id),
                    InvalidateReason::Initial,
                ) {
                    cached
                } else {
                    continue;
                };

                if let Some(correction) = action_feedback::corrective_message(
                    &step,
                    &input,
                    &rolling_frame,
                    &frame_after,
                    poll,
                    self.lang,
                ) {
                    if step.instruction != correction {
                        tracing::info!(
                            target: "roota.orchestrator",
                            cursor_x = input.cursor.x,
                            cursor_y = input.cursor.y,
                            "coaching correction"
                        );
                        step.instruction = correction;
                        self.emit_step(sink.as_ref(), &step).await;
                        session.corrections_this_step += 1;
                        let correction_count = session.corrections_this_step;
                        if correction_count >= MAX_CORRECTIONS_BEFORE_REPLAN && session.can_replan() {
                            let _ = self
                                .try_replan(
                                    sink.as_ref(),
                                    &mut template,
                                    &mut session,
                                    &task_brief,
                                    &scan_ctx,
                                    &frame_after,
                                    ReplanReason::WrongClick {
                                        count: correction_count,
                                    },
                                )
                                .await;
                            break;
                        }
                    }
                }

                if need_capture {
                    if let Ok(refreshed) =
                        self.decision
                            .next_step(&intent, &template, &frame_after, &session)
                    {
                        if refreshed.anchor_xy.is_some()
                            && (last_anchor != refreshed.anchor_xy
                                || refreshed.anchor_bounds != step.anchor_bounds)
                        {
                            let mut updated = refreshed;
                            updated.instruction = step.instruction.clone();
                            step = updated;
                            last_anchor = step.anchor_xy;
                            self.emit_step(sink.as_ref(), &step).await;
                        }
                    }

                    let outcome = self.detector.is_completed(
                        &step,
                        &frame_before,
                        &frame_after,
                        poll,
                    );
                    rolling_frame = frame_after;
                    if outcome.completed {
                        tracing::info!(
                            target: "roota.orchestrator",
                            reason = %outcome.reason,
                            "step completed"
                        );
                        completed = true;
                        let finished_index = step.index;
                        session.advance();
                        if !session.completed {
                            sink.send(OrchestratorEvent::StepCompleted {
                                index: finished_index,
                            })
                            .await;
                        }
                        break;
                    }
                }
            }

            if !completed {
                sink.send(OrchestratorEvent::Error {
                    message: i18n::t(
                        "guidance.element_not_found",
                        self.lang,
                        &[("target", &step.target_text)],
                    ),
                })
                .await;
                sink.send(OrchestratorEvent::Finished).await;
                return;
            }
        }

        let total = session.history.len();
        sink.send(OrchestratorEvent::GoalCompleted { steps: total })
            .await;
        sink.send(OrchestratorEvent::Finished).await;
    }

    /// OBSERVE → PLAN (with optional second capture).
    async fn observe_and_plan<S: EventSink + ?Sized>(
        &self,
        sink: &S,
        brief: &TaskBrief,
        scan_ctx: &ScanContext,
        _intent: &Intent,
    ) -> Result<(TaskPlan, bool), String> {
        sink.send(OrchestratorEvent::Observing { pass: 1 }).await;
        let mut input_monitor = InputMonitor::new();
        let cursor = input_monitor.poll().cursor;
        let frame = self
            .capture_frame(scan_ctx, cursor)
            .await
            .map_err(|_| i18n::t("guidance.perception_failed", self.lang, &[]))?;

        let mut plan = self
            .planner
            .plan_from_brief(brief, &frame, scan_ctx, &self.settings.perception)
            .await;
        let mut report = PlanValidator::new().validate(&plan, &frame);
        let mut partial = report.needs_reobserve;

        if report.needs_reobserve {
            sink.send(OrchestratorEvent::Observing { pass: 2 }).await;
            let cursor = input_monitor.poll().cursor;
            if let Ok(frame2) = self.capture_frame(scan_ctx, cursor).await {
                plan = self
                    .planner
                    .plan_from_brief(brief, &frame2, scan_ctx, &self.settings.perception)
                    .await;
                report = PlanValidator::new().validate(&plan, &frame2);
                partial = report.needs_reobserve;
            }
        }

        if plan.steps.is_empty() {
            plan = TaskPlan {
                steps: crate::orchestration::planner::heuristic_plan(
                    &brief.raw_utterance,
                    &brief.goal_summary,
                    Some(&frame),
                )
                .steps,
                brief: brief.clone(),
                expected_window: plan.expected_window,
                source: PlanSource::Heuristic,
            };
        }

        if plan.steps.is_empty() {
            return Err(i18n::t("intent.unknown", self.lang, &[]));
        }

        Ok((plan, partial))
    }

    async fn emit_plan_preview<S: EventSink + ?Sized>(&self, sink: &S, plan: &TaskPlan) {
        let steps: Vec<PlanStepSummary> = plan
            .steps
            .iter()
            .enumerate()
            .map(|(i, s)| PlanStepSummary {
                index: i + 1,
                action: s.action,
                target: s.target_query.clone(),
            })
            .collect();
        sink.send(OrchestratorEvent::PlanPreview {
            summary: plan.brief.goal_summary.clone(),
            steps,
        })
        .await;
    }

    async fn try_replan<S: EventSink + ?Sized>(
        &self,
        sink: &S,
        template: &mut GuidanceTemplate,
        session: &mut SessionState,
        brief: &TaskBrief,
        scan_ctx: &ScanContext,
        frame: &ScreenFrame,
        reason: ReplanReason,
    ) -> bool {
        if !session.can_replan() {
            return false;
        }
        sink.send(OrchestratorEvent::Replanning {
            reason: reason.label().into(),
        })
        .await;

        let new_plan = self
            .replan_engine
            .replan(
                brief,
                frame,
                scan_ctx,
                &self.settings.perception,
                &template.steps,
                session.step_index,
                &reason,
            )
            .await;

        if new_plan.steps.is_empty() {
            return false;
        }

        ReplanEngine::apply_replan(template, &new_plan, session.step_index);
        session.total_steps = template.steps.len();
        session.record_replan();
        self.emit_plan_preview(sink, &new_plan).await;
        true
    }

    /// Perception gate: scan until target has coordinates, emitting prep overlay meanwhile.
    async fn wait_for_target<S: EventSink + ?Sized>(
        &self,
        sink: &S,
        intent: &Intent,
        template: &mut GuidanceTemplate,
        scan_ctx: &ScanContext,
        session: &mut SessionState,
        brief: &mut TaskBrief,
    ) -> Option<(ScreenFrame, GuideStep)> {
        let mut input_monitor = InputMonitor::new();
        loop {
            for attempt in 0..PERCEPTION_MAX_ATTEMPTS {
                if self.cancelled.load(Ordering::Relaxed) {
                    return None;
                }

                let cursor = input_monitor.poll().cursor;
                let frame = match self.capture_frame(scan_ctx, cursor).await {
                    Ok(f) => f,
                    Err(err) => {
                        tracing::warn!(
                            target: "roota.orchestrator",
                            attempt,
                            "perception failed: {err}"
                        );
                        tokio::time::sleep(Duration::from_millis(PERCEPTION_POLL_MS)).await;
                        continue;
                    }
                };
                if let Ok(step) = self.decision.next_step(intent, template, &frame, session) {
                    if step.anchor_xy.is_some() {
                        tracing::info!(
                            target: "roota.orchestrator",
                            attempt,
                            primary = %frame.primary_window_title(),
                            windows = frame.windows.len(),
                            elements = frame.elements.len(),
                            "target located on screen"
                        );
                        return Some((frame, step));
                    }
                }

                if attempt == 0 || attempt % 3 == 0 {
                    let prep = self.prep_step(session, template, &frame, intent);
                    self.emit_step(sink, &prep).await;
                }

                tokio::time::sleep(Duration::from_millis(PERCEPTION_POLL_MS)).await;
            }

            if !session.can_replan() {
                return None;
            }
            let cursor = input_monitor.poll().cursor;
            let frame = match self.capture_frame(scan_ctx, cursor).await {
                Ok(f) => f,
                Err(_) => return None,
            };
            let target = template
                .steps
                .get(session.step_index)
                .map(|s| s.target_query.clone())
                .unwrap_or_default();
            let reason = ReplanReason::TargetNotFound {
                step_index: session.step_index,
                target,
            };
            if self
                .try_replan(sink, template, session, brief, scan_ctx, &frame, reason)
                .await
            {
                continue;
            }
            return None;
        }
    }

    /// Capture a `ScreenFrame` on the blocking pool (UIA + optional vision
    /// must not block the async runtime).
    async fn capture_frame(
        &self,
        scan_ctx: &ScanContext,
        cursor: crate::input::PhysicalPoint,
    ) -> Result<ScreenFrame, PerceptionError> {
        self.capture_frame_cached(
            scan_ctx,
            cursor,
            &FrameCache::new(),
            InvalidateReason::StepBoundary,
            false,
        )
        .await
    }

    /// Cached capture for the guide loop — skips expensive UIA when idle.
    async fn capture_frame_cached(
        &self,
        scan_ctx: &ScanContext,
        cursor: crate::input::PhysicalPoint,
        cache: &FrameCache,
        reason: InvalidateReason,
        guide_mode: bool,
    ) -> Result<ScreenFrame, PerceptionError> {
        let now = now_ms();
        if let Some(cached) = cache.get(now, cursor, None, reason) {
            return Ok(cached);
        }

        let mut ctx = PerceptionContext::from_scan_ctx(scan_ctx, cursor);
        let mut settings = self.settings.perception.clone();
        if guide_mode {
            settings.max_windows = settings.max_windows.min(4);
            match reason {
                InvalidateReason::UserAction | InvalidateReason::StepBoundary => {}
                _ => settings.vision_enabled = false,
            }
        } else if matches!(reason, InvalidateReason::StepBoundary) {
            settings.max_windows = settings.max_windows.max(4);
        }
        ctx.settings = settings;
        let perceiver = self.perceiver.clone();
        let frame = tokio::task::spawn_blocking(move || perceiver.capture(&ctx))
            .await
            .map_err(|_| PerceptionError::ThreadJoin)??;
        cache.put(frame.clone());
        Ok(frame)
    }

    fn prep_step(
        &self,
        session: &SessionState,
        template: &GuidanceTemplate,
        frame: &ScreenFrame,
        intent: &Intent,
    ) -> GuideStep {
        let primary_title = frame.primary_window_title();
        let element_count = frame.elements.len();
        let instruction = if primary_title.is_empty() || element_count == 0 {
            i18n::t(
                "guidance.prep_open_explorer",
                self.lang,
                &[("target", &intent.target)],
            )
        } else {
            i18n::t(
                "guidance.waiting_for_screen",
                self.lang,
                &[
                    ("window", &primary_title),
                    ("count", &element_count.to_string()),
                    ("target", &intent.target),
                ],
            )
        };
        GuideStep {
            index: session.step_index + 1,
            total: template.steps.len(),
            action: ActionVerb::Locate,
            target_text: intent.target.clone(),
            instruction,
            anchor_xy: None,
            anchor_bounds: None,
        }
    }

    async fn emit_step<S: EventSink + ?Sized>(&self, sink: &S, step: &GuideStep) {
        sink.send(OrchestratorEvent::StepReady { step: step.clone() })
            .await;
        if let Some((x, y)) = step.anchor_xy {
            sink.send(OrchestratorEvent::AnchorChanged {
                x,
                y,
                label: step.target_text.clone(),
            })
            .await;
        }
    }

    /// Instant copy when waiting; LLM only when we have a live anchor (PRD §8.3).
    #[allow(clippy::too_many_arguments)]
    async fn instruction_for_step(
        &self,
        step: &GuideStep,
        frame: &ScreenFrame,
        intent: &Intent,
        template: &GuidanceTemplate,
        scan_ctx: &ScanContext,
        input: &crate::input::InputSample,
        allow_llm: bool,
    ) -> String {
        let has_anchor = step.anchor_xy.is_some();
        let fallback = canonical_instruction(self.lang, step, has_anchor);

        if !self.settings.step_llm_enabled || !allow_llm || !has_anchor || frame.elements.is_empty() {
            return fallback;
        }

        let perception = &self.settings.perception;
        let mut hints = scan_ctx.window_hints.clone();
        if !intent.target.is_empty() {
            hints.push(intent.target.clone());
        }
        let visible = visible_elements_for_prompt(
            frame,
            perception.prompt_max_elements,
            &hints,
            input.cursor,
            &step.target_text,
        );
        let window_list = frame.window_list_for_prompt(perception.prompt_max_windows);
        let primary_title = frame.primary_window_title();
        let window_title = if primary_title.is_empty() {
            template
                .expected_window
                .clone()
                .unwrap_or_else(|| "la aplicación".into())
        } else {
            primary_title
        };

        let cursor_line = i18n::t(
            "guidance.cursor_position",
            self.lang,
            &[
                ("x", &input.cursor.x.to_string()),
                ("y", &input.cursor.y.to_string()),
            ],
        );

        let warnings_summary = frame.warnings_summary();
        let warnings_line = if warnings_summary.is_empty() {
            String::new()
        } else {
            format!("Aviso de lectura: {warnings_summary}.")
        };

        let element_source_note = if frame.has_vlm_elements() {
            i18n::t("guidance.perception_vlm_note", self.lang, &[])
        } else {
            String::new()
        };

        let hint = click_hint(self.lang, step.action);
        let cue = guidance_copy::overlay_cue(self.lang, step.action);
        let goal = goal_summary(self.lang, template, &step.target_text);
        let target_el = target_element(frame, &step.target_text);
        let spatial = spatial_hint(frame, target_el);

        let prompt = prompts::render_instruction_step(InstructionPromptContext {
            goal_summary: &goal,
            step_index: step.index,
            total_steps: step.total,
            click_hint: &hint,
            overlay_cue: &cue,
            target: &step.target_text,
            window_title: &window_title,
            window_list: &window_list,
            visible_elements: &visible,
            spatial_hint: &spatial,
            cursor_line: &cursor_line,
            target_on_screen: true,
            perception_quality: frame.quality.label(),
            warnings_line: &warnings_line,
            element_source_note: &element_source_note,
        });

        let llm_fut = self
            .llm
            .complete_text(&prompt, Some(prompts::SYSTEM_PROMPT));
        match tokio::time::timeout(
            Duration::from_secs_f32(STEP_LLM_TIMEOUT_SECS),
            llm_fut,
        )
        .await
        {
            Ok(Ok(text)) => {
                if let Some(accepted) =
                    accept_llm_instruction(&text, step, &hint)
                {
                    return accepted;
                }
                tracing::warn!(
                    target: "roota.orchestrator",
                    "LLM instruction rejected (contract mismatch)"
                );
            }
            Ok(Err(err)) => {
                tracing::warn!(target: "roota.orchestrator", "LLM instruction: {err}");
            }
            Err(_) => {
                tracing::warn!(target: "roota.orchestrator", "LLM instruction timed out");
            }
        }
        fallback
    }

    pub fn llm_name(&self) -> &str {
        self.llm.name()
    }

    pub fn perceiver_name(&self) -> &str {
        self.perceiver.name()
    }

    /// Map classifier output to a registry template (known intents or universal `windows_task`).
    fn resolve_template_key(&self, intent: &Intent) -> String {
        if intent.intent == "unknown" {
            return "unknown".to_string();
        }
        if self.templates.get(&intent.intent).is_some() {
            return intent.intent.clone();
        }
        if self.templates.get("windows_task").is_some() {
            return "windows_task".to_string();
        }
        "unknown".to_string()
    }
}

#[cfg(test)]
mod orchestrator_event_tests {
    use super::*;

    #[test]
    fn plan_preview_event_serializes() {
        let e = OrchestratorEvent::PlanPreview {
            summary: "Abrir Descargas".into(),
            steps: vec![PlanStepSummary {
                index: 1,
                action: ActionVerb::Click,
                target: "Descargas".into(),
            }],
        };
        let j = serde_json::to_string(&e).unwrap();
        assert!(j.contains("PlanPreview"));
        assert!(j.contains("Descargas"));
    }
}
