//! The only production `Perceiver`. Composes:
//!
//!   1. window enumeration + scoring (top-K)
//!   2. multi-window UIA walk (`UiaPerceiver`)
//!   3. optional desktop/taskbar walk (when no app windows survive)
//!   4. optional vision/OCR (when primary UIA tree is sparse)
//!   5. fusion (`FusionEngine`) — owned exclusively here
//!
//! Never automates anything (PRD §8.9): only screen-space read.

use crate::perception::context::PerceptionContext;
use crate::perception::desktop;
use crate::perception::error::PerceptionError;
use crate::perception::frame::{
    now_ms, PerceptionQuality, PerceptionWarning, ScreenElement, ScreenFrame, WindowId,
    WindowSnapshot,
};
use crate::perception::fusion::FusionEngine;
use crate::perception::uia::{UiaCapture, UiaPerceiver};
use crate::perception::vision::coords::inflate_rect;
use crate::perception::vision::{
    default_vision_perceiver, VisionPerceiver, VisionRequest,
};
use crate::perception::window_enum::list_visible_windows;
use crate::perception::window_score::{rank_windows, visible_count, RankedWindow};
use crate::perception::Perceiver;
use crate::settings::Settings;

pub struct HybridPerceiver {
    uia: UiaPerceiver,
    vision: Box<dyn VisionPerceiver>,
    fusion: FusionEngine,
}

impl Default for HybridPerceiver {
    fn default() -> Self {
        Self::new()
    }
}

impl HybridPerceiver {
    pub fn new() -> Self {
        Self::from_settings(&Settings::from_env())
    }

    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            uia: UiaPerceiver::new(),
            vision: default_vision_perceiver(settings),
            fusion: FusionEngine::new(),
        }
    }

    pub fn with_vision(mut self, vision: Box<dyn VisionPerceiver>) -> Self {
        self.vision = vision;
        self
    }
}

impl Perceiver for HybridPerceiver {
    fn name(&self) -> &str {
        "hybrid"
    }

    fn capture(&self, ctx: &PerceptionContext) -> Result<ScreenFrame, PerceptionError> {
        let started_ms = now_ms();
        let mode = ctx.perception_mode();

        let raw_windows = list_visible_windows();
        let visible_total = visible_count(&raw_windows);
        let ranked = rank_windows(&raw_windows, ctx);

        if ranked.is_empty() {
            // Fall back to desktop chrome only — perhaps the user is on the
            // desktop with no app windows.
            let desktop_cap = desktop::walk_desktop_chrome();
            let desktop_has_windows = !desktop_cap.windows.is_empty();
            let primary_id = desktop_cap
                .windows
                .first()
                .map(|w| w.id)
                .unwrap_or_default();
            let mut frame_warnings = desktop_cap.warnings;
            if !desktop_has_windows {
                frame_warnings.push(PerceptionWarning::SecureDesktop);
            }
            let frame = ScreenFrame {
                captured_at_ms: started_ms,
                cursor: ctx.cursor,
                primary_window_id: primary_id,
                windows: desktop_cap.windows,
                elements: desktop_cap.elements,
                quality: if desktop_has_windows {
                    PerceptionQuality::Full
                } else {
                    PerceptionQuality::DegradedUiaOnly
                },
                warnings: frame_warnings,
            };
            return Ok(frame);
        }

        let primary_id = ranked[0].meta.id;

        let mut warnings = Vec::<PerceptionWarning>::new();
        if visible_total > ranked.len() as u32 {
            warnings.push(PerceptionWarning::WindowCapTruncated {
                total_visible: visible_total,
                scanned: ranked.len() as u32,
            });
        }

        // UIA pass (may be skipped in vision-only mode)
        let mut uia_capture = if mode.uia_enabled() {
            self.uia.capture(&ranked)?
        } else {
            UiaCapture::default()
        };
        warnings.append(&mut uia_capture.warnings);

        // Make sure ranked windows always show up as `WindowSnapshot`s
        // even if UIA failed for them (so the LLM gets the title).
        ensure_window_snapshots(&mut uia_capture.windows, &ranked);

        // Count interactable elements *inside the primary window only* —
        // this drives the OCR gate (`ROOTA_MIN_UIA_ELEMENTS`).
        let interactable_in_primary = count_interactable_in_primary(
            &uia_capture.elements,
            primary_id,
        );

        let primary_rect = uia_capture
            .windows
            .iter()
            .find(|w| w.id == primary_id)
            .map(|w| w.bounds)
            .unwrap_or_default();

        // Maybe run vision (OCR).
        let mut ocr_elements: Vec<ScreenElement> = Vec::new();
        let mut vision_contributed = false;
        let should_run_vision = mode.vision_enabled()
            && ctx.vision_enabled()
            && self.vision.is_available()
            && (interactable_in_primary < ctx.min_uia_elements()
                || matches!(mode, crate::perception::context::PerceptionMode::VisionOnly));

        if should_run_vision {
            let capture_rect = if primary_rect.width > 0 && primary_rect.height > 0 {
                inflate_rect(primary_rect, ctx.settings.capture_margin_px, None)
            } else {
                primary_rect
            };
            let req = VisionRequest {
                primary_window_id: primary_id,
                primary_window_rect: capture_rect,
                language: ctx.settings.ocr_language.clone(),
                scale: ctx.settings.ocr_capture_scale,
                max_edge: ctx.settings.ocr_max_edge,
                preprocess_ocr: ctx.settings.ocr_preprocess,
            };
            match self.vision.recognize(&req) {
                Ok(mut cap) => {
                    if !cap.elements.is_empty() {
                        vision_contributed = true;
                        ocr_elements = cap.elements;
                    }
                    warnings.append(&mut cap.warnings);
                }
                Err(err) => {
                    tracing::warn!(
                        target: "roota.perception.hybrid",
                        "vision recognize failed: {err}"
                    );
                    warnings.push(PerceptionWarning::OcrUnavailable);
                }
            }
        }

        // Fusion — the ONLY place uia + ocr meet.
        let merged = self.fusion.fuse(uia_capture.elements, ocr_elements);

        // Low element count warning (post-fusion, per spec).
        if count_interactable_in_primary(&merged, primary_id) == 0 {
            warnings.push(PerceptionWarning::LowElementCount {
                window_id: primary_id,
                count: 0,
            });
        }

        let quality = compute_quality(
            count_interactable_in_primary(&merged, primary_id),
            ctx.min_uia_elements(),
            vision_contributed,
        );

        let frame = ScreenFrame {
            captured_at_ms: started_ms,
            cursor: ctx.cursor,
            primary_window_id: primary_id,
            windows: uia_capture.windows,
            elements: merged,
            quality,
            warnings,
        };

        tracing::info!(
            target: "roota.perception",
            windows = frame.windows.len(),
            elements = frame.elements.len(),
            primary = %frame.primary_window_title(),
            quality = quality.label(),
            warnings = ?frame.warnings_summary(),
            mode = mode.label(),
            "captured ScreenFrame"
        );

        Ok(frame)
    }
}

