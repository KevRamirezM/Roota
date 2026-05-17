//! Unified screen-space perception types — replaces single-window `UiSnapshot`
//! at orchestration boundaries. Wall-clock-friendly (`captured_at_ms: u64`),
//! Serialize/Deserialize for logging and tests.
//!
//! Design spec: `docs/superpowers/specs/2026-05-18-roota-universal-perception-design.md`

use serde::{Deserialize, Serialize};

use crate::input::PhysicalPoint;
use crate::orchestration::state::ActionVerb;

/// Stable identifier for a top-level window. On Windows this is the HWND cast
/// to `u64`; on other platforms it is the synthesized id from a fixture.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WindowId(pub u64);

/// Physical screen-space rectangle.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self { x, y, width, height }
    }

    pub fn from_ltrb(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        }
    }

    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x
            && py >= self.y
            && px <= self.x + self.width
            && py <= self.y + self.height
    }

    pub fn center(&self) -> (i32, i32) {
        (self.x + self.width / 2, self.y + self.height / 2)
    }

    pub fn area(&self) -> i64 {
        i64::from(self.width.max(0)) * i64::from(self.height.max(0))
    }

    /// Intersection-over-Union for fusion overlap tests (Phase 3).
    pub fn iou(&self, other: &Rect) -> f32 {
        let inter_x = self.x.max(other.x);
        let inter_y = self.y.max(other.y);
        let inter_r = (self.x + self.width).min(other.x + other.width);
        let inter_b = (self.y + self.height).min(other.y + other.height);
        if inter_r <= inter_x || inter_b <= inter_y {
            return 0.0;
        }
        let inter = i64::from(inter_r - inter_x) * i64::from(inter_b - inter_y);
        let union = self.area() + other.area() - inter;
        if union <= 0 {
            return 0.0;
        }
        inter as f32 / union as f32
    }
}

/// Where this element came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElementSource {
    Uia,
    Ocr,
    /// Local VLM (e.g. Moondream via Ollama).
    Vlm,
    Fused,
}

/// A single perceived UI element, with screen-space bounds.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenElement {
    pub source: ElementSource,
    pub text: String,
    pub bounds: Rect,
    pub window_id: WindowId,
    pub kind: String,
    /// 1.0 for UIA; OCR/VLM lower.
    pub confidence: f32,
    #[serde(default)]
    pub automation_id: Option<String>,
}

impl ScreenElement {
    pub fn center(&self) -> (i32, i32) {
        self.bounds.center()
    }

    pub fn matches(&self, query: &str) -> bool {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return false;
        }
        if self.text.to_lowercase().contains(&q) {
            return true;
        }
        if let Some(id) = &self.automation_id {
            if id.to_lowercase().contains(&q) {
                return true;
            }
        }
        false
    }
}

/// Per-window metadata included alongside elements.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowSnapshot {
    pub id: WindowId,
    pub title: String,
    #[serde(default)]
    pub class_name: String,
    pub bounds: Rect,
    pub is_foreground: bool,
    pub z_order: u32,
    pub uia_element_count: usize,
}

/// Overall quality signal for the frame. Drives whether the orchestrator
/// shows a "limited perception" sentence to the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PerceptionQuality {
    #[default]
    Full,
    DegradedUiaOnly,
    VisionAssisted,
}

impl PerceptionQuality {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::DegradedUiaOnly => "degraded_uia_only",
            Self::VisionAssisted => "vision_assisted",
        }
    }
}

/// Non-fatal warnings surfaced from any layer (UIA, vision, fusion).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PerceptionWarning {
    SecureDesktop,
    OcrUnavailable,
    ModalAttachFailed { dialog_title: String },
    WindowCapTruncated { total_visible: u32, scanned: u32 },
    LowElementCount { window_id: WindowId, count: usize },
}

impl PerceptionWarning {
    pub fn label(&self) -> &'static str {
        match self {
            Self::SecureDesktop => "secure_desktop",
            Self::OcrUnavailable => "ocr_unavailable",
            Self::ModalAttachFailed { .. } => "modal_attach_failed",
            Self::WindowCapTruncated { .. } => "window_cap_truncated",
            Self::LowElementCount { .. } => "low_element_count",
        }
    }
}

