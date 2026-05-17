//! Merge UIA + OCR element layers into a single ranked list.
//!
//! Owned exclusively by `HybridPerceiver` (per design spec): keeping fusion
//! rules in one place avoids double-merge when OCR is disabled.

use crate::perception::frame::{ElementSource, ScreenElement};

/// IoU threshold above which a UIA box "owns" an overlapping OCR text and
/// the two are fused into a single `ElementSource::Fused` entry.
pub const FUSION_IOU_THRESHOLD: f32 = 0.5;

#[derive(Debug, Default)]
pub struct FusionEngine;

impl FusionEngine {
    pub fn new() -> Self {
        Self
    }

    /// Returns a new vector with UIA + OCR elements merged.
    ///
    /// Rules (design spec §Vision layer):
    ///   - UIA boxes with IoU > 0.5 against an OCR line → merge (Fused),
    ///     keep UIA bounds + kind, prefer OCR text if it is richer.
    ///   - OCR lines with no UIA overlap → keep as `Ocr` candidates.
    ///   - UIA lines with no OCR overlap → keep unchanged.
    pub fn fuse(&self, uia: Vec<ScreenElement>, ocr: Vec<ScreenElement>) -> Vec<ScreenElement> {
        if ocr.is_empty() {
            return uia;
        }
        if uia.is_empty() {
            return ocr;
        }

        let mut merged: Vec<ScreenElement> = Vec::with_capacity(uia.len() + ocr.len());
        let mut ocr_used = vec![false; ocr.len()];

        for u in uia.into_iter() {
            let mut best: Option<(usize, f32)> = None;
            for (idx, o) in ocr.iter().enumerate() {
                if ocr_used[idx] {
                    continue;
                }
                if u.window_id != o.window_id {
                    continue;
                }
                let iou = u.bounds.iou(&o.bounds);
                if iou > FUSION_IOU_THRESHOLD
                    && best.map(|(_, s)| iou > s).unwrap_or(true)
                {
                    best = Some((idx, iou));
                }
            }

            if let Some((idx, _iou)) = best {
                ocr_used[idx] = true;
                let o = &ocr[idx];
                merged.push(ScreenElement {
                    source: ElementSource::Fused,
                    text: pick_richer_text(&u.text, &o.text),
                    bounds: u.bounds,
                    window_id: u.window_id,
                    kind: u.kind.clone(),
                    confidence: u.confidence.max(o.confidence),
                    automation_id: u.automation_id.clone(),
                });
            } else {
                merged.push(u);
            }
        }

        for (idx, o) in ocr.into_iter().enumerate() {
            if !ocr_used[idx] {
                merged.push(o);
            }
        }

        merged
    }
}

fn pick_richer_text(uia_text: &str, ocr_text: &str) -> String {
    let u = uia_text.trim();
    let o = ocr_text.trim();
    if u.is_empty() {
        return o.to_string();
    }
    if o.is_empty() {
        return u.to_string();
    }
    // UIA labels are normally cleaner; only swap if OCR text is dramatically
    // longer (e.g. a button with no `Name` whose label is rendered as a glyph).
    if o.len() > u.len() * 2 {
        o.to_string()
    } else {
        u.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perception::frame::{Rect, WindowId};

    fn el(source: ElementSource, text: &str, x: i32, y: i32, w: i32, h: i32, window: u64) -> ScreenElement {
        ScreenElement {
            source,
            text: text.into(),
            bounds: Rect::new(x, y, w, h),
            window_id: WindowId(window),
            kind: "Button".into(),
            confidence: match source {
                ElementSource::Uia => 1.0,
                _ => 0.7,
            },
            automation_id: None,
        }
    }

    #[test]
    fn passthrough_when_one_layer_empty() {
        let fusion = FusionEngine::new();
        let uia = vec![el(ElementSource::Uia, "A", 0, 0, 10, 10, 1)];
        let out = fusion.fuse(uia.clone(), Vec::new());
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].text, "A");
    }

    #[test]
    fn overlap_merges_into_fused() {
        let fusion = FusionEngine::new();
        let uia = vec![el(ElementSource::Uia, "Btn", 0, 0, 100, 50, 1)];
        let ocr = vec![el(ElementSource::Ocr, "Btn Label", 5, 5, 100, 50, 1)];
        let out = fusion.fuse(uia, ocr);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].source, ElementSource::Fused);
    }

    #[test]
    fn disjoint_keeps_both_layers() {
        let fusion = FusionEngine::new();
        let uia = vec![el(ElementSource::Uia, "A", 0, 0, 50, 30, 1)];
        let ocr = vec![el(ElementSource::Ocr, "B", 500, 500, 50, 30, 1)];
        let out = fusion.fuse(uia, ocr);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn vlm_overlaps_uia_like_ocr() {
        let fusion = FusionEngine::new();
        let uia = vec![el(ElementSource::Uia, "Btn", 0, 0, 100, 50, 1)];
        let vlm = vec![el(ElementSource::Vlm, "Btn Label", 5, 5, 100, 50, 1)];
        let out = fusion.fuse(uia, vlm);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].source, ElementSource::Fused);
    }

    #[test]
    fn different_windows_never_merge() {
        let fusion = FusionEngine::new();
        let uia = vec![el(ElementSource::Uia, "X", 0, 0, 100, 100, 1)];
        let ocr = vec![el(ElementSource::Ocr, "X", 0, 0, 100, 100, 2)];
        let out = fusion.fuse(uia, ocr);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|e| e.source != ElementSource::Fused));
    }
}
