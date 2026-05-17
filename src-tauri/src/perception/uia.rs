//! Multi-window UI Automation walker. Given a list of ranked windows it
//! returns ScreenElements tagged with their owning `WindowId`, plus
//! lightweight `WindowSnapshot` metadata.
//!
//! Non-Windows builds return an empty capture (the orchestrator falls
//! back to `StubPerceiver` in tests).

use crate::perception::error::PerceptionError;
use crate::perception::frame::{
    ElementSource, PerceptionWarning, Rect, ScreenElement, WindowId, WindowSnapshot,
};
use crate::perception::window_score::RankedWindow;

/// Skip these UIA control types — they pollute the prompt without being
/// useful click targets.
pub(crate) const SKIP_CONTROL_MARKERS: &[&str] = &[
    "TitleBar",
    "Scrollbar",
    "Thumb",
    "ToolTip",
    "Separator",
];

/// Skip chrome containers but not toolbar buttons (`ToolBarButton` matched `ToolBar`).
pub(crate) fn should_skip_control(kind: &str) -> bool {
    if is_toolbar_container(kind) {
        return true;
    }
    SKIP_CONTROL_MARKERS.iter().any(|m| kind.contains(m))
}

pub(crate) fn is_toolbar_container(kind: &str) -> bool {
    kind.contains("ToolBar")
        && !kind.contains("Button")
        && !kind.contains("Item")
        && !kind.contains("Menu")
}

/// Hard ceiling so a runaway tree walk cannot blow up RAM / latency.
pub const MAX_ELEMENTS_TOTAL: usize = 800;
pub const MAX_ELEMENTS_PER_WINDOW: usize = 600;

#[derive(Debug, Default)]
pub struct UiaCapture {
    pub windows: Vec<WindowSnapshot>,
    pub elements: Vec<ScreenElement>,
    pub warnings: Vec<PerceptionWarning>,
}

pub struct UiaPerceiver;

impl Default for UiaPerceiver {
    fn default() -> Self {
        Self
    }
}

impl UiaPerceiver {
    pub fn new() -> Self {
        Self
    }

    pub fn name(&self) -> &'static str {
        "uia-multi"
    }

    /// Walk UIA descendants for each ranked window and aggregate.
    ///
    /// The caller (HybridPerceiver) is responsible for assigning the
    /// primary window and building the final ScreenFrame.
    #[allow(clippy::needless_return)]
    pub fn capture(&self, ranked: &[RankedWindow]) -> Result<UiaCapture, PerceptionError> {
        #[cfg(windows)]
        {
            return windows_impl::capture(ranked);
        }
        #[cfg(not(windows))]
        {
            return Ok(stub_capture(ranked));
        }
    }
}

