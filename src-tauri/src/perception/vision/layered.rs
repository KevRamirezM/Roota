//! OCR-first vision stack with optional VLM fallback when OCR is sparse.

use crate::perception::error::PerceptionError;
use crate::perception::frame::PerceptionWarning;
use crate::perception::vision::{VisionCapture, VisionPerceiver, VisionRequest};

/// Minimum OCR elements before skipping the VLM fallback.
const MIN_OCR_ELEMENTS_FOR_SKIP_VLM: usize = 2;

pub struct LayeredVisionPerceiver {
    primary: Box<dyn VisionPerceiver>,
    fallback: Option<Box<dyn VisionPerceiver>>,
}

impl LayeredVisionPerceiver {
    pub fn new(primary: Box<dyn VisionPerceiver>, fallback: Option<Box<dyn VisionPerceiver>>) -> Self {
        Self { primary, fallback }
    }
}

impl VisionPerceiver for LayeredVisionPerceiver {
    fn name(&self) -> &str {
        if self.fallback.is_some() {
            "layered-ocr-vlm"
        } else {
            self.primary.name()
        }
    }

    fn is_available(&self) -> bool {
        self.primary.is_available()
            || self
                .fallback
                .as_ref()
                .is_some_and(|f| f.is_available())
    }

    fn recognize(&self, req: &VisionRequest) -> Result<VisionCapture, PerceptionError> {
        let mut cap = match self.primary.recognize(req) {
            Ok(c) => c,
            Err(err) => {
                if let Some(fb) = &self.fallback {
                    if fb.is_available() {
                        tracing::warn!(
                            target: "roota.perception.vision",
                            primary = self.primary.name(),
                            "primary vision failed ({err}); trying VLM fallback"
                        );
                        return fb.recognize(req);
                    }
                }
                return Err(err);
            }
        };

        if cap.elements.len() >= MIN_OCR_ELEMENTS_FOR_SKIP_VLM {
            tracing::debug!(
                target: "roota.perception.vision",
                lines = cap.elements.len(),
                "OCR sufficient; skipping VLM"
            );
            return Ok(cap);
        }

        let Some(fb) = &self.fallback else {
            return Ok(cap);
        };

        if !fb.is_available() {
            return Ok(cap);
        }

        tracing::info!(
            target: "roota.perception.vision",
            ocr_lines = cap.elements.len(),
            "sparse OCR; trying VLM fallback"
        );

        match fb.recognize(req) {
            Ok(vlm) => {
                if vlm.elements.len() > cap.elements.len() {
                    cap.elements = vlm.elements;
                    cap.warnings = vlm.warnings;
                } else {
                    cap.elements.extend(vlm.elements);
                    cap.warnings.extend(vlm.warnings);
                }
            }
            Err(err) => {
                tracing::warn!(
                    target: "roota.perception.vision",
                    "VLM fallback failed: {err}"
                );
                if cap.elements.is_empty() {
                    cap.warnings.push(PerceptionWarning::OcrUnavailable);
                }
            }
        }

        Ok(cap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perception::frame::{ElementSource, Rect, ScreenElement, WindowId};

    struct StubVision {
        name: &'static str,
        available: bool,
        elements: Vec<ScreenElement>,
        fail: bool,
    }

    impl VisionPerceiver for StubVision {
        fn name(&self) -> &str {
            self.name
        }

        fn is_available(&self) -> bool {
            self.available
        }

        fn recognize(&self, _req: &VisionRequest) -> Result<VisionCapture, PerceptionError> {
            if self.fail {
                return Err(PerceptionError::Ocr("stub fail".into()));
            }
            Ok(VisionCapture {
                elements: self.elements.clone(),
                warnings: Vec::new(),
            })
        }
    }

    fn el(text: &str) -> ScreenElement {
        ScreenElement {
            source: ElementSource::Ocr,
            text: text.into(),
            bounds: Rect::new(0, 0, 40, 20),
            window_id: WindowId(1),
            kind: "text".into(),
            confidence: 0.85,
            automation_id: None,
        }
    }

    #[test]
    fn skips_vlm_when_ocr_has_enough_lines() {
        let primary = Box::new(StubVision {
            name: "ocr",
            available: true,
            elements: vec![el("a"), el("b")],
            fail: false,
        });
        let fallback = Box::new(StubVision {
            name: "vlm",
            available: true,
            elements: vec![el("vlm-only")],
            fail: false,
        });
        let layered = LayeredVisionPerceiver::new(primary, Some(fallback));
        let cap = layered.recognize(&VisionRequest {
            primary_window_id: WindowId(1),
            primary_window_rect: Rect::new(0, 0, 800, 600),
            language: "es".into(),
            scale: 0.75,
        })
        .unwrap();
        assert_eq!(cap.elements.len(), 2);
        assert_eq!(cap.elements[0].text, "a");
    }

    #[test]
    fn uses_vlm_when_ocr_sparse() {
        let primary = Box::new(StubVision {
            name: "ocr",
            available: true,
            elements: vec![el("only")],
            fail: false,
        });
        let fallback = Box::new(StubVision {
            name: "vlm",
            available: true,
            elements: vec![el("vlm-a"), el("vlm-b")],
            fail: false,
        });
        let layered = LayeredVisionPerceiver::new(primary, Some(fallback));
        let cap = layered
            .recognize(&VisionRequest {
                primary_window_id: WindowId(1),
                primary_window_rect: Rect::new(0, 0, 800, 600),
                language: "es".into(),
                scale: 0.75,
            })
            .unwrap();
        assert_eq!(cap.elements.len(), 2);
        assert_eq!(cap.elements[0].text, "vlm-a");
    }
}