/// One capture cycle — what the orchestrator/decision/detector consume.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenFrame {
    /// Wall-clock millis since UNIX epoch. **Not** `Instant` (see spec
    /// resolved-defaults #4: `Instant` is not serialized nor log-safe).
    pub captured_at_ms: u64,
    pub cursor: PhysicalPoint,
    pub primary_window_id: WindowId,
    pub windows: Vec<WindowSnapshot>,
    pub elements: Vec<ScreenElement>,
    pub quality: PerceptionQuality,
    #[serde(default)]
    pub warnings: Vec<PerceptionWarning>,
}

impl ScreenFrame {
    /// Empty deterministic frame (cursor at 0, no windows). Useful for tests
    /// and as a safe fallback inside fixtures — production code should always
    /// build a real frame from a `Perceiver`.
    pub fn empty() -> Self {
        Self {
            captured_at_ms: 0,
            cursor: PhysicalPoint::default(),
            primary_window_id: WindowId::default(),
            windows: Vec::new(),
            elements: Vec::new(),
            quality: PerceptionQuality::DegradedUiaOnly,
            warnings: Vec::new(),
        }
    }

    pub fn primary_window(&self) -> Option<&WindowSnapshot> {
        self.windows
            .iter()
            .find(|w| w.id == self.primary_window_id)
    }

    pub fn primary_window_title(&self) -> String {
        self.primary_window()
            .map(|w| w.title.clone())
            .unwrap_or_default()
    }

    pub fn window_title(&self, id: WindowId) -> Option<&str> {
        self.windows
            .iter()
            .find(|w| w.id == id)
            .map(|w| w.title.as_str())
    }

    /// First element that matches the query, anywhere in the frame.
    pub fn find(&self, query: &str) -> Option<&ScreenElement> {
        self.elements.iter().find(|e| e.matches(query))
    }

    /// Best clickable target across all perceived windows, with the
    /// primary window getting a tiebreaker bonus. Mirrors the heuristics
    /// from the legacy `UiSnapshot::find_best_for_action`.
    pub fn find_best_for_action(
        &self,
        queries: &[String],
        action: ActionVerb,
    ) -> Option<&ScreenElement> {
        let mut best: Option<(&ScreenElement, i32)> = None;
        for element in &self.elements {
            for query in queries {
                let base = match_score(element, query);
                if base == 0 {
                    continue;
                }
                let mut score = rank_element(element, base, action);
                if element.window_id == self.primary_window_id {
                    score += 8;
                }
                // Confidence scales between 0..1 → 0..15 bonus
                score += (element.confidence.clamp(0.0, 1.0) * 15.0) as i32;
                if best.map(|(_, s)| score > s).unwrap_or(true) {
                    best = Some((element, score));
                }
            }
        }
        best.map(|(e, _)| e)
    }

