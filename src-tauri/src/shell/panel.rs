//! Show / hide / toggle the Roota assistant panel (`main` webview).

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Runtime, WebviewWindow};

pub const PANEL_LABEL: &str = "main";
pub const TOGGLE_SHORTCUT: &str = "Ctrl+Shift+Space";
pub const EVENT_PANEL_VISIBLE: &str = "roota://panel-visible";

const MARGIN_PX: i32 = 24;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelVisiblePayload {
    pub visible: bool,
}

pub fn panel_window<R: Runtime>(app: &AppHandle<R>) -> Option<WebviewWindow<R>> {
    app.get_webview_window(PANEL_LABEL)
}

pub fn is_visible<R: Runtime>(app: &AppHandle<R>) -> bool {
    panel_window(app)
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false)
}

/// Place the assistant panel centered along the top edge of the active monitor.
pub fn position_top_center<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    let monitor = window
        .current_monitor()?
        .or_else(|| window.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else {
        return Ok(());
    };
    let screen = monitor.size();
    let win = window.outer_size()?;
    let x = ((screen.width as i32 - win.width as i32) / 2).max(MARGIN_PX);
    let y = MARGIN_PX;
    window.set_position(PhysicalPosition::new(x, y))?;
    Ok(())
}

fn emit_visibility<R: Runtime>(app: &AppHandle<R>, visible: bool) {
    let payload = PanelVisiblePayload { visible };
    if let Err(err) = app.emit_to(PANEL_LABEL, EVENT_PANEL_VISIBLE, payload) {
        tracing::warn!(target: "roota.shell.panel", "emit panel-visible failed: {err}");
    }
}

pub fn show<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let Some(window) = panel_window(app) else {
        return Ok(());
    };
    position_top_center(&window)?;
    let _ = window.set_skip_taskbar(false);
    window.show()?;
    let _ = window.unminimize();
    window.set_focus()?;
    emit_visibility(app, true);
    tracing::info!(target: "roota.shell.panel", "panel shown");
    Ok(())
}

pub fn hide<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let Some(window) = panel_window(app) else {
        return Ok(());
    };
    window.hide()?;
    let _ = window.set_skip_taskbar(true);
    emit_visibility(app, false);
    tracing::info!(target: "roota.shell.panel", "panel hidden");
    Ok(())
}

pub fn toggle<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<bool> {
    if is_visible(app) {
        hide(app)?;
        Ok(false)
    } else {
        show(app)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_shortcut_is_modifiers_not_single_key() {
        assert!(TOGGLE_SHORTCUT.contains("Ctrl"));
        assert!(TOGGLE_SHORTCUT.contains("Shift"));
    }
}
