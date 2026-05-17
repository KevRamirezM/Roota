//! Image ↔ screen coordinate helpers shared by OCR and VLM layers.

use crate::perception::frame::Rect;

/// Expand a rectangle by `margin` on each side, optionally clamped to `bounds`.
pub fn inflate_rect(rect: Rect, margin: i32, clamp: Option<Rect>) -> Rect {
    if margin <= 0 {
        return clamp.map(|c| intersect_rect(rect, c)).unwrap_or(rect);
    }
    let inflated = Rect::new(
        rect.x - margin,
        rect.y - margin,
        rect.width + 2 * margin,
        rect.height + 2 * margin,
    );
    match clamp {
        Some(c) => intersect_rect(inflated, c),
        None => inflated,
    }
}

pub fn intersect_rect(a: Rect, b: Rect) -> Rect {
    let left = a.x.max(b.x);
    let top = a.y.max(b.y);
    let right = (a.x + a.width).min(b.x + b.width);
    let bottom = (a.y + a.height).min(b.y + b.height);
    Rect::from_ltrb(left, top, right, bottom)
}

/// Map a box in captured-bitmap pixels to physical screen coordinates.
pub fn map_image_rect_to_screen(
    ix: i32,
    iy: i32,
    iw: i32,
    ih: i32,
    img_w: u32,
    img_h: u32,
    source_rect: Rect,
) -> Rect {
    let img_w = img_w.max(1) as f64;
    let img_h = img_h.max(1) as f64;
    let sw = source_rect.width.max(1) as f64;
    let sh = source_rect.height.max(1) as f64;

    let x = ix.max(0) as f64;
    let y = iy.max(0) as f64;
    let w = iw.max(4) as f64;
    let h = ih.max(4) as f64;

    let screen_x = source_rect.x as f64 + (x / img_w) * sw;
    let screen_y = source_rect.y as f64 + (y / img_h) * sh;
    let screen_w = ((w / img_w) * sw).max(4.0);
    let screen_h = ((h / img_h) * sh).max(4.0);

    Rect::new(
        screen_x.round() as i32,
        screen_y.round() as i32,
        screen_w.round() as i32,
        screen_h.round() as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_image_rect_rounds_half_pixel_offsets() {
        let r = map_image_rect_to_screen(100, 50, 80, 24, 800, 600, Rect::new(100, 200, 800, 600));
        assert_eq!(r.x, 200);
        assert_eq!(r.y, 250);
        assert_eq!(r.width, 80);
        assert_eq!(r.height, 24);
    }

    #[test]
    fn inflate_rect_adds_margin_and_clamps() {
        let inner = Rect::new(50, 50, 100, 100);
        let clamp = Rect::new(0, 0, 200, 200);
        let out = inflate_rect(inner, 20, Some(clamp));
        assert_eq!(out.x, 30);
        assert_eq!(out.y, 30);
        assert_eq!(out.width, 140);
        assert_eq!(out.height, 140);
    }

    #[test]
    fn intersect_rect_keeps_overlap_only() {
        let a = Rect::new(0, 0, 100, 100);
        let b = Rect::new(50, 50, 100, 100);
        let i = intersect_rect(a, b);
        assert_eq!(i, Rect::new(50, 50, 50, 50));
    }
}
