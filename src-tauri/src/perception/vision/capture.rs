//! Bitmap capture for a single window rectangle (physical screen coords).

use crate::perception::frame::Rect;

#[derive(Debug, Clone)]
pub struct CapturedFrame {
    pub width: u32,
    pub height: u32,
    /// RGBA8 little-endian.
    pub pixels: Vec<u8>,
    /// Physical screen rect that was captured (may differ from request after clamp).
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
    #[error("screen capture not available on this platform")]
    Unimplemented,
    #[error("capture failed: {0}")]
    Platform(String),
}

/// Capture the region `rect` (physical screen coords), downscale so the long
/// edge is at most `max_edge`, then apply an extra `scale` factor (0..1].
pub fn capture_window_bitmap(
    rect: Rect,
    scale: f32,
    max_edge: u32,
) -> Result<CapturedFrame, CaptureError> {
    #[cfg(windows)]
    {
        windows_impl::capture(rect, scale, max_edge)
    }
    #[cfg(not(windows))]
    {
        let _ = (scale, max_edge);
        tracing::debug!(
            target: "roota.perception.vision",
            rect = ?rect,
            "capture_window_bitmap unavailable (non-windows)"
        );
        Ok(CapturedFrame::empty(rect))
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::{CaptureError, CapturedFrame};
    use crate::perception::frame::Rect;
    use image::imageops::FilterType;
    use image::RgbaImage;

    pub fn capture(rect: Rect, scale: f32, max_edge: u32) -> Result<CapturedFrame, CaptureError> {
        if rect.width <= 0 || rect.height <= 0 {
            return Ok(CapturedFrame::empty(rect));
        }

        let (cx, cy) = rect.center();
        let monitors = xcap::Monitor::all()
            .map_err(|e| CaptureError::Platform(format!("Monitor::all: {e}")))?;

        let monitor = monitors
            .into_iter()
            .find(|m| monitor_contains(m, cx, cy))
            .ok_or_else(|| CaptureError::Platform("no monitor for capture rect".into()))?;

        let monitor_x = monitor.x();
        let monitor_y = monitor.y();
        let monitor_w = monitor.width() as i32;
        let monitor_h = monitor.height() as i32;

        let mon_rect = Rect::new(monitor_x, monitor_y, monitor_w, monitor_h);
        let crop = intersect_rect(rect, mon_rect);
        if crop.width <= 0 || crop.height <= 0 {
            return Ok(CapturedFrame::empty(rect));
        }

        let full = monitor
            .capture_image()
            .map_err(|e| CaptureError::Platform(format!("capture_image: {e}")))?;

        let rel_x = (crop.x - monitor_x).max(0) as u32;
        let rel_y = (crop.y - monitor_y).max(0) as u32;
        let crop_w = crop.width.min(monitor_w - rel_x as i32).max(0) as u32;
        let crop_h = crop.height.min(monitor_h - rel_y as i32).max(0) as u32;

        if crop_w == 0 || crop_h == 0 {
            return Ok(CapturedFrame::empty(crop));
        }

        let cropped = crop_rgba(&full, rel_x, rel_y, crop_w, crop_h);
        let mut img = cropped;
        let max_edge = max_edge.max(64);
        let long_edge = img.width().max(img.height());
        let mut target_long = max_edge;
        if scale > 0.0 && scale < 1.0 {
            target_long = ((target_long as f32) * scale.clamp(0.1, 1.0)) as u32;
        }
        if long_edge > target_long && target_long > 0 {
            let ratio = target_long as f32 / long_edge as f32;
            let nw = ((img.width() as f32) * ratio).max(1.0) as u32;
            let nh = ((img.height() as f32) * ratio).max(1.0) as u32;
            img = image::imageops::resize(&img, nw, nh, FilterType::Triangle);
        }

        let width = img.width();
        let height = img.height();
        Ok(CapturedFrame {
            width,
            height,
            pixels: img.into_raw(),
            source_rect: crop,
        })
    }

    fn monitor_contains(m: &xcap::Monitor, px: i32, py: i32) -> bool {
        let x = m.x();
        let y = m.y();
        let w = m.width() as i32;
        let h = m.height() as i32;
        Rect::new(x, y, w, h).contains(px, py)
    }

    fn intersect_rect(a: Rect, b: Rect) -> Rect {
        let left = a.x.max(b.x);
        let top = a.y.max(b.y);
        let right = (a.x + a.width).min(b.x + b.width);
        let bottom = (a.y + a.height).min(b.y + b.height);
        Rect::from_ltrb(left, top, right, bottom)
    }

    fn crop_rgba(img: &RgbaImage, x: u32, y: u32, w: u32, h: u32) -> RgbaImage {
        let mut out = RgbaImage::new(w, h);
        for row in 0..h {
            for col in 0..w {
                let px = img.get_pixel(x + col, y + row);
                out.put_pixel(col, row, *px);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_windows_capture_returns_empty() {
        let r = Rect::new(0, 0, 800, 600);
        let frame = capture_window_bitmap(r, 0.75, 768).unwrap();
        #[cfg(not(windows))]
        assert!(frame.is_empty());
        assert_eq!(frame.source_rect, r);
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "requires display; run manually"]
    fn windows_capture_non_empty() {
        let r = Rect::new(0, 0, 200, 200);
        let frame = capture_window_bitmap(r, 0.75, 768).unwrap();
        assert!(!frame.is_empty());
        assert!(frame.width > 0 && frame.height > 0);
    }
}
