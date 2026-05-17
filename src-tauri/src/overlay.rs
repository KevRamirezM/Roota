//! Map UI Automation screen coordinates to overlay webview logical pixels.

use serde::Serialize;
use tauri::WebviewWindow;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Pure conversion for unit tests (screen physical px → overlay logical px).
pub fn screen_rect_to_logical(
    screen_x: i32,
    screen_y: i32,
    width: i32,
    height: i32,
    origin_x: f64,
    origin_y: f64,
    scale: f64,
) -> OverlayRect {
    let scale = scale.max(0.01);
    OverlayRect {
        x: (screen_x as f64 - origin_x) / scale,
        y: (screen_y as f64 - origin_y) / scale,
        width: width as f64 / scale,
        height: height as f64 / scale,
    }
}

pub fn screen_point_to_logical(
    x: i32,
    y: i32,
    origin_x: f64,
    origin_y: f64,
    scale: f64,
) -> (f64, f64) {
    let r = screen_rect_to_logical(x, y, 0, 0, origin_x, origin_y, scale);
    (r.x, r.y)
}

pub fn screen_rect_to_overlay(
    overlay: &WebviewWindow,
    screen_x: i32,
    screen_y: i32,
    width: i32,
    height: i32,
) -> Option<OverlayRect> {
    let scale = overlay.scale_factor().ok()?;
    let pos = overlay.outer_position().ok()?;
    Some(screen_rect_to_logical(
        screen_x,
        screen_y,
        width,
        height,
        pos.x as f64,
        pos.y as f64,
        scale,
    ))
}

pub fn screen_center_to_overlay(overlay: &WebviewWindow, cx: i32, cy: i32) -> Option<(f64, f64)> {
    let scale = overlay.scale_factor().ok()?;
    let pos = overlay.outer_position().ok()?;
    Some(screen_point_to_logical(
        cx,
        cy,
        pos.x as f64,
        pos.y as f64,
        scale,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_coords_undo_scale_and_origin() {
        let r = screen_rect_to_logical(300, 400, 160, 32, 0.0, 0.0, 2.0);
        assert!((r.x - 150.0).abs() < 0.01);
        assert!((r.y - 200.0).abs() < 0.01);
        assert!((r.width - 80.0).abs() < 0.01);
        assert!((r.height - 16.0).abs() < 0.01);
    }

    #[test]
    fn subtracts_monitor_origin() {
        let r = screen_rect_to_logical(1920, 100, 80, 40, 1920.0, 0.0, 1.0);
        assert!((r.x - 0.0).abs() < 0.01);
        assert!((r.y - 100.0).abs() < 0.01);
    }
}
