//! Drives the classify -> confirm -> step-loop pipeline. Pure async,
//! emits typed events through a Tauri channel-style sender so the
//! frontend can listen without coupling the brain to Tauri itself.

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
use crate::orchestration::state::{ActionVerb, GuideStep, SessionState};
use crate::orchestration::templates::GuidanceTemplate;
use crate::orchestration::templates::TemplateRegistry;
use crate::prompts::{self, InstructionPromptContext};
use crate::settings::Lang;

const SCAN_RETRY_ATTEMPTS: u32 = 10;
const SCAN_RETRY_MS: u64 = 800;
const POST_CONFIRM_FOCUS_MS: u64 = 900;
const STEP_LLM_TIMEOUT_SECS: f32 = 12.0;

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

        tokio::time::sleep(Duration::from_millis(POST_CONFIRM_FOCUS_MS)).await;

        let scan_ctx = ScanContext::from_expected_window(
            template.expected_window.as_deref(),
        );

        let mut session = SessionState::default();
        session.begin(intent.clone(), template.steps.len());

        while !session.completed && !self.cancelled.load(Ordering::Relaxed) {
            let (snapshot_before, mut step) = match self
                .prepare_step(&intent, &template, &scan_ctx, &session)
                .await
            {
                Ok(pair) => pair,
                Err(err) => {
                    sink.send(OrchestratorEvent::Error {
                        message: err.to_string(),
                    })
                    .await;
                    sink.send(OrchestratorEvent::Finished).await;
                    return;
                }
            };

            step.instruction = self
                .instruction_from_screen(&step, &snapshot_before, &intent, &template, &session)
                .await;

            session.record(step.clone());
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

            let mut completed = false;
            for poll in 0..60u32 {
                if self.cancelled.load(Ordering::Relaxed) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
                let snapshot_after = self.scanner.snapshot_with_context(&scan_ctx);
                let outcome = self.detector.is_completed(
                    &step,
                    &snapshot_before,
                    &snapshot_after,
                    poll,
                );
                if outcome.completed {
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

    /// Scan until the target appears on screen or retries exhaust (PRD: read then guide).
    async fn prepare_step(
        &self,
        intent: &crate::orchestration::state::Intent,
        template: &GuidanceTemplate,
        scan_ctx: &ScanContext,
        session: &SessionState,
    ) -> Result<(UiSnapshot, GuideStep), crate::orchestration::decision::StepResolutionError> {
        let mut last_snapshot = UiSnapshot::default();
        let mut last_step = None;

        for attempt in 0..SCAN_RETRY_ATTEMPTS {
            if self.cancelled.load(Ordering::Relaxed) {
                break;
            }
            let snapshot = self.scanner.snapshot_with_context(scan_ctx);
            last_snapshot = snapshot.clone();
            let step = self
                .decision
                .next_step(intent, template, &snapshot, session)?;
            let found = step.anchor_xy.is_some();
            tracing::info!(
                target: "roota.orchestrator",
                attempt,
                window = %snapshot.window,
                elements = snapshot.elements.len(),
                target = %step.target_text,
                found,
                "prepare_step scan"
            );
            last_step = Some(step.clone());
            if found {
                return Ok((snapshot, step));
            }
            if attempt + 1 < SCAN_RETRY_ATTEMPTS {
                tokio::time::sleep(Duration::from_millis(SCAN_RETRY_MS)).await;
            }
        }

        let step = last_step.ok_or(crate::orchestration::decision::StepResolutionError::NoMoreSteps)?;
        Ok((last_snapshot, step))
    }

    /// LLM generates the instruction from what is actually visible (PRD §8.3).
    async fn instruction_from_screen(
        &self,
        step: &GuideStep,
        snapshot: &UiSnapshot,
        intent: &crate::orchestration::state::Intent,
        template: &GuidanceTemplate,
        _session: &SessionState,
    ) -> String {
        let fallback = step.instruction.clone();
        let visible = if snapshot.elements.is_empty() {
            i18n::t("guidance.screen_empty", self.lang, &[]).to_string()
        } else {
            snapshot.visible_summary(35)
        };
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
            target_on_screen: step.anchor_xy.is_some(),
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
                    tracing::info!(target: "roota.orchestrator", "LLM step instruction ready");
                    return trimmed;
                }
            }
            Ok(Err(err)) => {
                tracing::warn!(target: "roota.orchestrator", "LLM instruction failed: {err}");
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
