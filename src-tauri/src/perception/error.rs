//! Fallible `Perceiver::capture` returns one of these. The orchestrator
//! converts them into user-facing i18n strings.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PerceptionError {
    #[error("UI automation failed: {0}")]
    Uia(String),
    #[error("screen capture failed: {0}")]
    Capture(String),
    #[error("OCR engine failed: {0}")]
    Ocr(String),
    #[error("no visible windows to perceive")]
    NoWindows,
    #[error("secure desktop active — cannot read the screen")]
    SecureDesktop,
    #[error("perception thread panicked")]
    ThreadJoin,
    #[error("perception aborted")]
    Cancelled,
}

impl PerceptionError {
    /// i18n key the orchestrator can pass through `i18n::t`.
    pub fn i18n_key(&self) -> &'static str {
        match self {
            Self::SecureDesktop => "guidance.secure_desktop_blocked",
            _ => "guidance.perception_failed",
        }
    }
}
