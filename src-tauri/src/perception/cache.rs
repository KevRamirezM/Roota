//! Tiny single-slot frame cache for the guide poll loop.
//!
//! Invalidation rules (per spec/plan Task 16, strict):
//!   - bypass when cursor moved more than `CURSOR_MOVE_INVALIDATE_PX` pixels
//!   - bypass on user click (any button) since the cache was filled
//!   - bypass on foreground/primary window change
//!   - bypass on step boundary or perception error retry
//!   - never cache `Err` results
//!
//! Cache TTL is short on purpose (500ms default) — perception is cheap.

use std::sync::Mutex;

use crate::input::PhysicalPoint;
use crate::perception::frame::{ScreenFrame, WindowId};

pub const DEFAULT_TTL_MS: u64 = 500;
pub const CURSOR_MOVE_INVALIDATE_PX: i32 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidateReason {
    /// First call / cache cold.
    Initial,
    /// User clicked (any button).
    UserAction,
    /// Foreground HWND or primary window changed since last frame.
    ForegroundChanged,
    /// Step boundary — must re-perceive.
    StepBoundary,
    /// Previous perception failed; do not reuse.
    PerceptionError,
}

#[derive(Debug, Clone)]
struct CachedEntry {
    frame: ScreenFrame,
    cursor: PhysicalPoint,
    primary_window_id: WindowId,
    captured_at_ms: u64,
}

#[derive(Debug, Default)]
pub struct FrameCache {
    inner: Mutex<Option<CachedEntry>>,
    ttl_ms: u64,
}

impl FrameCache {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
            ttl_ms: DEFAULT_TTL_MS,
        }
    }

    pub fn with_ttl(ttl_ms: u64) -> Self {
        Self {
            inner: Mutex::new(None),
            ttl_ms,
        }
    }

    pub fn ttl_ms(&self) -> u64 {
        self.ttl_ms
    }

    /// Get a cached frame *only* when **all** of these hold:
    ///   - age < ttl
    ///   - cursor moved ≤ CURSOR_MOVE_INVALIDATE_PX since cached frame
    ///   - foreground (primary) window unchanged
    ///   - reason is not an explicit invalidation
    pub fn get(
        &self,
        now_ms: u64,
        cursor: PhysicalPoint,
        foreground: Option<WindowId>,
        reason: InvalidateReason,
    ) -> Option<ScreenFrame> {
        if !matches!(reason, InvalidateReason::Initial) {
            return None;
        }
        let guard = self.inner.lock().ok()?;
        let entry = guard.as_ref()?;

        if now_ms.saturating_sub(entry.captured_at_ms) >= self.ttl_ms {
            return None;
        }

        let dx = (cursor.x - entry.cursor.x).abs();
        let dy = (cursor.y - entry.cursor.y).abs();
        if dx > CURSOR_MOVE_INVALIDATE_PX || dy > CURSOR_MOVE_INVALIDATE_PX {
            return None;
        }

        if let Some(fg) = foreground {
            if fg != entry.primary_window_id {
                return None;
            }
        }

        Some(entry.frame.clone())
    }

    /// Replace the cache with a freshly captured frame.
    pub fn put(&self, frame: ScreenFrame) {
        let entry = CachedEntry {
            cursor: frame.cursor,
            primary_window_id: frame.primary_window_id,
            captured_at_ms: frame.captured_at_ms,
            frame,
        };
        if let Ok(mut guard) = self.inner.lock() {
            *guard = Some(entry);
        }
    }

    pub fn clear(&self) {
        if let Ok(mut guard) = self.inner.lock() {
            *guard = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perception::frame::{PerceptionQuality, ScreenFrame, WindowId};

    fn frame_at(t: u64, cursor_x: i32, primary: u64) -> ScreenFrame {
        ScreenFrame {
            captured_at_ms: t,
            cursor: PhysicalPoint { x: cursor_x, y: 0 },
            primary_window_id: WindowId(primary),
            quality: PerceptionQuality::Full,
            ..ScreenFrame::empty()
        }
    }

    #[test]
    fn cache_returns_frame_within_ttl_when_idle() {
        let cache = FrameCache::with_ttl(500);
        cache.put(frame_at(100, 0, 1));
        let hit = cache.get(
            300,
            PhysicalPoint { x: 0, y: 0 },
            Some(WindowId(1)),
            InvalidateReason::Initial,
        );
        assert!(hit.is_some());
    }

    #[test]
    fn cache_bypassed_on_user_action() {
        let cache = FrameCache::with_ttl(500);
        cache.put(frame_at(100, 0, 1));
        let hit = cache.get(
            150,
            PhysicalPoint { x: 0, y: 0 },
            Some(WindowId(1)),
            InvalidateReason::UserAction,
        );
        assert!(hit.is_none());
    }

    #[test]
    fn cache_bypassed_when_cursor_moves_far() {
        let cache = FrameCache::with_ttl(500);
        cache.put(frame_at(100, 0, 1));
        let hit = cache.get(
            200,
            PhysicalPoint { x: 200, y: 0 },
            Some(WindowId(1)),
            InvalidateReason::Initial,
        );
        assert!(hit.is_none());
    }

    #[test]
    fn cache_bypassed_when_primary_changes() {
        let cache = FrameCache::with_ttl(500);
        cache.put(frame_at(100, 0, 1));
        let hit = cache.get(
            200,
            PhysicalPoint { x: 0, y: 0 },
            Some(WindowId(2)),
            InvalidateReason::Initial,
        );
        assert!(hit.is_none());
    }

    #[test]
    fn cache_bypassed_after_ttl() {
        let cache = FrameCache::with_ttl(500);
        cache.put(frame_at(100, 0, 1));
        let hit = cache.get(
            900,
            PhysicalPoint { x: 0, y: 0 },
            Some(WindowId(1)),
            InvalidateReason::Initial,
        );
        assert!(hit.is_none());
    }
}
