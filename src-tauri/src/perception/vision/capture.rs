//! Bitmap capture for a single window rectangle.
//!
//! v1: stub only — returns an empty buffer. A real implementation will use
//! `xcap` (or `PrintWindow`/`BitBlt` fallback) once the dependency spike
//! (plan Task 11) is signed off. The function signature is stable so
//! downstream code does not change.

use crate::perception::frame::Rect;

#[derive(Debug, Clone)]
pub struct CapturedFrame {
    /// Logical pixel buffer dimensions. Zero when no real engine is wired.
    pub width: u32,
    pub height: u32,
    /// RGBA8 little-endian (empty until the real engine lands).
    pub pixels: Vec<u8>,
    /// Physical screen rect we captured (mirrors input for callers).
    pub source_rect: Rect,
}

impl CapturedFrame {
    pub fn empty(rect: Rect) -> Self {
        Self {
            width: 0,
            height: 0,
            pixels: Vec::new(),
            source_rect: rect,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.pixels.is_empty()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error("screen capture not implemented in this build")]
    Unimplemented,
    #[error("capture failed: {0}")]
    Platform(String),
}

/// Stub capture — always returns an empty buffer. Keeps the OCR layer
/// callable without adding heavyweight deps.
pub fn capture_window_bitmap(rect: Rect, _scale: f32) -> Result<CapturedFrame, CaptureError> {
    tracing::debug!(
        target: "roota.perception.vision",
        rect = ?rect,
        "capture_window_bitmap (stub) — install xcap/BitBlt backend in plan Task 11"
    );
    Ok(CapturedFrame::empty(rect))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_capture_returns_empty_frame() {
        let r = Rect::new(0, 0, 800, 600);
        let frame = capture_window_bitmap(r, 0.75).unwrap();
        assert!(frame.is_empty());
        assert_eq!(frame.source_rect, r);
    }
}
