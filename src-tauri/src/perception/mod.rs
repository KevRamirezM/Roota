//! Universal Windows perception — multi-window UIA + optional vision.
//!
//! See `docs/superpowers/specs/2026-05-18-roota-universal-perception-design.md`.
//!
//! `HybridPerceiver` is the only production `Perceiver`. It owns
//! `FusionEngine` and composes `UiaPerceiver` + optional `VisionPerceiver`.
//! Tests use `StubPerceiver`.

pub mod cache;
pub mod context;
pub mod desktop;
pub mod error;
pub mod frame;
pub mod fusion;
pub mod hybrid;
pub mod stub;
pub mod uia;
pub mod vision;
pub mod window_enum;
pub mod window_score;

pub use cache::{FrameCache, InvalidateReason};
pub use context::{PerceptionContext, PerceptionMode};
pub use error::PerceptionError;
pub use frame::{
    now_ms, ElementSource, PerceptionQuality, PerceptionWarning, Rect, ScreenElement,
    ScreenFrame, WindowId, WindowSnapshot,
};
pub use fusion::FusionEngine;
pub use hybrid::HybridPerceiver;
pub use stub::StubPerceiver;
pub use uia::UiaPerceiver;

/// All production perception goes through this trait. Implementations must
/// be `Send + Sync` because the orchestrator clones an `Arc<dyn Perceiver>`
/// into `spawn_blocking` workers.
///
/// `capture` is blocking — the orchestrator wraps it in
/// `tokio::task::spawn_blocking`.
pub trait Perceiver: Send + Sync {
    fn name(&self) -> &str;
    fn capture(&self, ctx: &PerceptionContext) -> Result<ScreenFrame, PerceptionError>;
}

use std::sync::Arc;

use crate::settings::Settings;

/// Build the production perceiver. The hackathon build always uses
/// `HybridPerceiver`; if we ever need a forced-stub override, gate it
/// behind `ROOTA_PERCEPTION_MODE=stub` (not in scope for v1).
pub fn build_perceiver(settings: &Settings) -> Arc<dyn Perceiver> {
    let _ = settings; // perception settings live on the context, not the struct
    Arc::new(HybridPerceiver::new())
}
