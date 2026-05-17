//! Moondream via local Ollama — pixel perception when UIA is sparse.

use std::io::Cursor;

use image::RgbaImage;
use serde::Deserialize;

use crate::llm::ollama::OllamaClient;
use crate::perception::error::PerceptionError;
use crate::perception::frame::{
    ElementSource, PerceptionWarning, ScreenElement, WindowId,
};
use crate::perception::vision::capture::{capture_window_bitmap, CaptureOptions, CapturedFrame};
use crate::perception::vision::coords::map_image_rect_to_screen;
use crate::perception::vision::{VisionCapture, VisionPerceiver, VisionRequest};
use crate::settings::Settings;

const VISION_DETECT_PROMPT: &str = include_str!("../../../prompts/vision_detect.txt");
const VLM_CONFIDENCE: f32 = 0.65;

pub struct MoondreamVisionPerceiver {
    client: OllamaClient,
    max_edge: u32,
    capture_scale: f32,
    debug_capture: bool,
    available: bool,
}

impl MoondreamVisionPerceiver {
    pub fn new(settings: &Settings) -> Self {
        let client = OllamaClient::for_vision(settings);
        let available = client.vision_model_available();
        if available {
            tracing::info!(
                target: "roota.perception.vision",
                model = %client.model(),
                "Moondream vision perceiver ready"
            );
        } else {
            tracing::warn!(
                target: "roota.perception.vision",
                model = %client.model(),
                "Moondream model not found in Ollama — vision fallback disabled"
            );
        }
        let slf = Self {
            client,
            max_edge: settings.perception.vision_max_edge,
            capture_scale: settings.perception.capture_scale,
            debug_capture: settings.perception.debug_capture,
            available,
        };
        if available {
            let client = slf.client.clone();
            std::thread::Builder::new()
                .name("roota-vision-warmup".into())
                .spawn(move || {
                    let started = std::time::Instant::now();
                    match client.warmup_vision_blocking() {
                        Ok(()) => tracing::info!(
                            target: "roota.perception.vision",
                            ms = started.elapsed().as_millis(),
                            "moondream warmup complete"
                        ),
                        Err(err) => tracing::warn!(
                            target: "roota.perception.vision",
                            ms = started.elapsed().as_millis(),
                            "moondream warmup failed: {err}"
                        ),
                    }
                })
                .ok();
        }
        slf
    }
}

impl VisionPerceiver for MoondreamVisionPerceiver {
    fn name(&self) -> &str {
        "moondream"
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
            &CaptureOptions {
                scale: self.capture_scale,
                max_edge: self.max_edge,
                preprocess_ocr: false,
            },
        )
        .map_err(|e| PerceptionError::Capture(e.to_string()))?;

        if bitmap.is_empty() {
            return Ok(VisionCapture {
                elements: Vec::new(),
                warnings: vec![PerceptionWarning::OcrUnavailable],
            });
        }

        if self.debug_capture {
            if let Ok(png) = rgba_to_png(&bitmap) {
                let path = std::env::temp_dir().join(format!(
                    "roota_capture_{}.png",
                    crate::perception::frame::now_ms()
                ));
                if std::fs::write(&path, &png).is_ok() {
                    tracing::info!(target: "roota.perception.vision", path = ?path, "debug capture written");
                }
            }
        }

        let png = rgba_to_png(&bitmap)
            .map_err(|e| PerceptionError::Capture(format!("png encode: {e}")))?;

        let prompt = VISION_DETECT_PROMPT
            .replace("{width}", &bitmap.width.to_string())
            .replace("{height}", &bitmap.height.to_string());

        tracing::debug!(
            target: "roota.perception.vision",
            w = bitmap.width,
            h = bitmap.height,
            png_kb = png.len() / 1024,
            timeout_secs = self.client.timeout_secs(),
            "moondream inference starting"
        );

        let started = std::time::Instant::now();
        let json = self
            .client
            .complete_vision_json_blocking(&prompt, &png)
            .map_err(|e| PerceptionError::Ocr(e.to_string()))?;

        tracing::info!(
            target: "roota.perception.vision",
            ms = started.elapsed().as_millis(),
            "moondream inference complete"
        );

        let elements = parse_vision_elements(&json, req.primary_window_id, &bitmap);
        Ok(VisionCapture {
            elements,
            warnings: Vec::new(),
        })
    }
}

pub fn rgba_to_png(bitmap: &CapturedFrame) -> Result<Vec<u8>, String> {
    let img = RgbaImage::from_raw(bitmap.width, bitmap.height, bitmap.pixels.clone())
        .ok_or_else(|| "invalid rgba dimensions".to_string())?;
    let mut buf = Vec::new();
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(buf)
}

#[derive(Debug, Deserialize)]
struct VisionElementsResponse {
    #[serde(default)]
    elements: Vec<VisionElementJson>,
}

#[derive(Debug, Deserialize)]
struct VisionElementJson {
    text: Option<String>,
    x: Option<i32>,
    y: Option<i32>,
    w: Option<i32>,
    h: Option<i32>,
    kind: Option<String>,
}

/// Map model JSON (image-relative coords) to screen-space `ScreenElement`s.
pub fn parse_vision_elements(
    json: &serde_json::Value,
    window_id: WindowId,
    bitmap: &CapturedFrame,
) -> Vec<ScreenElement> {
    let parsed: VisionElementsResponse = match serde_json::from_value(json.clone()) {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!(
                target: "roota.perception.vision",
                "moondream json parse failed: {err}"
            );
            return Vec::new();
        }
    };

    parsed
        .elements
        .into_iter()
        .filter_map(|el| {
            let text = el.text.unwrap_or_default().trim().to_string();
            if text.is_empty() {
                return None;
            }
            let x = el.x.unwrap_or(0);
            let y = el.y.unwrap_or(0);
            let w = el.w.unwrap_or(40);
            let h = el.h.unwrap_or(24);
            let bounds = map_image_rect_to_screen(
                x,
                y,
                w,
                h,
                bitmap.width,
                bitmap.height,
                bitmap.source_rect,
            );

            Some(ScreenElement {
                source: ElementSource::Vlm,
                text,
                bounds,
                window_id,
                kind: el.kind.unwrap_or_else(|| "Unknown".into()),
                confidence: VLM_CONFIDENCE,
                automation_id: None,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perception::frame::Rect;

    #[test]
    fn parse_vision_elements_maps_coords() {
        let json = serde_json::json!({
            "elements": [
                {"text": "Guardar", "x": 100, "y": 50, "w": 80, "h": 24, "kind": "button"}
            ]
        });
        let bitmap = CapturedFrame {
            width: 800,
            height: 600,
            pixels: vec![],
            source_rect: Rect::new(100, 200, 800, 600),
        };
        let out = parse_vision_elements(&json, WindowId(1), &bitmap);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].text, "Guardar");
        assert_eq!(out[0].source, ElementSource::Vlm);
        assert!(out[0].bounds.x >= 100);
        assert!(out[0].bounds.y >= 200);
    }

    #[test]
    fn parse_vision_elements_skips_empty_text() {
        let json = serde_json::json!({
            "elements": [{"text": "", "x": 0, "y": 0, "w": 10, "h": 10}]
        });
        let bitmap = CapturedFrame {
            width: 100,
            height: 100,
            pixels: vec![],
            source_rect: Rect::new(0, 0, 100, 100),
        };
        assert!(parse_vision_elements(&json, WindowId(1), &bitmap).is_empty());
    }
}
