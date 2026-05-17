//! Windows.Media.Ocr — fast native text detection for hybrid perception.

use crate::perception::error::PerceptionError;
use crate::perception::frame::{
    ElementSource, PerceptionWarning, Rect, ScreenElement, WindowId,
};
use crate::perception::vision::capture::{capture_window_bitmap, CapturedFrame};
use crate::perception::vision::{VisionCapture, VisionPerceiver, VisionRequest};
use crate::settings::PerceptionSettings;

const OCR_CONFIDENCE: f32 = 0.88;

#[derive(Debug)]
pub struct WindowsOcrPerceiver {
    available: bool,
    max_edge: u32,
}

impl WindowsOcrPerceiver {
    pub fn new(settings: &PerceptionSettings) -> Self {
        let available = probe_ocr_engine();
        if available {
            tracing::info!(
                target: "roota.perception.vision",
                "Windows.Media.Ocr ready"
            );
        } else {
            tracing::warn!(
                target: "roota.perception.vision",
                "Windows.Media.Ocr unavailable — vision layer disabled"
            );
        }
        Self {
            available,
            max_edge: settings.vision_max_edge,
        }
    }
}

impl Default for WindowsOcrPerceiver {
    fn default() -> Self {
        Self::new(&PerceptionSettings::default())
    }
}

impl VisionPerceiver for WindowsOcrPerceiver {
    fn name(&self) -> &str {
        "windows-ocr"
    }

    fn is_available(&self) -> bool {
        self.available
    }

    fn recognize(&self, req: &VisionRequest) -> Result<VisionCapture, PerceptionError> {
        if !self.available {
            return Ok(VisionCapture {
                elements: Vec::new(),
                warnings: vec![PerceptionWarning::OcrUnavailable],
            });
        }

        let bitmap = capture_window_bitmap(
            req.primary_window_rect,
            req.scale,
            self.max_edge,
        )
        .map_err(|e| PerceptionError::Capture(e.to_string()))?;

        if bitmap.is_empty() {
            return Ok(VisionCapture {
                elements: Vec::new(),
                warnings: vec![PerceptionWarning::OcrUnavailable],
            });
        }

        let started = std::time::Instant::now();
        let elements = recognize_bitmap(&bitmap, req.primary_window_id, &req.language)
            .map_err(|e| PerceptionError::Ocr(e))?;

        tracing::info!(
            target: "roota.perception.vision",
            ms = started.elapsed().as_millis(),
            lines = elements.len(),
            "windows OCR complete"
        );

        Ok(VisionCapture {
            elements,
            warnings: Vec::new(),
        })
    }
}

/// Map OCR word box (image coords) to screen-space `ScreenElement`.
pub fn map_ocr_word_to_element(
    text: &str,
    word_x: i32,
    word_y: i32,
    word_w: i32,
    word_h: i32,
    bitmap: &CapturedFrame,
    window_id: WindowId,
) -> ScreenElement {
    let img_w = bitmap.width.max(1) as i32;
    let img_h = bitmap.height.max(1) as i32;
    let rect = bitmap.source_rect;

    let x = word_x.clamp(0, img_w);
    let y = word_y.clamp(0, img_h);
    let w = word_w.clamp(4, img_w - x);
    let h = word_h.clamp(4, img_h - y);

    let screen_x = rect.x + (x * rect.width) / img_w;
    let screen_y = rect.y + (y * rect.height) / img_h;
    let screen_w = ((w * rect.width) / img_w).max(4);
    let screen_h = ((h * rect.height) / img_h).max(4);

    ScreenElement {
        source: ElementSource::Ocr,
        text: text.trim().to_string(),
        bounds: Rect::new(screen_x, screen_y, screen_w, screen_h),
        window_id,
        kind: "text".into(),
        confidence: OCR_CONFIDENCE,
        automation_id: None,
    }
}

#[cfg(windows)]
fn probe_ocr_engine() -> bool {
    windows::Media::Ocr::OcrEngine::TryCreateFromUserProfileLanguages().is_ok()
}

