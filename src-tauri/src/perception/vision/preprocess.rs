//! OCR-oriented bitmap preprocessing and word→line grouping.

use image::{Rgba, RgbaImage};

use crate::perception::frame::{Rect, ScreenElement};

/// Mild contrast stretch on luminance — helps Windows OCR on washed-out themes.
pub fn enhance_for_ocr(img: &RgbaImage) -> RgbaImage {
    if img.width() == 0 || img.height() == 0 {
        return img.clone();
    }

    let mut min_l: u16 = 255;
    let mut max_l: u16 = 0;
    for px in img.pixels() {
        let l = luminance(px);
        min_l = min_l.min(l);
        max_l = max_l.max(l);
    }

    if max_l <= min_l + 8 {
        return img.clone();
    }

    let span = (max_l - min_l) as f32;
    let mut out = img.clone();
    for px in out.pixels_mut() {
        let l = luminance(px);
        let norm = ((l - min_l) as f32 / span).clamp(0.0, 1.0);
        let boosted = (norm * 255.0).round() as u8;
        let scale = boosted as f32 / l.max(1) as f32;
        px[0] = (px[0] as f32 * scale).round().clamp(0.0, 255.0) as u8;
        px[1] = (px[1] as f32 * scale).round().clamp(0.0, 255.0) as u8;
        px[2] = (px[2] as f32 * scale).round().clamp(0.0, 255.0) as u8;
    }
    out
}

fn luminance(px: &Rgba<u8>) -> u16 {
    let r = u32::from(px[0]);
    let g = u32::from(px[1]);
    let b = u32::from(px[2]);
    ((r * 299 + g * 587 + b * 114) / 1000) as u16
}

/// Merge per-word OCR hits on the same text line into one clickable region.
pub fn merge_ocr_words_into_lines(mut elements: Vec<ScreenElement>) -> Vec<ScreenElement> {
    if elements.len() < 2 {
        return elements;
    }

    elements.sort_by_key(|e| (e.bounds.y, e.bounds.x));

    let mut lines: Vec<ScreenElement> = Vec::new();
    for word in elements {
        if let Some(line) = lines.last_mut() {
            if same_ocr_line(line, &word) {
                merge_into(line, &word);
                continue;
            }
        }
        lines.push(word);
    }
    lines
}

fn same_ocr_line(a: &ScreenElement, b: &ScreenElement) -> bool {
    if a.window_id != b.window_id {
        return false;
    }
    let tol = a.bounds.height.max(b.bounds.height).max(12) / 2;
    (a.bounds.y - b.bounds.y).abs() <= tol
}

fn merge_into(line: &mut ScreenElement, word: &ScreenElement) {
    let right = line.bounds.x + line.bounds.width;
    let word_right = word.bounds.x + word.bounds.width;
    let left = line.bounds.x.min(word.bounds.x);
    let top = line.bounds.y.min(word.bounds.y);
    let bottom = (line.bounds.y + line.bounds.height).max(word.bounds.y + word.bounds.height);
    line.bounds = Rect::new(left, top, right.max(word_right) - left, bottom - top);
    if !line.text.contains(&word.text) {
        if !line.text.is_empty() {
            line.text.push(' ');
        }
        line.text.push_str(word.text.trim());
    }
    line.confidence = line.confidence.min(word.confidence);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perception::frame::{ElementSource, WindowId};

    fn ocr_word(text: &str, x: i32, y: i32, w: i32, h: i32) -> ScreenElement {
        ScreenElement {
            source: ElementSource::Ocr,
            text: text.into(),
            bounds: Rect::new(x, y, w, h),
            window_id: WindowId(1),
            kind: "text".into(),
            confidence: 0.88,
            automation_id: None,
        }
    }

    #[test]
    fn merge_ocr_words_joins_same_line() {
        let words = vec![
            ocr_word("Nueva", 10, 100, 40, 18),
            ocr_word("pestaña", 55, 102, 50, 18),
        ];
        let merged = merge_ocr_words_into_lines(words);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "Nueva pestaña");
        assert!(merged[0].bounds.width >= 90);
    }

    #[test]
    fn merge_ocr_words_keeps_different_lines() {
        let words = vec![
            ocr_word("Archivo", 10, 20, 50, 18),
            ocr_word("Editar", 10, 60, 50, 18),
        ];
        assert_eq!(merge_ocr_words_into_lines(words).len(), 2);
    }

    #[test]
    fn enhance_for_ocr_stretches_low_contrast() {
        let mut img = RgbaImage::new(4, 4);
        for (x, y, px) in img.enumerate_pixels_mut() {
            let v = if (x + y) % 2 == 0 { 90 } else { 140 };
            *px = Rgba([v, v, v, 255]);
        }
        let out = enhance_for_ocr(&img);
        let dark = out.get_pixel(0, 0)[0];
        let light = out.get_pixel(1, 0)[0];
        assert!(dark < 90, "dark pixels should deepen, got {dark}");
        assert!(light > 140, "light pixels should brighten, got {light}");
    }
}
