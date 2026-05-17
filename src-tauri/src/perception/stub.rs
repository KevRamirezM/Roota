//! Deterministic multi-window perceiver — drives unit tests and is the
//! production fallback when no live UIA backend is available.

use std::sync::Mutex;

use crate::perception::context::PerceptionContext;
use crate::perception::error::PerceptionError;
use crate::perception::frame::{
    now_ms, ElementSource, PerceptionQuality, PerceptionWarning, Rect, ScreenElement, ScreenFrame,
    WindowId, WindowSnapshot,
};
use crate::perception::Perceiver;

pub struct StubPerceiver {
    frame: Mutex<ScreenFrame>,
}

impl Default for StubPerceiver {
    fn default() -> Self {
        Self {
            frame: Mutex::new(default_frame()),
        }
    }
}

impl StubPerceiver {
    pub fn new(frame: ScreenFrame) -> Self {
        Self {
            frame: Mutex::new(frame),
        }
    }

    pub fn set(&self, frame: ScreenFrame) {
        if let Ok(mut g) = self.frame.lock() {
            *g = frame;
        }
    }

    pub fn snapshot(&self) -> ScreenFrame {
        self.frame
            .lock()
            .map(|g| g.clone())
            .unwrap_or_else(|_| ScreenFrame::empty())
    }
}

impl Perceiver for StubPerceiver {
    fn name(&self) -> &str {
        "stub"
    }

    fn capture(&self, ctx: &PerceptionContext) -> Result<ScreenFrame, PerceptionError> {
        let mut frame = self.snapshot();
        frame.captured_at_ms = now_ms();
        frame.cursor = ctx.cursor;
        if frame.windows.is_empty() {
            frame = default_frame();
            frame.captured_at_ms = now_ms();
            frame.cursor = ctx.cursor;
        }
        Ok(frame)
    }
}

fn el(
    text: &str,
    automation_id: &str,
    window_id: u64,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) -> ScreenElement {
    ScreenElement {
        source: ElementSource::Uia,
        text: text.into(),
        bounds: Rect::new(x, y, w, h),
        window_id: WindowId(window_id),
        kind: "Button".into(),
        confidence: 1.0,
        automation_id: Some(automation_id.into()),
    }
}

pub fn default_elements() -> Vec<ScreenElement> {
    vec![
        el("Descargas", "downloads", 1, 120, 340, 160, 32),
        el("Documentos", "documents", 1, 120, 380, 160, 32),
        el("Imágenes", "pictures", 1, 120, 420, 160, 32),
        el("Escritorio", "desktop", 1, 120, 460, 160, 32),
        ScreenElement {
            source: ElementSource::Uia,
            text: "Buscar".into(),
            bounds: Rect::new(300, 80, 400, 28),
            window_id: WindowId(1),
            kind: "Text".into(),
            confidence: 1.0,
            automation_id: Some("search_box".into()),
        },
        el("Nueva pestaña", "new_tab", 2, 20, 20, 120, 28),
        el("Redactar", "compose", 3, 40, 160, 120, 40),
        el("Bandeja de entrada", "inbox", 3, 40, 220, 200, 32),
        el("Imprimir", "print", 4, 80, 140, 120, 32),
    ]
}

pub fn default_frame() -> ScreenFrame {
    let windows = vec![
        WindowSnapshot {
            id: WindowId(1),
            title: "Inicio - Explorador de archivos".into(),
            class_name: "CabinetWClass".into(),
            bounds: Rect::new(0, 0, 1280, 720),
            is_foreground: true,
            z_order: 0,
            uia_element_count: 5,
        },
        WindowSnapshot {
            id: WindowId(2),
            title: "Google Chrome".into(),
            class_name: "Chrome_WidgetWin_1".into(),
            bounds: Rect::new(1280, 0, 1280, 720),
            is_foreground: false,
            z_order: 1,
            uia_element_count: 1,
        },
        WindowSnapshot {
            id: WindowId(3),
            title: "Gmail — Bandeja de entrada".into(),
            class_name: "Chrome_WidgetWin_1".into(),
            bounds: Rect::new(0, 720, 1280, 360),
            is_foreground: false,
            z_order: 2,
            uia_element_count: 2,
        },
        WindowSnapshot {
            id: WindowId(4),
            title: "Documento1 - Word".into(),
            class_name: "OpusApp".into(),
            bounds: Rect::new(1280, 720, 1280, 360),
            is_foreground: false,
            z_order: 3,
            uia_element_count: 1,
        },
    ];
    ScreenFrame {
        captured_at_ms: 0,
        cursor: Default::default(),
        primary_window_id: WindowId(1),
        windows,
        elements: default_elements(),
        quality: PerceptionQuality::Full,
        warnings: Vec::new(),
    }
}

/// Convenience: a degraded frame (sparse UIA tree) for testing the OCR gate.
pub fn sparse_frame() -> ScreenFrame {
    let mut f = default_frame();
    f.elements.retain(|e| e.window_id != WindowId(1));
    f.quality = PerceptionQuality::DegradedUiaOnly;
    f.warnings.push(PerceptionWarning::LowElementCount {
        window_id: WindowId(1),
        count: 0,
    });
    f
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::state::ActionVerb;

    #[test]
    fn stub_default_finds_descargas() {
        let s = StubPerceiver::default();
        let frame = s.capture(&PerceptionContext::default()).unwrap();
        let found = frame.find_best_for_action(&["descargas".into()], ActionVerb::Click);
        assert!(found.is_some());
        assert_eq!(found.unwrap().window_id, WindowId(1));
    }

    #[test]
    fn stub_carries_cursor_into_frame() {
        let s = StubPerceiver::default();
        let ctx = PerceptionContext {
            cursor: crate::input::PhysicalPoint { x: 500, y: 200 },
            ..PerceptionContext::default()
        };
        let frame = s.capture(&ctx).unwrap();
        assert_eq!(frame.cursor.x, 500);
    }
}
