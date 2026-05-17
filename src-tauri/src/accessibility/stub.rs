use std::sync::Mutex;

use crate::accessibility::element::{UiElement, UiSnapshot};
use crate::accessibility::scanner::{ScanContext, Scanner};

pub struct StubScanner {
    snapshot: Mutex<UiSnapshot>,
}

impl Default for StubScanner {
    fn default() -> Self {
        Self {
            snapshot: Mutex::new(UiSnapshot {
                window: "Explorer".into(),
                elements: default_elements(),
            }),
        }
    }
}

impl StubScanner {
    pub fn set(&self, snapshot: UiSnapshot) {
        if let Ok(mut guard) = self.snapshot.lock() {
            *guard = snapshot;
        }
    }
}

impl Scanner for StubScanner {
    fn name(&self) -> &str {
        "stub"
    }

    fn snapshot_with_context(&self, _ctx: &ScanContext) -> UiSnapshot {
        self.snapshot.lock().map(|g| g.clone()).unwrap_or_default()
    }
}

#[allow(clippy::too_many_arguments)]
fn el(kind: &str, text: &str, id: &str, window: &str, x: i32, y: i32, w: i32, h: i32) -> UiElement {
    UiElement {
        kind: kind.into(),
        text: text.into(),
        x,
        y,
        width: w,
        height: h,
        automation_id: Some(id.into()),
        window: window.into(),
    }
}

pub fn default_elements() -> Vec<UiElement> {
    vec![
        el(
            "button",
            "Descargas",
            "downloads",
            "Explorer",
            120,
            340,
            160,
            32,
        ),
        el(
            "button",
            "Documentos",
            "documents",
            "Explorer",
            120,
            380,
            160,
            32,
        ),
        el(
            "button",
            "Imágenes",
            "pictures",
            "Explorer",
            120,
            420,
            160,
            32,
        ),
        el(
            "button",
            "Escritorio",
            "desktop",
            "Explorer",
            120,
            460,
            160,
            32,
        ),
        el("text", "Buscar", "search_box", "Explorer", 300, 80, 400, 28),
        el(
            "button",
            "Nueva pestaña",
            "new_tab",
            "Chrome",
            20,
            20,
            120,
            28,
        ),
        el("button", "Redactar", "compose", "Gmail", 40, 160, 120, 40),
        el(
            "button",
            "Bandeja de entrada",
            "inbox",
            "Gmail",
            40,
            220,
            200,
            32,
        ),
        el("button", "Imprimir", "print", "Word", 80, 140, 120, 32),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_snapshot_has_descargas() {
        let s = StubScanner::default();
        let snap = s.snapshot();
        assert_eq!(snap.window, "Explorer");
        assert!(snap.find("Descargas").is_some());
    }
}
