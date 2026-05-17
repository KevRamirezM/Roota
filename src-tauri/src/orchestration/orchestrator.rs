//! Drives the classify -> confirm -> perceive -> point -> verify pipeline (PRD §8).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tokio::sync::oneshot;
use tokio::sync::Mutex;

use crate::accessibility::element::UiSnapshot;
use crate::accessibility::scanner::{ScanContext, Scanner};
use crate::i18n;
use crate::llm::LlmClient;
use crate::orchestration::decision::DecisionEngine;
use crate::orchestration::detector::StateDetector;
use crate::orchestration::intent::IntentRecognizer;
use crate::orchestration::state::{ActionVerb, GuideStep, Intent, SessionState};
use crate::orchestration::templates::GuidanceTemplate;
use crate::orchestration::templates::TemplateRegistry;
use crate::prompts::{self, InstructionPromptContext};
use crate::settings::Lang;

const PERCEPTION_POLL_MS: u64 = 700;
const PERCEPTION_MAX_ATTEMPTS: u32 = 28;
const GUIDE_POLL_MS: u64 = 500;
const GUIDE_MAX_POLLS: u32 = 60;
const POST_CONFIRM_MS: u64 = 600;
const STEP_LLM_TIMEOUT_SECS: f32 = 6.0;

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
    scanner: Arc<dyn Scanner>,
    templates: Arc<TemplateRegistry>,
    recognizer: IntentRecognizer,
    decision: DecisionEngine,
    detector: StateDetector,
    pending_confirmation: Mutex<Option<oneshot::Sender<bool>>>,
    cancelled: AtomicBool,
    lang: Lang,
}

impl Orchestrator {
    pub fn new(
        llm: Arc<dyn LlmClient>,
        scanner: Arc<dyn Scanner>,
        templates: Arc<TemplateRegistry>,
        lang: Lang,
        intent_timeout_secs: f32,
    ) -> Self {
        let recognizer = IntentRecognizer::new(
            llm.clone(),
            templates.clone(),
            lang,
            intent_timeout_secs,
        );
        Self {
            llm,
            scanner,
            templates,
            recognizer,
            decision: DecisionEngine::new(lang),
            detector: StateDetector,
            pending_confirmation: Mutex::new(None),
            cancelled: AtomicBool::new(false),
            lang,
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
        self.ensure_target_app(&intent);

        let scan_ctx =
            ScanContext::from_expected_window(template.expected_window.as_deref());

        let mut session = SessionState::default();
        session.begin(intent.clone(), template.steps.len());

        while !session.completed && !self.cancelled.load(Ordering::Relaxed) {
            let Some((snapshot_before, mut step)) = self
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

            step.instruction = self
                .instruction_for_step(&step, &snapshot_before, &intent, &template, true)
                .await;

            session.record(step.clone());
            self.emit_step(sink.as_ref(), &step).await;

            let mut completed = false;
            let mut last_anchor = step.anchor_xy;

            for poll in 0..GUIDE_MAX_POLLS {
                if self.cancelled.load(Ordering::Relaxed) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(GUIDE_POLL_MS)).await;

                let snapshot_after = self.scanner.snapshot_with_context(&scan_ctx);
                let refreshed = match self.decision.next_step(
                    &intent,
                    &template,
                    &snapshot_after,
                    &session,
                ) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                if refreshed.anchor_xy.is_some()
                    && (last_anchor != refreshed.anchor_xy
                        || poll == 0
                        || refreshed.anchor_bounds != step.anchor_bounds)
                {
                    let mut updated = refreshed;
                    updated.instruction = self
                        .instruction_for_step(
                            &updated,
                            &snapshot_after,
                            &intent,
                            &template,
                            poll == 0,
                        )
                        .await;
                    step = updated;
                    last_anchor = step.anchor_xy;
                    self.emit_step(sink.as_ref(), &step).await;
                }

                let outcome = self.detector.is_completed(
                    &step,
                    &snapshot_before,
                    &snapshot_after,
                    poll,
                );
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

    fn ensure_target_app(&self, intent: &Intent) {
        if intent.intent == "open_folder" || intent.intent == "move_file" || intent.intent == "delete_file"
        {
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            crate::shell::explorer::launch_file_explorer();
        }
    }

    /// Perception gate: scan until target has coordinates, emitting prep overlay meanwhile.
    async fn wait_for_target<S: EventSink + ?Sized>(
        &self,
        sink: &S,
        intent: &Intent,
        template: &GuidanceTemplate,
        scan_ctx: &ScanContext,
        session: &SessionState,
    ) -> Option<(UiSnapshot, GuideStep)> {
        for attempt in 0..PERCEPTION_MAX_ATTEMPTS {
            if self.cancelled.load(Ordering::Relaxed) {
                return None;
            }

            let snapshot = self.scanner.snapshot_with_context(scan_ctx);
            if let Ok(step) = self
                .decision
                .next_step(intent, template, &snapshot, session)
            {
                if step.anchor_xy.is_some() {
                    tracing::info!(
                        target: "roota.orchestrator",
                        attempt,
                        window = %snapshot.window,
                        elements = snapshot.elements.len(),
                        "target located on screen"
                    );
                    return Some((snapshot, step));
                }
            }

            if attempt == 0 || attempt.is_multiple_of(3) {
                let prep = self.prep_step(session, template, &snapshot, intent);
                self.emit_step(sink, &prep).await;
            }

            tokio::time::sleep(Duration::from_millis(PERCEPTION_POLL_MS)).await;
        }
        None
    }

    fn prep_step(
        &self,
        session: &SessionState,
        template: &GuidanceTemplate,
        snapshot: &UiSnapshot,
        intent: &Intent,
    ) -> GuideStep {
        let instruction = if snapshot.window.is_empty() || snapshot.elements.is_empty() {
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
                    ("window", &snapshot.window),
                    ("count", &snapshot.elements.len().to_string()),
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
        snapshot: &UiSnapshot,
        intent: &Intent,
        template: &GuidanceTemplate,
        allow_llm: bool,
    ) -> String {
        let fallback = if step.anchor_xy.is_none() {
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

        if !allow_llm || step.anchor_xy.is_none() || snapshot.elements.is_empty() {
            return fallback;
        }

        let visible = snapshot.visible_summary(35);
        let window_title = if snapshot.window.is_empty() {
            template
                .expected_window
                .clone()
                .unwrap_or_else(|| "la aplicación".into())
        } else {
            snapshot.window.clone()
        };

        let prompt = prompts::render_instruction_step(InstructionPromptContext {
            goal: &intent.intent,
            step_index: step.index,
            total_steps: step.total,
            action: &action_verb_label(step.action, self.lang),
            target: &step.target_text,
            window_title: &window_title,
            visible_elements: &visible,
            target_on_screen: true,
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

    pub fn scanner_name(&self) -> &str {
        self.scanner.name()
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