    /// Compact label list for the LLM prompt (capped to `limit`).
    pub fn visible_summary(&self, limit: usize) -> String {
        let limit = limit.max(1);
        self.elements
            .iter()
            .take(limit)
            .map(format_element_line)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Like `visible_summary`, but ranks by hint match, primary window, interactable
    /// kind, cursor proximity, and confidence — not tree-walk order.
    pub fn ranked_visible_summary(
        &self,
        limit: usize,
        hints: &[String],
        cursor: PhysicalPoint,
    ) -> String {
        let limit = limit.max(1);
        let mut scored: Vec<(i32, &ScreenElement)> = self
            .elements
            .iter()
            .map(|e| (rank_for_prompt(e, hints, cursor, self.primary_window_id), e))
            .collect();
        scored.sort_by_key(|b| std::cmp::Reverse(b.0));
        scored
            .into_iter()
            .take(limit)
            .map(|(_, e)| format_element_line(e))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// True when any element came from the local VLM layer.
    pub fn has_vlm_elements(&self) -> bool {
        self.elements.iter().any(|e| e.source == ElementSource::Vlm)
    }

    /// One-line-per-window list for the LLM prompt (capped to `limit`).
    pub fn window_list_for_prompt(&self, limit: usize) -> String {
        let limit = limit.max(1);
        let mut entries: Vec<&WindowSnapshot> = self.windows.iter().collect();
        // Primary window first, then by descending element count.
        entries.sort_by_key(|w| {
            (
                if w.id == self.primary_window_id { 0 } else { 1 },
                std::cmp::Reverse(w.uia_element_count),
            )
        });
        entries
            .into_iter()
            .take(limit)
            .map(|w| {
                let marker = if w.id == self.primary_window_id {
                    "*"
                } else {
                    " "
                };
                format!(
                    "{marker} {title} [{kind}] ({count} elementos)",
                    title = w.title,
                    kind = if w.class_name.is_empty() {
                        "—"
                    } else {
                        w.class_name.as_str()
                    },
                    count = w.uia_element_count,
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Short, log-safe description of any warnings (for prompt + tracing).
    pub fn warnings_summary(&self) -> String {
        if self.warnings.is_empty() {
            return String::new();
        }
        let parts: Vec<String> = self
            .warnings
            .iter()
            .map(|w| match w {
                PerceptionWarning::ModalAttachFailed { dialog_title } => {
                    format!("modal_attach_failed:{dialog_title}")
                }
                PerceptionWarning::WindowCapTruncated {
                    total_visible,
                    scanned,
                } => format!("window_cap:{scanned}/{total_visible}"),
                PerceptionWarning::LowElementCount { count, .. } => {
                    format!("low_elements:{count}")
                }
                other => other.label().to_string(),
            })
            .collect();
        parts.join(", ")
    }

    /// Total interactable element count inside the primary window's client
    /// rect — drives the OCR gate (`ROOTA_MIN_UIA_ELEMENTS`).
    pub fn primary_interactable_count(&self) -> usize {
        self.elements
            .iter()
            .filter(|e| e.window_id == self.primary_window_id)
            .filter(|e| is_interactable_kind(&e.kind))
            .count()
    }
}

/// Wall-clock helper: millis since UNIX epoch, 0 if the system clock is broken.
pub fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn format_element_line(e: &ScreenElement) -> String {
    let src = match e.source {
        ElementSource::Uia => "uia",
        ElementSource::Ocr => "ocr",
        ElementSource::Vlm => "vlm",
        ElementSource::Fused => "fused",
    };
    let aid = e
        .automation_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|id| format!(" id={id}"))
        .unwrap_or_default();
    let (cx, cy) = e.center();
    format!(
        "- {} ({}) [{src}]{aid} @({cx},{cy})",
        e.text, e.kind
    )
}

fn rank_for_prompt(
    element: &ScreenElement,
    hints: &[String],
    cursor: PhysicalPoint,
    primary_id: WindowId,
) -> i32 {
    let mut score = 0i32;
    for hint in hints {
        score = score.max(match_score(element, hint));
    }
    if element.window_id == primary_id {
        score += 8;
    }
    if is_interactable_kind(&element.kind) {
        score += 10;
    }
    let (cx, cy) = element.center();
    let dx = (cursor.x - cx).abs();
    let dy = (cursor.y - cy).abs();
    let dist = dx.saturating_add(dy);
    score += (500i32.saturating_sub(dist.min(500))) / 10;
    score += (element.confidence.clamp(0.0, 1.0) * 15.0) as i32;
    score
}

fn is_interactable_kind(kind: &str) -> bool {
    let k = kind.to_lowercase();
    k.contains("button")
        || k.contains("hyperlink")
        || k.contains("treeitem")
        || k.contains("listitem")
        || k.contains("menuitem")
        || k.contains("tabitem")
        || k.contains("edit")
        || k.contains("text")
        || k.contains("checkbox")
        || k.contains("radiobutton")
}

fn match_score(element: &ScreenElement, query: &str) -> i32 {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return 0;
    }
    let text = element.text.to_lowercase();
    if text == q {
        return 100;
    }
    if text.starts_with(&q) {
        return 85;
    }
    if text.contains(&q) {
        return 70;
    }
    if let Some(id) = &element.automation_id {
        let id = id.to_lowercase();
        if id == q {
            return 90;
        }
        if id.contains(&q) {
            return 65;
        }
    }
    0
}

fn rank_element(element: &ScreenElement, base: i32, action: ActionVerb) -> i32 {
    let mut score = base;
    let kind = element.kind.to_lowercase();
    let text = element.text.to_lowercase();

    if kind.contains("treeitem") || kind.contains("listitem") {
        score += 30;
    } else if kind.contains("button") || kind.contains("hyperlink") {
        score += 12;
    }

    if text.contains("barra de estado") || text.contains("status bar") || kind.contains("status") {
        score -= 50;
    }
    if text.contains("campo propiedades") || text.contains("modos de vista") {
        score -= 35;
    }
    if text.contains("controlar host") || text.contains("vertical") && text.len() < 12 {
        score -= 25;
    }

    if element.bounds.x < 380
        && element.bounds.width < 320
        && element.bounds.height >= 18
        && element.bounds.height <= 48
    {
        score += 20;
    }

    if matches!(action, ActionVerb::DoubleClick | ActionVerb::Click) && element.bounds.height > 64 {
        score -= 15;
    }

    score
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen_el(
        text: &str,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        window_id: u64,
    ) -> ScreenElement {
        ScreenElement {
            source: ElementSource::Uia,
            text: text.into(),
            bounds: Rect::new(x, y, w, h),
            window_id: WindowId(window_id),
            kind: "Button".into(),
            confidence: 1.0,
            automation_id: Some(text.to_lowercase()),
        }
    }

    fn tree_item(text: &str, x: i32, y: i32, window_id: u64) -> ScreenElement {
        ScreenElement {
            source: ElementSource::Uia,
            text: text.into(),
            bounds: Rect::new(x, y, 200, 32),
            window_id: WindowId(window_id),
            kind: "TreeItem".into(),
            confidence: 1.0,
            automation_id: Some(text.to_lowercase()),
        }
    }

    fn window_snap(id: u64, title: &str, count: usize) -> WindowSnapshot {
        WindowSnapshot {
            id: WindowId(id),
            title: title.into(),
            class_name: String::new(),
            bounds: Rect::new(0, 0, 800, 600),
            is_foreground: false,
            z_order: 0,
            uia_element_count: count,
        }
    }

    #[test]
    fn rect_contains_inclusive_on_edges() {
        let r = Rect::new(10, 20, 30, 40);
        assert!(r.contains(10, 20));
        assert!(r.contains(40, 60));
        assert!(!r.contains(9, 20));
        assert!(!r.contains(10, 19));
    }

    #[test]
    fn rect_iou_returns_zero_for_disjoint() {
        let a = Rect::new(0, 0, 10, 10);
        let b = Rect::new(100, 100, 10, 10);
        assert_eq!(a.iou(&b), 0.0);
    }

    #[test]
    fn rect_iou_overlap_above_half() {
        let a = Rect::new(0, 0, 10, 10);
        let b = Rect::new(2, 2, 10, 10);
        assert!(a.iou(&b) > 0.4);
    }

    #[test]
    fn find_best_prefers_exact_text_across_windows() {
        let frame = ScreenFrame {
            elements: vec![
                screen_el("Descargas", 100, 100, 80, 24, 1),
                screen_el("Documentos", 200, 100, 80, 24, 2),
            ],
            ..ScreenFrame::empty()
        };
        let q = vec!["descargas".into()];
        let found = frame.find_best_for_action(&q, ActionVerb::Click);
        assert_eq!(found.unwrap().text, "Descargas");
    }

    #[test]
    fn find_best_uses_primary_window_tiebreaker() {
        // Two identical "Descargas" buttons in the centre of two apps — the
        // primary one should win on the tiebreaker. Both x values are outside
        // the sidebar sweet spot so neither element gets the sidebar bonus.
        let mut frame = ScreenFrame {
            windows: vec![window_snap(1, "Primary", 5), window_snap(2, "Other", 5)],
            elements: vec![
                screen_el("Descargas", 500, 100, 80, 24, 2),
                screen_el("Descargas", 600, 100, 80, 24, 1),
            ],
            ..ScreenFrame::empty()
        };
        frame.primary_window_id = WindowId(1);
        let q = vec!["descargas".into()];
        let found = frame.find_best_for_action(&q, ActionVerb::Click).unwrap();
        assert_eq!(found.window_id, WindowId(1));
    }

    #[test]
    fn find_best_prefers_treeitem_in_sidebar() {
        let frame = ScreenFrame {
            elements: vec![
                screen_el("Descargas", 400, 0, 800, 40, 1),
                tree_item("Descargas", 80, 220, 1),
            ],
            ..ScreenFrame::empty()
        };
        let found = frame
            .find_best_for_action(&["descargas".into()], ActionVerb::DoubleClick)
            .unwrap();
        assert_eq!(found.kind, "TreeItem");
        assert_eq!(found.bounds.x, 80);
    }

    #[test]
    fn visible_summary_caps_lines() {
        let mut elements = Vec::new();
        for i in 0..10 {
            elements.push(screen_el(&format!("el{i}"), 0, i * 10, 50, 20, 1));
        }
        let frame = ScreenFrame {
            elements,
            ..ScreenFrame::empty()
        };
        let summary = frame.visible_summary(3);
        assert_eq!(summary.lines().count(), 3);
    }

    #[test]
    fn ranked_visible_summary_prefers_hint_match() {
        let mut elements = Vec::new();
        for i in 0..50 {
            elements.push(screen_el(&format!("el{i}"), 0, i * 10, 50, 20, 1));
        }
        elements.push(screen_el("Descargas", 400, 400, 80, 24, 1));
        let frame = ScreenFrame {
            elements,
            ..ScreenFrame::empty()
        };
        let summary = frame.ranked_visible_summary(
            5,
            &["descargas".into()],
            PhysicalPoint { x: 420, y: 410 },
        );
        assert!(summary.contains("Descargas"));
    }

    #[test]
    fn window_list_marks_primary_and_caps() {
        let mut frame = ScreenFrame {
            windows: vec![
                window_snap(1, "Primary App", 12),
                window_snap(2, "Background", 5),
                window_snap(3, "Sidebar", 3),
                window_snap(4, "Other", 1),
            ],
            ..ScreenFrame::empty()
        };
        frame.primary_window_id = WindowId(1);
        let listing = frame.window_list_for_prompt(3);
        assert_eq!(listing.lines().count(), 3);
        let first = listing.lines().next().unwrap();
        assert!(first.starts_with("*"));
        assert!(first.contains("Primary App"));
    }

    #[test]
    fn warnings_summary_joins_labels() {
        let frame = ScreenFrame {
            warnings: vec![
                PerceptionWarning::SecureDesktop,
                PerceptionWarning::WindowCapTruncated {
                    total_visible: 12,
                    scanned: 8,
                },
            ],
            ..ScreenFrame::empty()
        };
        let s = frame.warnings_summary();
        assert!(s.contains("secure_desktop"));
        assert!(s.contains("window_cap:8/12"));
    }

    #[test]
    fn primary_interactable_count_filters_by_window() {
        let mut frame = ScreenFrame {
            elements: vec![
                screen_el("a", 0, 0, 20, 20, 1),
                screen_el("b", 0, 0, 20, 20, 1),
                screen_el("c", 0, 0, 20, 20, 2),
                ScreenElement {
                    source: ElementSource::Uia,
                    text: "title".into(),
                    bounds: Rect::new(0, 0, 20, 20),
                    window_id: WindowId(1),
                    kind: "TitleBar".into(),
                    confidence: 1.0,
                    automation_id: None,
                },
            ],
            ..ScreenFrame::empty()
        };
        frame.primary_window_id = WindowId(1);
        assert_eq!(frame.primary_interactable_count(), 2);
    }
}
