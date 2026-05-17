//! Score-then-cap window selection.
//!
//! **Critical ordering invariant:** enumerate *all* candidates → score every
//! one → sort by score descending → take top K. Capping before scoring drops
//! the correct background window when the user hints at it (see spec).

use crate::perception::context::PerceptionContext;
use crate::perception::window_enum::WindowMeta;

const W_FOREGROUND: i32 = 40;
const W_CURSOR_INSIDE: i32 = 35;
const W_HINT_MATCH: i32 = 50;
const W_NONEMPTY_UIA: i32 = 10;
const W_MODAL_OWNED: i32 = 30;
/// System overlay HWNDs (touch keyboard, Input Experience) must not become primary.
const W_JUNK_OVERLAY: i32 = -90;

#[derive(Debug, Clone)]
pub struct RankedWindow {
    pub meta: WindowMeta,
    pub score: i32,
}

impl RankedWindow {
    pub fn id(&self) -> crate::perception::frame::WindowId {
        self.meta.id
    }

    pub fn title(&self) -> &str {
        &self.meta.title
    }
}

/// Score every candidate, sort descending, then cap to `ctx.max_windows()`.
pub fn rank_windows(windows: &[WindowMeta], ctx: &PerceptionContext) -> Vec<RankedWindow> {
    let max = ctx.max_windows();
    let mut ranked: Vec<RankedWindow> = windows
        .iter()
        .filter(|w| !w.is_roota && w.is_visible && !w.is_minimized)
        .map(|w| RankedWindow {
            score: score_window(w, ctx, windows),
            meta: w.clone(),
        })
        .collect();

    ranked.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then(b.meta.area().cmp(&a.meta.area()))
    });

    ranked.truncate(max);
    ranked
}

/// Number of visible non-Roota windows (for the `WindowCapTruncated` warning).
pub fn visible_count(windows: &[WindowMeta]) -> u32 {
    windows
        .iter()
        .filter(|w| !w.is_roota && w.is_visible && !w.is_minimized)
        .count() as u32
}

fn score_window(w: &WindowMeta, ctx: &PerceptionContext, all: &[WindowMeta]) -> i32 {
    let mut score = 0i32;

    if w.is_foreground {
        score += W_FOREGROUND;
    }

    if w.contains_point(ctx.cursor.x, ctx.cursor.y) {
        score += W_CURSOR_INSIDE;
    }

    let title_lower = w.title.to_lowercase();
    for hint in &ctx.window_hints {
        let h = hint.trim().to_lowercase();
        if !h.is_empty() && title_lower.contains(&h) {
            score += W_HINT_MATCH;
            break;
        }
    }

    if let Some(parent_id) = w.owner_id {
        if let Some(parent) = all.iter().find(|p| p.id == parent_id) {
            let parent_title = parent.title.to_lowercase();
            for hint in &ctx.window_hints {
                let h = hint.trim().to_lowercase();
                if !h.is_empty() && parent_title.contains(&h) {
                    score += W_MODAL_OWNED;
                    break;
                }
            }
        }
    }

    let area = w.area().min(i32::MAX as i64) as i32;
    score += area / 50_000;

    if w.bounds.width < 80 || w.bounds.height < 40 {
        score -= 20;
    }

    if !w.title.trim().is_empty() {
        score += W_NONEMPTY_UIA / 2;
    }

    if is_junk_overlay_window(w) {
        score += W_JUNK_OVERLAY;
    }

    score
}

