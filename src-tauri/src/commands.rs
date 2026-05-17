//! Public Tauri command surface. Thin wrappers around the orchestrator.

use std::sync::Arc;

use serde::Serialize;
use tauri::{ipc::Channel, AppHandle, Emitter, Manager, State, WebviewWindow};
use thiserror::Error;

use crate::i18n;
use crate::orchestration::state::{ActionVerb, GuideStep};
use crate::orchestration::OrchestratorEvent;
use crate::overlay::OverlayRect;
use crate::settings::Lang;
use crate::AppState;
use crate::RuntimeHandle;

pub const EVENT_GUIDANCE: &str = "roota://guidance";
pub const EVENT_ANCHOR: &str = "roota://anchor";
pub const EVENT_ANCHOR_CLEAR: &str = "roota://anchor-clear";

#[derive(Debug, Error)]
pub enum AppError {
    #[error("invalid arguments: {0}")]
    BadInput(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GuidancePayload {
    pub active: bool,
    pub instruction: String,
    pub action: ActionVerb,
    pub step_index: usize,
    pub step_total: usize,
    pub target_label: String,
    pub click_hint: String,
    pub has_target: bool,
    pub rect: Option<OverlayRect>,
}

#[derive(Serialize, Clone)]
struct AnchorPayload {
    x: i32,
    y: i32,
    label: String,
}

fn action_click_hint(action: ActionVerb, lang: Lang) -> String {
    let key = match action {
        ActionVerb::Click => "guidance.hint.click",
        ActionVerb::DoubleClick => "guidance.hint.double_click",
        ActionVerb::RightClick => "guidance.hint.right_click",
        ActionVerb::Type => "guidance.hint.type",
        ActionVerb::Locate => "guidance.hint.locate",
    };
    i18n::t(key, lang, &[])
}

fn step_rect(overlay: &WebviewWindow, step: &GuideStep) -> Option<OverlayRect> {
    if let Some((x, y, w, h)) = step.anchor_bounds {
        return crate::overlay::screen_rect_to_overlay(overlay, x, y, w, h);
    }
    if let Some((cx, cy)) = step.anchor_xy {
        let (lx, ly) = crate::overlay::screen_center_to_overlay(overlay, cx, cy)?;
        return Some(OverlayRect {
            x: lx - 40.0,
            y: ly - 20.0,
            width: 80.0,
            height: 40.0,
        });
    }
    None
}

fn emit_guidance(app: &AppHandle, step: &GuideStep, lang: Lang) {
    let Some(overlay) = app.get_webview_window("overlay") else {
        return;
    };
    let rect = step_rect(&overlay, step);
    let has_target = rect.is_some();
    let payload = GuidancePayload {
        active: true,
        instruction: step.instruction.clone(),
        action: step.action,
        step_index: step.index,
        step_total: step.total,
        target_label: step.target_text.clone(),
        click_hint: action_click_hint(step.action, lang),
        has_target,
        rect,
    };
    if !overlay.is_visible().unwrap_or(false) {
        let _ = overlay.show();
    }
    let _ = overlay.set_ignore_cursor_events(true);
    if let Err(err) = app.emit_to("overlay", EVENT_GUIDANCE, payload.clone()) {
        tracing::warn!(target: "roota.overlay", "emit guidance failed: {err}");
    }
    if let Some(r) = &payload.rect {
        let cx = (r.x + r.width / 2.0).round() as i32;
        let cy = (r.y + r.height / 2.0).round() as i32;
        let _ = app.emit_to(
            "overlay",
            EVENT_ANCHOR,
            AnchorPayload {
                x: cx,
                y: cy,
                label: step.target_text.clone(),
            },
        );
    } else {
        let _ = app.emit_to("overlay", EVENT_ANCHOR_CLEAR, ());
    }
    tracing::info!(
        target: "roota.overlay",
        step = step.index,
        target = %step.target_text,
        has_rect = has_target,
        "guidance active"
    );
}

fn clear_guidance(app: &AppHandle, lang: Lang) {
    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = app.emit_to(
            "overlay",
            EVENT_GUIDANCE,
            GuidancePayload {
                active: false,
                instruction: String::new(),
                action: ActionVerb::Locate,
                step_index: 0,
                step_total: 0,
                target_label: String::new(),
                click_hint: String::new(),
                has_target: false,
                rect: None,
            },
        );
        let _ = app.emit_to("overlay", EVENT_ANCHOR_CLEAR, ());
        let _ = overlay.hide();
    }
    let _ = lang;
}

struct ChannelSink {
    channel: Channel<OrchestratorEvent>,
    app: AppHandle,
    lang: Lang,
}

#[async_trait::async_trait]
impl crate::orchestration::orchestrator::EventSink for ChannelSink {
    async fn send(&self, event: OrchestratorEvent) {
        match &event {
            OrchestratorEvent::StepReady { step } => {
                emit_guidance(&self.app, step, self.lang);
            }
            OrchestratorEvent::AnchorChanged { .. } => {}
            OrchestratorEvent::GoalCompleted { .. }
            | OrchestratorEvent::Error { .. }
            | OrchestratorEvent::Finished => {
                clear_guidance(&self.app, self.lang);
            }
            _ => {}
        }
        if let Err(err) = self.channel.send(event) {
            tracing::warn!(target: "roota.commands", "channel send failed: {err}");
        }
    }
}

#[tauri::command]
pub async fn start_session(
    utterance: String,
    on_event: Channel<OrchestratorEvent>,
    app: AppHandle,
    state: State<'_, AppState>,
    runtime: State<'_, RuntimeHandle>,
) -> Result<(), AppError> {
    let utterance = utterance.trim().to_string();
    if utterance.is_empty() {
        return Err(AppError::BadInput("empty utterance".into()));
    }

    let orchestrator = state.orchestrator.clone();
    let lang = state.settings.ui_language;
    let sink = Arc::new(ChannelSink {
        channel: on_event,
        app: app.clone(),
        lang,
    });

    let handle = runtime.0.spawn(async move {
        orchestrator.run(utterance, sink).await;
    });

    let mut slot = state.running_session.lock().await;
    if let Some(prev) = slot.replace(handle) {
        prev.abort();
    }
    Ok(())
}

#[tauri::command]
pub async fn confirm_response(
    accepted: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    if accepted {
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            let _ = crate::shell::panel::hide(&app);
        }
    }
    state.orchestrator.resolve_confirmation(accepted).await;
    Ok(())
}

