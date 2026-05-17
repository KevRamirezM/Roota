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
use crate::orchestration::intent::IntentRecognizer;
use crate::orchestration::state::{ActionVerb, GuideStep, Intent, SessionState};
use crate::orchestration::templates::GuidanceTemplate;
use crate::orchestration::templates::TemplateRegistry;
use crate::perception::{
    cache::{FrameCache, InvalidateReason},
    frame::now_ms,
    PerceptionContext, PerceptionError, PerceptionQuality, Perceiver, ScreenFrame,
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
const STEP_LLM_TIMEOUT_SECS: f32 = 4.0;
const FRAME_CACHE_TTL_MS: u64 = 450;

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", content = "data")]
pub enum OrchestratorEvent {
    ConfirmationRequested { message: String },
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
    recognizer: IntentRecognizer,
    decision: DecisionEngine,
    detector: StateDetector,
    pending_confirmation: Mutex<Option<oneshot::Sender<bool>>>,
    cancelled: AtomicBool,
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
        let recognizer = IntentRecognizer::new(
            llm.clone(),
            templates.clone(),
            lang,
            settings.llm_intent_timeout_seconds,
        );
        Self {
            llm,
            perceiver,
            templates,
            recognizer,
            decision: DecisionEngine::new(lang),
            detector: StateDetector,
            pending_confirmation: Mutex::new(None),
            cancelled: AtomicBool::new(false),
            lang,
            settings,
        }
    }

    pub async fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
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
        let intent = self.recognizer.recognise(&utterance).await;
        let template = match self.templates.get(&intent.intent) {
            Some(t) if intent.is_known() => t.clone(),
            _ => {
                sink.send(OrchestratorEvent::Error {
                    message: i18n::t("intent.unknown", self.lang, &[]),
                })
                .await;
                sink.send(OrchestratorEvent::Finished).await;
                return;
            }
        };

        let target_label = if intent.target.is_empty() {
            i18n::t("intent.no_target", self.lang, &[])
        } else {
            intent.target.clone()
        };
        let confirmation_message = i18n::t(
            &template.confirmation_action_key,
            self.lang,
            &[("target", &target_label)],
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

        let scan_ctx = ScanContext::from_expected_window(template.expected_window.as_deref());

        let mut session = SessionState::default();
        session.begin(intent.clone(), template.steps.len());

        while !session.completed && !self.cancelled.load(Ordering::Relaxed) {
            let Some((frame_before, mut step)) = self
                .wait_for_target(sink.as_ref(), &intent, &template, &scan_ctx, &session)
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

            step.instruction = self
                .instruction_for_step(
                    &step,
                    &frame_before,
                    &intent,
                    &template,
                    &input_monitor.poll(),
                    true,
                )
                .await;

            session.record(step.clone());
            self.emit_step(sink.as_ref(), &step).await;

            let mut completed = false;
            let mut last_anchor = step.anchor_xy;
            let mut rolling_frame = frame_before.clone();
            let mut polls_since_capture: u32 = 0;
            let mut target_click_count: u32 = 0;

            for poll in 0..GUIDE_MAX_POLLS {
                if self.cancelled.load(Ordering::Relaxed) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(INPUT_POLL_MS)).await;

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

    /// Perception gate: scan until target has coordinates, emitting prep overlay meanwhile.
    async fn wait_for_target<S: EventSink + ?Sized>(
        &self,
        sink: &S,
        intent: &Intent,
        template: &GuidanceTemplate,
        scan_ctx: &ScanContext,
        session: &SessionState,
    ) -> Option<(ScreenFrame, GuideStep)> {
        let mut input_monitor = InputMonitor::new();
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
        None
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
            InvalidateReason::Initial,
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
            // Lighter scan while coaching — fewer HWND walks, no vision.
            settings.max_windows = settings.max_windows.min(4);
            settings.vision_enabled = false;
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
    async fn instruction_for_step(
        &self,
        step: &GuideStep,
        frame: &ScreenFrame,
        intent: &Intent,
        template: &GuidanceTemplate,
        input: &crate::input::InputSample,
        allow_llm: bool,
    ) -> String {
        let mut fallback = if step.anchor_xy.is_none() {
            step.instruction.clone()
        } else {
            let key = match step.action {
                ActionVerb::Click => "guidance.click_target",
                ActionVerb::DoubleClick => "guidance.double_click_target",
                ActionVerb::RightClick => "guidance.right_click_target",
                ActionVerb::Type => "guidance.type_in_target",
                ActionVerb::Locate => "guidance.locate_target",
            };
            i18n::t(key, self.lang, &[("target", &step.target_text)])
        };

        if frame.quality == PerceptionQuality::DegradedUiaOnly {
            let limited = i18n::t("guidance.perception_limited", self.lang, &[]);
            if !limited.starts_with("guidance.") {
                fallback = format!("{limited} {fallback}");
            }
        }

        if !allow_llm || step.anchor_xy.is_none() || frame.elements.is_empty() {
            return fallback;
        }

        let perception = &self.settings.perception;
        let visible = frame.visible_summary(perception.prompt_max_elements);
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

        let prompt = prompts::render_instruction_step(InstructionPromptContext {
            goal: &intent.intent,
            step_index: step.index,
            total_steps: step.total,
            action: &action_verb_label(step.action, self.lang),
            target: &step.target_text,
            window_title: &window_title,
            window_list: &window_list,
            visible_elements: &visible,
            cursor_line: &cursor_line,
            target_on_screen: true,
            perception_quality: frame.quality.label(),
            warnings_line: &warnings_line,
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
                let trimmed = text.trim().lines().next().unwrap_or("").trim().to_string();
                if trimmed.len() >= 8 {
                    return trimmed;
                }
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
}

fn action_verb_label(action: ActionVerb, lang: Lang) -> String {
    let key = match action {
        ActionVerb::Click => "action.click",
        ActionVerb::DoubleClick => "action.double_click",
        ActionVerb::RightClick => "action.right_click",
        ActionVerb::Type => "action.type",
        ActionVerb::Locate => "action.locate",
    };
    i18n::t(key, lang, &[])
}