#[cfg(not(windows))]
fn stub_capture(ranked: &[RankedWindow]) -> UiaCapture {
    let mut windows = Vec::new();
    let mut elements = Vec::new();
    for r in ranked {
        windows.push(WindowSnapshot {
            id: r.meta.id,
            title: r.meta.title.clone(),
            class_name: r.meta.class_name.clone(),
            bounds: r.meta.bounds,
            is_foreground: r.meta.is_foreground,
            z_order: r.meta.z_order,
            uia_element_count: 1,
        });
        elements.push(ScreenElement {
            source: ElementSource::Uia,
            text: format!("Botón {}", r.meta.title),
            bounds: Rect::new(
                r.meta.bounds.x + 20,
                r.meta.bounds.y + 20,
                160,
                32,
            ),
            window_id: r.meta.id,
            kind: "Button".into(),
            confidence: 1.0,
            automation_id: Some(r.meta.title.to_lowercase()),
        });
    }
    UiaCapture {
        windows,
        elements,
        warnings: Vec::new(),
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::{
        ElementSource, PerceptionError, PerceptionWarning, Rect, RankedWindow, ScreenElement,
        UiaCapture, WindowId, WindowSnapshot, MAX_ELEMENTS_PER_WINDOW, MAX_ELEMENTS_TOTAL,
    };

    use std::ffi::c_void;

    use uiautomation::core::UIAutomation;
    use uiautomation::types::TreeScope;
    use uiautomation::UIElement;
    use windows::Win32::Foundation::HWND;

    pub fn capture(ranked: &[RankedWindow]) -> Result<UiaCapture, PerceptionError> {
        let automation = UIAutomation::new()
            .map_err(|e| PerceptionError::Uia(format!("UIAutomation::new: {e}")))?;
        let true_condition = automation
            .create_true_condition()
            .map_err(|e| PerceptionError::Uia(format!("create_true_condition: {e}")))?;

        let per_window_cap = if ranked.is_empty() {
            MAX_ELEMENTS_PER_WINDOW
        } else {
            (MAX_ELEMENTS_TOTAL / ranked.len()).clamp(60, MAX_ELEMENTS_PER_WINDOW)
        };

        let mut windows_out = Vec::new();
        let mut elements_out = Vec::new();
        let mut warnings = Vec::new();

        for r in ranked {
            if elements_out.len() >= MAX_ELEMENTS_TOTAL {
                break;
            }

            let hwnd = HWND(r.meta.id.0 as *mut c_void);
            let root = match automation.element_from_handle(hwnd.into()) {
                Ok(el) => el,
                Err(err) => {
                    tracing::warn!(
                        target: "roota.perception.uia",
                        title = %r.meta.title,
                        "element_from_handle failed: {err}"
                    );
                    windows_out.push(WindowSnapshot {
                        id: r.meta.id,
                        title: r.meta.title.clone(),
                        class_name: r.meta.class_name.clone(),
                        bounds: r.meta.bounds,
                        is_foreground: r.meta.is_foreground,
                        z_order: r.meta.z_order,
                        uia_element_count: 0,
                    });
                    continue;
                }
            };

            let descendants = match root.find_all(TreeScope::Descendants, &true_condition) {
                Ok(d) => d,
                Err(err) => {
                    tracing::warn!(
                        target: "roota.perception.uia",
                        title = %r.meta.title,
                        "find_all failed: {err}"
                    );
                    windows_out.push(WindowSnapshot {
                        id: r.meta.id,
                        title: r.meta.title.clone(),
                        class_name: r.meta.class_name.clone(),
                        bounds: r.meta.bounds,
                        is_foreground: r.meta.is_foreground,
                        z_order: r.meta.z_order,
                        uia_element_count: 0,
                    });
                    continue;
                }
            };

            let mut count = 0usize;
            for node in descendants {
                if count >= per_window_cap || elements_out.len() >= MAX_ELEMENTS_TOTAL {
                    break;
                }
                if let Some(el) = to_screen_element(&node, r.meta.id) {
                    elements_out.push(el);
                    count += 1;
                }
            }

            if count == 0 {
                warnings.push(PerceptionWarning::LowElementCount {
                    window_id: r.meta.id,
                    count,
                });
            }

            windows_out.push(WindowSnapshot {
                id: r.meta.id,
                title: r.meta.title.clone(),
                class_name: r.meta.class_name.clone(),
                bounds: r.meta.bounds,
                is_foreground: r.meta.is_foreground,
                z_order: r.meta.z_order,
                uia_element_count: count,
            });

            tracing::debug!(
                target: "roota.perception.uia",
                title = %r.meta.title,
                hwnd = r.meta.id.0,
                elements = count,
                "walked window"
            );
        }

        Ok(UiaCapture {
            windows: windows_out,
            elements: elements_out,
            warnings,
        })
    }

    fn to_screen_element(node: &UIElement, window_id: WindowId) -> Option<ScreenElement> {
        let control_type = node.get_control_type().ok()?;
        let kind = format!("{control_type:?}");
        if super::should_skip_control(&kind) {
            return None;
        }

        let text = element_label(node);
        if text.trim().is_empty() {
            return None;
        }

        let rect = node.get_bounding_rectangle().ok()?;
        let width = rect.get_right() - rect.get_left();
        let height = rect.get_bottom() - rect.get_top();
        if width <= 2 || height <= 2 || width > 4000 || height > 3000 {
            return None;
        }

        let automation_id = node.get_automation_id().ok().filter(|s| !s.is_empty());

        Some(ScreenElement {
            source: ElementSource::Uia,
            text,
            bounds: Rect::new(rect.get_left(), rect.get_top(), width, height),
            window_id,
            kind,
            confidence: 1.0,
            automation_id,
        })
    }

    fn element_label(node: &UIElement) -> String {
        let automation_id = node.get_automation_id().ok().filter(|s| !s.is_empty());
        let name = node.get_name().unwrap_or_default();
        if let Some(label) = crate::perception::labels::humanize_label(&name, automation_id.as_deref())
        {
            return label;
        }
        let help = node
            .get_help_text()
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_default();
        crate::perception::labels::humanize_label(&help, automation_id.as_deref()).unwrap_or(help)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perception::window_enum::WindowMeta;
    use crate::perception::window_score::RankedWindow;

    #[allow(dead_code)]
    fn ranked(id: u64, title: &str) -> RankedWindow {
        RankedWindow {
            meta: WindowMeta {
                id: WindowId(id),
                title: title.into(),
                class_name: "CabinetWClass".into(),
                bounds: Rect::new(0, 0, 800, 600),
                is_foreground: false,
                is_visible: true,
                is_minimized: false,
                is_roota: false,
                owner_id: None,
                z_order: 0,
            },
            score: 10,
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn non_windows_stub_returns_one_element_per_window() {
        let perceiver = UiaPerceiver::new();
        let cap = perceiver
            .capture(&[ranked(1, "Explorer"), ranked(2, "Chrome")])
            .unwrap();
        assert_eq!(cap.windows.len(), 2);
        assert_eq!(cap.elements.len(), 2);
        assert!(cap
            .elements
            .iter()
            .any(|e| e.window_id == WindowId(1) && e.text.contains("Explorer")));
    }

    #[test]
    fn perceiver_name_is_stable() {
        assert_eq!(UiaPerceiver::new().name(), "uia-multi");
    }

    #[test]
    fn toolbar_button_not_skipped_toolbar_container_is() {
        assert!(!should_skip_control("ToolBarButton"));
        assert!(!should_skip_control("ControlType_Button"));
        assert!(should_skip_control("ToolBar"));
        assert!(should_skip_control("ControlType_ToolBar"));
    }
}
