//! Inputs to a single perception cycle: cursor, hints, and live settings.

use crate::accessibility::scanner::ScanContext;
use crate::input::PhysicalPoint;
use crate::settings::PerceptionSettings;

/// Per-capture parameters that travel through every `Perceiver`.
#[derive(Debug, Clone, Default)]
pub struct PerceptionContext {
    pub cursor: PhysicalPoint,
    pub window_hints: Vec<String>,
    pub settings: PerceptionSettings,
}

impl PerceptionContext {
    pub fn from_scan_ctx(scan: &ScanContext, cursor: PhysicalPoint) -> Self {
        Self {
            cursor,
            window_hints: scan.window_hints.clone(),
            settings: PerceptionSettings::default(),
        }
    }

    pub fn with_settings(mut self, settings: PerceptionSettings) -> Self {
        self.settings = settings;
        self
    }

    pub fn max_windows(&self) -> usize {
        self.settings.max_windows.max(1)
    }

    pub fn min_uia_elements(&self) -> usize {
        self.settings.min_uia_elements
    }

    pub fn vision_enabled(&self) -> bool {
        self.settings.vision_enabled
    }

    pub fn perception_mode(&self) -> PerceptionMode {
        self.settings.mode
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PerceptionMode {
    /// UIA only — skip OCR even when sparse.
    Uia,
    /// UIA first; OCR fills in when primary window is sparse.
    #[default]
    Hybrid,
    /// Vision only — dev/debug, no UIA tree walk.
    VisionOnly,
}

impl PerceptionMode {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "uia" => Self::Uia,
            "vision_only" | "vision-only" | "visiononly" => Self::VisionOnly,
            _ => Self::Hybrid,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Uia => "uia",
            Self::Hybrid => "hybrid",
            Self::VisionOnly => "vision_only",
        }
    }

    pub fn uia_enabled(&self) -> bool {
        !matches!(self, Self::VisionOnly)
    }

    pub fn vision_enabled(&self) -> bool {
        !matches!(self, Self::Uia)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_perception_mode_falls_back_to_hybrid() {
        assert_eq!(PerceptionMode::parse("uia"), PerceptionMode::Uia);
        assert_eq!(PerceptionMode::parse("hybrid"), PerceptionMode::Hybrid);
        assert_eq!(
            PerceptionMode::parse("vision_only"),
            PerceptionMode::VisionOnly
        );
        assert_eq!(PerceptionMode::parse("???"), PerceptionMode::Hybrid);
    }

    #[test]
    fn from_scan_ctx_carries_hints_and_cursor() {
        let scan = ScanContext {
            window_hints: vec!["explorador".into()],
        };
        let ctx = PerceptionContext::from_scan_ctx(&scan, PhysicalPoint { x: 50, y: 80 });
        assert_eq!(ctx.cursor.x, 50);
        assert_eq!(ctx.window_hints, vec!["explorador".to_string()]);
        assert!(ctx.max_windows() >= 1);
    }
}
