//! Desktop / taskbar UIA walk.
//!
//! Surfaces Start, taskbar pins, and system tray as `ScreenElement`s tagged
//! with synthetic "desktop" `WindowSnapshot` so the orchestrator can guide
//! "open the Start menu" once the user requests it. The secure desktop /
//! UAC prompt is out of scope and surfaces `PerceptionWarning::SecureDesktop`.

use crate::perception::frame::{
    ElementSource, PerceptionWarning, Rect, ScreenElement, WindowId, WindowSnapshot,
};
pub const DESKTOP_WINDOW_ID: WindowId = WindowId(u64::MAX - 1);
pub const TASKBAR_WINDOW_ID: WindowId = WindowId(u64::MAX - 2);

const TASKBAR_CLASSES: &[&str] = &[
    "Shell_TrayWnd",        // Windows 10 + 11 taskbar
    "Shell_SecondaryTrayWnd", // Per-monitor taskbar
];

/// Return value of `walk_desktop_chrome` — extra elements + warnings that
/// HybridPerceiver merges into the frame.
#[derive(Debug, Default)]
pub struct DesktopCapture {
    pub windows: Vec<WindowSnapshot>,
    pub elements: Vec<ScreenElement>,
    pub warnings: Vec<PerceptionWarning>,
}

/// Walk taskbar/Start UIA tree. Cheap stub on non-Windows.
pub fn walk_desktop_chrome() -> DesktopCapture {
    #[cfg(windows)]
    {
        windows_impl::walk()
    }
    #[cfg(not(windows))]
    {
        stub_walk()
    }
}

#[cfg(not(windows))]
fn stub_walk() -> DesktopCapture {
    let bounds = Rect::new(0, 1040, 1920, 40);
    let mut elements = Vec::new();
    elements.push(ScreenElement {
        source: ElementSource::Uia,
        text: "Inicio".into(),
        bounds: Rect::new(10, 1045, 40, 30),
        window_id: TASKBAR_WINDOW_ID,
        kind: "Button".into(),
        confidence: 1.0,
        automation_id: Some("start".into()),
    });
    DesktopCapture {
        windows: vec![WindowSnapshot {
            id: TASKBAR_WINDOW_ID,
            title: "Taskbar".into(),
            class_name: "Shell_TrayWnd".into(),
            bounds,
            is_foreground: false,
            z_order: u32::MAX,
            uia_element_count: elements.len(),
        }],
        elements,
        warnings: Vec::new(),
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::{
        DesktopCapture, ElementSource, PerceptionWarning, Rect, ScreenElement, WindowSnapshot,
        TASKBAR_CLASSES, TASKBAR_WINDOW_ID,
    };
    use crate::perception::labels::humanize_label;

    use uiautomation::core::UIAutomation;
    use uiautomation::types::TreeScope;

    pub fn walk() -> DesktopCapture {
        let mut out = DesktopCapture::default();
        let automation = match UIAutomation::new() {
            Ok(a) => a,
            Err(err) => {
                tracing::warn!(
                    target: "roota.perception.desktop",
                    "UIAutomation::new failed: {err}"
                );
                return out;
            }
        };
        let condition = match automation.create_true_condition() {
            Ok(c) => c,
            Err(_) => return out,
        };

        let root = match automation.get_root_element() {
            Ok(r) => r,
            Err(err) => {
                tracing::warn!(
                    target: "roota.perception.desktop",
                    "get_root_element failed: {err}"
                );
                return out;
            }
        };

        let children = match root.find_all(TreeScope::Children, &condition) {
            Ok(c) => c,
            Err(_) => return out,
        };

        for child in children {
            let class = child.get_classname().unwrap_or_default();
            if !TASKBAR_CLASSES.iter().any(|c| class.contains(c)) {
                continue;
            }
            let descendants = match child.find_all(TreeScope::Descendants, &condition) {
                Ok(d) => d,
                Err(_) => continue,
            };
            let mut count = 0usize;
            for node in descendants {
                if let (Ok(name), Ok(rect)) = (node.get_name(), node.get_bounding_rectangle()) {
                    let automation_id =
                        node.get_automation_id().ok().filter(|s| !s.is_empty());
                    let Some(text) = humanize_label(&name, automation_id.as_deref()) else {
                        continue;
                    };
                    let width = rect.get_right() - rect.get_left();
                    let height = rect.get_bottom() - rect.get_top();
                    if width <= 2 || height <= 2 {
                        continue;
                    }
                    out.elements.push(ScreenElement {
                        source: ElementSource::Uia,
                        text,
                        bounds: Rect::new(rect.get_left(), rect.get_top(), width, height),
                        window_id: TASKBAR_WINDOW_ID,
                        kind: child
                            .get_control_type()
                            .map(|ct| format!("{ct:?}"))
                            .unwrap_or_else(|_| "TaskbarItem".into()),
                        confidence: 1.0,
                        automation_id,
                    });
                    count += 1;
                }
            }
            let bounds = match child.get_bounding_rectangle() {
                Ok(r) => Rect::new(
                    r.get_left(),
                    r.get_top(),
                    r.get_right() - r.get_left(),
                    r.get_bottom() - r.get_top(),
                ),
                Err(_) => Rect::default(),
            };
            out.windows.push(WindowSnapshot {
                id: TASKBAR_WINDOW_ID,
                title: child.get_name().unwrap_or_else(|_| "Taskbar".into()),
                class_name: class,
                bounds,
                is_foreground: false,
                z_order: u32::MAX,
                uia_element_count: count,
            });
            // Only walk the first taskbar (primary monitor).
            break;
        }

        if out.elements.is_empty() {
            out.warnings.push(PerceptionWarning::OcrUnavailable);
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(windows))]
    #[test]
    fn stub_returns_start_button() {
        let cap = walk_desktop_chrome();
        assert_eq!(cap.windows.len(), 1);
        assert!(cap.elements.iter().any(|e| e.text == "Inicio"));
    }

    #[test]
    fn desktop_ids_are_distinct_sentinels() {
        assert_ne!(DESKTOP_WINDOW_ID, TASKBAR_WINDOW_ID);
    }
}