/// Shell overlays and IME surfaces that steal foreground but are not user apps.
fn is_junk_overlay_window(w: &WindowMeta) -> bool {
    let title = w.title.to_lowercase();
    let class = w.class_name.to_lowercase();

    if title.contains("experiencia de entrada")
        || title.contains("input experience")
        || title.contains("windows input experience")
        || title.contains("msctfime ui")
        || title.contains("program manager")
    {
        return true;
    }

    if class.contains("foregroundstaging")
        || class.contains("tooltips_class32")
        || (class.contains("corewindow") && w.bounds.width < 320 && w.bounds.height < 200)
    {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::PhysicalPoint;
    use crate::perception::frame::{Rect, WindowId};
    use crate::perception::window_enum::WindowMeta;
    use crate::settings::PerceptionSettings;

    fn meta(id: u64, title: &str, x: i32, y: i32, w: i32, h: i32) -> WindowMeta {
        WindowMeta {
            id: WindowId(id),
            title: title.into(),
            class_name: "TestClass".into(),
            bounds: Rect::new(x, y, w, h),
            is_foreground: false,
            is_visible: true,
            is_minimized: false,
            is_roota: false,
            owner_id: None,
            z_order: 0,
        }
    }

    fn ctx_with_cursor_and_hint(x: i32, y: i32, hint: &str) -> PerceptionContext {
        PerceptionContext {
            cursor: PhysicalPoint { x, y },
            window_hints: vec![hint.into()],
            settings: PerceptionSettings::default(),
        }
    }

    fn ctx_with_explorer_hint() -> PerceptionContext {
        PerceptionContext {
            cursor: PhysicalPoint { x: 100, y: 100 },
            window_hints: vec!["explorador".into()],
            settings: PerceptionSettings::default(),
        }
    }

    fn many_windows_fixture(n: usize) -> Vec<WindowMeta> {
        let mut out = Vec::new();
        for i in 0..n {
            let title = if i == 7 {
                "Explorador de archivos".to_string()
            } else {
                format!("App {i}")
            };
            out.push(meta(i as u64 + 1, &title, 0, 0, 800, 600));
        }
        out[0].is_foreground = true;
        out
    }

    #[test]
    fn cursor_inside_adds_weight() {
        let wins = vec![meta(1, "Explorer", 0, 0, 800, 600)];
        let ranked = rank_windows(&wins, &ctx_with_cursor_and_hint(100, 100, "explorador"));
        assert_eq!(ranked[0].id(), wins[0].id);
    }

    #[test]
    fn hinted_background_beats_foreground_without_hint() {
        let mut wins = vec![
            meta(1, "Notepad", 0, 0, 400, 300),
            meta(2, "Explorador de archivos", 800, 0, 800, 600),
        ];
        wins[0].is_foreground = true;
        let ctx = PerceptionContext {
            cursor: PhysicalPoint { x: 10_000, y: 10_000 },
            window_hints: vec!["explorador".into()],
            settings: PerceptionSettings::default(),
        };
        let ranked = rank_windows(&wins, &ctx);
        assert_eq!(ranked[0].id(), WindowId(2));
    }

    #[test]
    fn cap_applied_after_sort_not_before() {
        let wins = many_windows_fixture(20);
        let ranked = rank_windows(&wins, &ctx_with_explorer_hint());
        assert!(ranked.len() <= 8);
        assert!(
            ranked[0].title().to_lowercase().contains("explorador"),
            "expected Explorador first, got {:?}",
            ranked[0].title()
        );
    }

    #[test]
    fn roota_windows_excluded() {
        let mut wins = vec![
            meta(1, "Roota", 0, 0, 360, 480),
            meta(2, "App", 400, 0, 800, 600),
        ];
        wins[0].is_roota = true;
        let ctx = PerceptionContext::default();
        let ranked = rank_windows(&wins, &ctx);
        assert!(ranked.iter().all(|r| !r.meta.is_roota));
    }

    #[test]
    fn modal_owned_by_hinted_app_gets_bonus() {
        let parent = meta(10, "Explorador de archivos", 0, 0, 1280, 720);
        let mut dialog = meta(11, "Confirmar", 400, 200, 400, 200);
        dialog.owner_id = Some(WindowId(10));
        let other = meta(12, "Random", 1500, 0, 600, 400);
        let wins = vec![parent, dialog.clone(), other];
        let ctx = PerceptionContext {
            cursor: PhysicalPoint { x: 5000, y: 5000 },
            window_hints: vec!["explorador".into()],
            settings: PerceptionSettings::default(),
        };
        let ranked = rank_windows(&wins, &ctx);
        let dialog_rank = ranked
            .iter()
            .position(|r| r.id() == dialog.id)
            .expect("dialog ranked");
        let other_rank = ranked
            .iter()
            .position(|r| r.id() == WindowId(12))
            .unwrap_or(usize::MAX);
        assert!(dialog_rank < other_rank);
    }

    #[test]
    fn junk_overlay_loses_to_real_app() {
        let mut overlay = meta(1, "Experiencia de entrada de Windows", 0, 0, 200, 80);
        overlay.is_foreground = true;
        let cursor = meta(2, "Cursor", 0, 0, 1280, 800);
        let ctx = PerceptionContext {
            cursor: PhysicalPoint { x: 640, y: 400 },
            window_hints: vec!["cursor".into()],
            settings: PerceptionSettings::default(),
        };
        let ranked = rank_windows(&[overlay, cursor], &ctx);
        assert_eq!(ranked[0].title(), "Cursor");
    }

    #[test]
    fn visible_count_excludes_roota() {
        let mut wins = vec![
            meta(1, "Explorador de archivos", 0, 0, 800, 600),
            meta(2, "Google Chrome", 800, 0, 800, 600),
        ];
        wins.push(meta(99, "Roota", 0, 0, 100, 100));
        if let Some(last) = wins.last_mut() {
            last.is_roota = true;
        }
        assert_eq!(visible_count(&wins), 2);
    }
}