fn ensure_window_snapshots(snapshots: &mut Vec<WindowSnapshot>, ranked: &[RankedWindow]) {
    for r in ranked {
        if !snapshots.iter().any(|w| w.id == r.meta.id) {
            snapshots.push(WindowSnapshot {
                id: r.meta.id,
                title: r.meta.title.clone(),
                class_name: r.meta.class_name.clone(),
                bounds: r.meta.bounds,
                is_foreground: r.meta.is_foreground,
                z_order: r.meta.z_order,
                uia_element_count: 0,
            });
        }
    }
}

fn count_interactable_in_primary(elements: &[ScreenElement], primary_id: WindowId) -> usize {
    elements
        .iter()
        .filter(|e| e.window_id == primary_id)
        .filter(|e| is_interactable(&e.kind))
        .count()
}

fn is_interactable(kind: &str) -> bool {
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

fn compute_quality(
    primary_interactable: usize,
    min_required: usize,
    vision_contributed: bool,
) -> PerceptionQuality {
    if vision_contributed {
        PerceptionQuality::VisionAssisted
    } else if primary_interactable >= min_required.max(1) {
        PerceptionQuality::Full
    } else {
        PerceptionQuality::DegradedUiaOnly
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perception::frame::ElementSource;
    use crate::perception::frame::Rect as PRect;

    #[test]
    fn compute_quality_picks_full_when_enough_uia() {
        assert_eq!(compute_quality(5, 3, false), PerceptionQuality::Full);
    }

    #[test]
    fn compute_quality_degraded_when_sparse_no_vision() {
        assert_eq!(
            compute_quality(1, 3, false),
            PerceptionQuality::DegradedUiaOnly
        );
    }

    #[test]
    fn compute_quality_vision_overrides_degradation() {
        assert_eq!(
            compute_quality(0, 3, true),
            PerceptionQuality::VisionAssisted
        );
    }

    #[test]
    fn count_interactable_filters_by_window_and_kind() {
        let primary = WindowId(7);
        let elements = vec![
            ScreenElement {
                source: ElementSource::Uia,
                text: "ok".into(),
                bounds: PRect::new(0, 0, 20, 20),
                window_id: primary,
                kind: "Button".into(),
                confidence: 1.0,
                automation_id: None,
            },
            ScreenElement {
                source: ElementSource::Uia,
                text: "ignored".into(),
                bounds: PRect::new(0, 0, 20, 20),
                window_id: primary,
                kind: "TitleBar".into(),
                confidence: 1.0,
                automation_id: None,
            },
            ScreenElement {
                source: ElementSource::Uia,
                text: "other".into(),
                bounds: PRect::new(0, 0, 20, 20),
                window_id: WindowId(8),
                kind: "Button".into(),
                confidence: 1.0,
                automation_id: None,
            },
        ];
        assert_eq!(count_interactable_in_primary(&elements, primary), 1);
    }

    #[cfg(not(windows))]
    #[test]
    fn hybrid_captures_non_empty_frame_in_stub_environment() {
        let perceiver = HybridPerceiver::new();
        let ctx = PerceptionContext {
            window_hints: vec!["explorador".into()],
            settings: crate::settings::PerceptionSettings::default(),
            ..PerceptionContext::default()
        };
        let frame = perceiver.capture(&ctx).unwrap();
        assert!(!frame.windows.is_empty());
        assert!(frame.primary_window_id.0 != 0);
    }
}
