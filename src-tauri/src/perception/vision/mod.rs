//! Optional pixel-side perception (capture + OCR). v1 only ships the trait
//! and a no-op implementation — `xcap` and the OCR engine are added behind
//! a 30-min spike in Task 12 (see plan).
//!
//! The architecture is wired end-to-end so adding a real engine is a single
//! file change: implement `VisionPerceiver` and pass it to `HybridPerceiver`.

pub mod capture;
pub mod layered;
pub mod moondream;
pub mod ocr_windows;

use crate::perception::error::PerceptionError;
use crate::perception::frame::{
    PerceptionWarning, Rect, ScreenElement, WindowId,
};

/// Per-call inputs for vision: which window to OCR, in physical screen coords.
#[derive(Debug, Clone)]
pub struct VisionRequest {
    pub primary_window_id: WindowId,
    pub primary_window_rect: Rect,
    pub language: String,
    pub scale: f32,
}

#[derive(Debug, Default)]
pub struct VisionCapture {
    pub elements: Vec<ScreenElement>,
    pub warnings: Vec<PerceptionWarning>,
}

pub trait VisionPerceiver: Send + Sync {
    fn name(&self) -> &str;
    fn recognize(&self, req: &VisionRequest) -> Result<VisionCapture, PerceptionError>;
    /// True when the engine is actually wired (real OCR loaded). The
    /// orchestrator surfaces `PerceptionWarning::OcrUnavailable` otherwise.
    fn is_available(&self) -> bool {
        false
    }
}

/// Default no-op vision perceiver — returns no OCR lines. Replace with
/// `WindowsOcrPerceiver` once the OCR spike (plan Task 12) lands.
#[derive(Debug, Default)]
pub struct NoopVisionPerceiver;

impl VisionPerceiver for NoopVisionPerceiver {
    fn name(&self) -> &str {
        "noop"
    }

    fn recognize(&self, _req: &VisionRequest) -> Result<VisionCapture, PerceptionError> {
        Ok(VisionCapture {
            elements: Vec::new(),
            warnings: vec![PerceptionWarning::OcrUnavailable],
        })
    }

    fn is_available(&self) -> bool {
        false
    }
}

/// Build the platform default vision perceiver (Windows OCR; optional Moondream VLM).
pub fn default_vision_perceiver(settings: &crate::settings::Settings) -> Box<dyn VisionPerceiver> {
    if !settings.perception.vision_enabled {
        return Box::new(NoopVisionPerceiver);
    }

    #[cfg(windows)]
    {
        let ocr: Box<dyn VisionPerceiver> =
            Box::new(ocr_windows::WindowsOcrPerceiver::new(&settings.perception));
        if settings.perception.vision_vlm_enabled {
            let vlm: Box<dyn VisionPerceiver> =
                Box::new(moondream::MoondreamVisionPerceiver::new(settings));
            return Box::new(layered::LayeredVisionPerceiver::new(ocr, Some(vlm)));
        }
        return ocr;
    }

    #[cfg(not(windows))]
    {
        if settings.perception.vision_vlm_enabled {
            Box::new(moondream::MoondreamVisionPerceiver::new(settings))
        } else {
            Box::new(NoopVisionPerceiver)
        }
    }
}