#[tauri::command]
pub async fn cancel_session(state: State<'_, AppState>) -> Result<(), AppError> {
    state.orchestrator.cancel().await;
    let mut slot = state.running_session.lock().await;
    if let Some(handle) = slot.take() {
        handle.abort();
    }
    Ok(())
}

#[tauri::command]
pub async fn show_overlay_anchor(
    x: i32,
    y: i32,
    label: String,
    app: AppHandle,
) -> Result<(), AppError> {
    if let Some(overlay) = app.get_webview_window("overlay") {
        if !overlay.is_visible().unwrap_or(false) {
            let _ = overlay.show();
        }
        let _ = overlay.set_ignore_cursor_events(true);
        if let Err(err) = app.emit_to("overlay", EVENT_ANCHOR, AnchorPayload { x, y, label }) {
            tracing::warn!(target: "roota.overlay", "emit anchor failed: {err}");
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn clear_overlay(app: AppHandle) -> Result<(), AppError> {
    clear_guidance(&app, Lang::Es);
    Ok(())
}

#[tauri::command]
pub fn toggle_panel(app: AppHandle) -> Result<bool, AppError> {
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        return crate::shell::panel::toggle(&app).map_err(|e| AppError::Internal(e.to_string()));
    }
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        let _ = app;
        Ok(false)
    }
}

#[tauri::command]
pub fn panel_visible(app: AppHandle) -> Result<bool, AppError> {
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        return Ok(crate::shell::panel::is_visible(&app));
    }
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        let _ = app;
        Ok(false)
    }
}
