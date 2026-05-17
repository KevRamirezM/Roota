//! Windows.Media.Ocr adapter — stub until plan Task 12 finishes the
//! WinRT vs `ocrs` spike. The struct is wired through `VisionPerceiver`
//! so swapping in a real engine is a single-file change.

use crate::perception::error::PerceptionError;
use crate::perception::frame::PerceptionWarning;
use crate::perception::vision::{VisionCapture, VisionPerceiver, VisionRequest};

#[derive(Debug, Default)]
pub struct WindowsOcrPerceiver {
    available: bool,
}

impl WindowsOcrPerceiver {
    pub fn new() -> Self {
        Self { available: false }
    }
}

impl VisionPerceiver for WindowsOcrPerceiver {
    fn name(&self) -> &str {
        "windows-ocr"
    }

    fn is_available(&self) -> bool {
        self.available
    }

    fn recognize(&self, _req: &VisionRequest) -> Result<VisionCapture, PerceptionError> {
        Ok(VisionCapture {
            elements: Vec::new(),
            warnings: vec![PerceptionWarning::OcrUnavailable],
        })
    }
}