#[cfg(not(windows))]
fn probe_ocr_engine() -> bool {
    false
}

#[cfg(windows)]
fn recognize_bitmap(
    bitmap: &CapturedFrame,
    window_id: WindowId,
    language: &str,
) -> Result<Vec<ScreenElement>, String> {
    use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
    use windows::Storage::Streams::DataWriter;

    let mut bgra = bitmap.pixels.clone();
    for chunk in bgra.chunks_exact_mut(4) {
        chunk.swap(0, 2);
    }

    let writer = DataWriter::new().map_err(|e| format!("DataWriter: {e}"))?;
    writer
        .WriteBytes(&bgra)
        .map_err(|e| format!("WriteBytes: {e}"))?;
    let buffer = writer
        .DetachBuffer()
        .map_err(|e| format!("DetachBuffer: {e}"))?;

    let software = SoftwareBitmap::CreateCopyFromBuffer(
        &buffer,
        BitmapPixelFormat::Bgra8,
        bitmap.width as i32,
        bitmap.height as i32,
    )
    .map_err(|e| format!("SoftwareBitmap: {e}"))?;

    let engine = ocr_engine_for_language(language)?;

    let result = engine
        .RecognizeAsync(&software)
        .map_err(|e| format!("RecognizeAsync: {e}"))?
        .get()
        .map_err(|e| format!("OCR result: {e}"))?;

    let lines = result
        .Lines()
        .map_err(|e| format!("Lines: {e}"))?;

    let mut elements = Vec::new();
    for line in lines {
        let words = match line.Words() {
            Ok(w) => w,
            Err(_) => continue,
        };
        for word in words {
            let text = word.Text().map(|s| s.to_string()).unwrap_or_default();
            if text.trim().is_empty() {
                continue;
            }
            let rect = match word.BoundingRect() {
                Ok(r) => r,
                Err(_) => continue,
            };
            elements.push(map_ocr_word_to_element(
                &text,
                rect.X as i32,
                rect.Y as i32,
                rect.Width as i32,
                rect.Height as i32,
                bitmap,
                window_id,
            ));
        }
    }

    Ok(elements)
}

#[cfg(windows)]
fn ocr_engine_for_language(lang_tag: &str) -> Result<windows::Media::Ocr::OcrEngine, String> {
    use windows::core::HSTRING;
    use windows::Globalization::Language;
    use windows::Media::Ocr::OcrEngine;

    let tag = lang_tag.trim().to_lowercase();
    let candidates: Vec<&str> = if tag.starts_with("es") {
        vec!["es-ES", "es-MX", "es"]
    } else if tag.starts_with("en") {
        vec!["en-US", "en"]
    } else {
        vec![tag.as_str()]
    };

    for candidate in candidates {
        let h = HSTRING::from(candidate);
        if let Ok(lang) = Language::CreateLanguage(&h) {
            if let Ok(engine) = OcrEngine::TryCreateFromLanguage(&lang) {
                return Ok(engine);
            }
        }
    }

    OcrEngine::TryCreateFromUserProfileLanguages().map_err(|e| format!("OCR engine: {e}"))
}

#[cfg(not(windows))]
fn recognize_bitmap(
    _bitmap: &CapturedFrame,
    _window_id: WindowId,
    _language: &str,
) -> Result<Vec<ScreenElement>, String> {
    Err("Windows OCR only available on Windows".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_ocr_word_scales_to_screen_rect() {
        let bitmap = CapturedFrame {
            width: 800,
            height: 600,
            pixels: vec![],
            source_rect: Rect::new(100, 200, 800, 600),
        };
        let el = map_ocr_word_to_element("Guardar", 100, 50, 80, 24, &bitmap, WindowId(1));
        assert_eq!(el.text, "Guardar");
        assert_eq!(el.source, ElementSource::Ocr);
        assert!(el.bounds.x >= 100);
        assert!(el.bounds.y >= 200);
    }
}
