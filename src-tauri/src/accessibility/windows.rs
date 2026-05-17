//! Read-only Windows UI Automation scanner.

#![cfg(windows)]

use uiautomation::core::UIAutomation;
use uiautomation::types::{ControlType, TreeScope};
use uiautomation::UIElement as UiaElement;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

use crate::accessibility::element::{UiElement, UiSnapshot};
use crate::accessibility::scanner::{ScanContext, Scanner};

const MAX_ELEMENTS: usize = 600;

const SKIP_CONTROL_MARKERS: &[&str] = &[
    "TitleBar",
    "Scrollbar",
    "Thumb",
    "ToolBar",
    "ToolTip",
    "Separator",
];

const ROOTA_TITLE_MARKERS: &[&str] = &["roota"];

#[derive(Debug, Clone, Copy, Default)]
pub struct WindowsScanner;

impl WindowsScanner {
    pub fn new() -> Result<Self, uiautomation::Error> {
        Ok(Self)
    }
}

impl Scanner for WindowsScanner {
    fn name(&self) -> &str {
        "windows"
    }

    fn snapshot_with_context(&self, ctx: &ScanContext) -> UiSnapshot {
        let hints = ctx.window_hints.clone();
        match std::thread::spawn(move || snapshot_inner(&hints)).join() {
            Ok(Ok(snapshot)) => snapshot,
            Ok(Err(err)) => {
                tracing::warn!(target: "roota.accessibility.windows", "scan failed: {err}");
                UiSnapshot::default()
            }
            Err(_) => {
                tracing::warn!(target: "roota.accessibility.windows", "scan thread panicked");
                UiSnapshot::default()
            }
        }
    }
}

fn snapshot_inner(hints: &[String]) -> Result<UiSnapshot, uiautomation::Error> {
    let automation = UIAutomation::new()?;
    let condition = automation.create_true_condition()?;

    let window = match resolve_target_window(&automation, hints) {
        Some(w) => w,
        None => {
            tracing::warn!(target: "roota.accessibility.windows", "no target window resolved");
            return Ok(UiSnapshot::default());
        }
    };

    let window_title = window.get_name().unwrap_or_default();
    let descendants = window.find_all(TreeScope::Descendants, &condition)?;

    let mut elements = Vec::new();
    for node in descendants {
        if elements.len() >= MAX_ELEMENTS {
            break;
        }
        if let Some(el) = to_ui_element(&node, &window_title) {
            elements.push(el);
        }
    }

    let sample: Vec<String> = elements.iter().take(8).map(|e| e.text.clone()).collect();

    tracing::info!(
        target: "roota.accessibility.windows",
        window = %window_title,
        elements = elements.len(),
        sample = ?sample,
        "snapshot"
    );

    Ok(UiSnapshot {
        window: window_title,
        elements,
    })
}

fn resolve_target_window(automation: &UIAutomation, hints: &[String]) -> Option<UiaElement> {
    if !hints.is_empty() {
        if let Some(w) = find_window_matching_hints(automation, hints) {
            return Some(w);
        }
    }

    if let Some(w) = window_from_foreground_hwnd(automation) {
        return Some(w);
    }

    if let Ok(focused) = automation.get_focused_element() {
        if let Some(window) = ascend_to_window(automation, &focused) {
            if !is_roota_window_element(&window) {
                return Some(window);
            }
        }
    }

    find_any_app_window(automation)
}

fn find_window_matching_hints(automation: &UIAutomation, hints: &[String]) -> Option<UiaElement> {
    let root = automation.get_root_element().ok()?;
    let condition = automation.create_true_condition().ok()?;
    let children = root.find_all(TreeScope::Children, &condition).ok()?;

    for child in children {
        if !is_top_level_window(&child) || is_roota_window_element(&child) {
            continue;
        }
        let title = child.get_name().unwrap_or_default().to_lowercase();
        if hints.iter().any(|h| !h.is_empty() && title.contains(h)) {
            tracing::info!(
                target: "roota.accessibility.windows",
                matched = %title,
                "resolved window by hint"
            );
            return Some(child);
        }
    }
    None
}

fn find_any_app_window(automation: &UIAutomation) -> Option<UiaElement> {
    let root = automation.get_root_element().ok()?;
    let condition = automation.create_true_condition().ok()?;
    let children = root.find_all(TreeScope::Children, &condition).ok()?;
    children
        .into_iter()
        .find(|c| is_top_level_window(c) && !is_roota_window_element(c))
}

fn window_from_foreground_hwnd(automation: &UIAutomation) -> Option<UiaElement> {
    let hwnd: HWND = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        return None;
    }
    let el = automation.element_from_handle(hwnd.into()).ok()?;
    if is_top_level_window(&el) && !is_roota_window_element(&el) {
        return Some(el);
    }
    ascend_to_window(automation, &el).filter(|w| !is_roota_window_element(w))
}

fn ascend_to_window(automation: &UIAutomation, start: &UiaElement) -> Option<UiaElement> {
    let walker = automation.create_tree_walker().ok()?;
    let mut current = start.clone();
    for _ in 0..40 {
        if is_top_level_window(&current) {
            return Some(current);
        }
        current = walker.get_parent(&current).ok()?;
    }
    None
}

fn is_top_level_window(element: &UiaElement) -> bool {
    element
        .get_control_type()
        .map(|t| t == ControlType::Window)
        .unwrap_or(false)
}

fn is_roota_window_element(element: &UiaElement) -> bool {
    is_roota_title(&element.get_name().unwrap_or_default())
}

fn is_roota_title(title: &str) -> bool {
    let lower = title.to_lowercase();
    ROOTA_TITLE_MARKERS.iter().any(|m| lower.contains(m))
}

fn is_skipped_control_type(control_type_name: &str) -> bool {
    SKIP_CONTROL_MARKERS
        .iter()
        .any(|m| control_type_name.contains(m))
}

fn element_label(node: &UiaElement) -> String {
    let name = node.get_name().unwrap_or_default();
    if !name.trim().is_empty() {
        return name;
    }
    node.get_help_text()
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_default()
}

fn to_ui_element(node: &UiaElement, window_title: &str) -> Option<UiElement> {
    let control_type = node.get_control_type().ok()?;
    let control_type_name = format!("{control_type:?}");

    if is_skipped_control_type(&control_type_name) {
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

    Some(UiElement {
        kind: control_type_name,
        text,
        x: rect.get_left(),
        y: rect.get_top(),
        width,
        height,
        automation_id,
        window: window_title.to_string(),
    })
}
