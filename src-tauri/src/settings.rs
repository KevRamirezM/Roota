//! Application settings, loaded from environment variables with defaults
//! that mirror the prior `.env.example`.

use std::env;

use crate::perception::context::PerceptionMode;

#[derive(Debug, Clone)]
pub struct Settings {
    pub ollama_host: String,
    pub llm_model: String,
    pub llm_temperature: f32,
    pub llm_max_tokens: u32,
    pub llm_timeout_seconds: f32,
    /// Max wait for intent classification before stub fallback (keeps UI responsive).
    pub llm_intent_timeout_seconds: f32,
    pub ui_language: Lang,
    pub overlay_opacity: f32,
    pub overlay_fps: u32,
    pub log_level: String,
    pub safety_strict: bool,
    pub perception: PerceptionSettings,
}

/// Tunables for the perception pipeline (universal-windows-perception feature).
#[derive(Debug, Clone)]
pub struct PerceptionSettings {
    pub mode: PerceptionMode,
    /// Cap EnumWindows scanning *after* scoring (never before — see spec).
    pub max_windows: usize,
    /// Enable vision (Moondream) fallback in hybrid mode.
    pub vision_enabled: bool,
    /// Ollama vision model tag (e.g. moondream:1.8b).
    pub vision_model: String,
    /// Per-vision-request timeout in seconds.
    pub vision_timeout_secs: f32,
    /// Max long edge of captured bitmap before sending to the VLM.
    pub vision_max_edge: u32,
    /// When true, write debug PNG captures to temp (dev only).
    pub debug_capture: bool,
    pub ocr_language: String,
    pub capture_scale: f32,
    /// Below this many *interactable* elements in the primary window client
    /// rect, hybrid will run vision (if enabled & engine available).
    pub min_uia_elements: usize,
    /// LLM prompt size caps (resolved in spec).
    pub prompt_max_elements: usize,
    pub prompt_max_windows: usize,
}

impl Default for PerceptionSettings {
    fn default() -> Self {
        Self {
            mode: PerceptionMode::Hybrid,
            max_windows: 8,
            vision_enabled: true,
            vision_model: "moondream:1.8b".into(),
            vision_timeout_secs: 8.0,
            vision_max_edge: 768,
            debug_capture: false,
            ocr_language: "es".into(),
            capture_scale: 0.75,
            min_uia_elements: 3,
            prompt_max_elements: 40,
            prompt_max_windows: 3,
        }
    }
}

impl PerceptionSettings {
    pub fn from_env() -> Self {
        let default = Self::default();
        Self {
            mode: PerceptionMode::parse(&env_or("ROOTA_PERCEPTION_MODE", "hybrid")),
            max_windows: env_parse("ROOTA_MAX_WINDOWS", default.max_windows),
            vision_enabled: env_parse_bool("ROOTA_VISION_ENABLED", default.vision_enabled),
            vision_model: env_or("ROOTA_VISION_MODEL", &default.vision_model),
            vision_timeout_secs: env_parse(
                "ROOTA_VISION_TIMEOUT_SECS",
                default.vision_timeout_secs,
            ),
            vision_max_edge: env_parse("ROOTA_VISION_MAX_EDGE", default.vision_max_edge),
            debug_capture: env_parse_bool("ROOTA_DEBUG_CAPTURE", default.debug_capture),
            ocr_language: env_or("ROOTA_OCR_LANGUAGE", &default.ocr_language),
            capture_scale: env_parse("ROOTA_CAPTURE_SCALE", default.capture_scale),
            min_uia_elements: env_parse("ROOTA_MIN_UIA_ELEMENTS", default.min_uia_elements),
            prompt_max_elements: env_parse(
                "ROOTA_PROMPT_MAX_ELEMENTS",
                default.prompt_max_elements,
            ),
            prompt_max_windows: env_parse(
                "ROOTA_PROMPT_MAX_WINDOWS",
                default.prompt_max_windows,
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Es,
    En,
}

impl Lang {
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "en" | "english" => Lang::En,
            _ => Lang::Es,
        }
    }
}

impl Settings {
    pub fn from_env() -> Self {
        Settings {
            ollama_host: env_or("OLLAMA_HOST", "http://localhost:11434"),
            llm_model: env_or("LLM_MODEL", "qwen2.5:3b"),
            llm_temperature: env_parse("LLM_TEMPERATURE", 0.3),
            llm_max_tokens: env_parse("LLM_MAX_TOKENS", 512),
            llm_timeout_seconds: env_parse("LLM_TIMEOUT_SECONDS", 30.0),
            llm_intent_timeout_seconds: env_parse("LLM_INTENT_TIMEOUT_SECONDS", 10.0),
            ui_language: Lang::parse(&env_or("UI_LANGUAGE", "es")),
            overlay_opacity: env_parse("OVERLAY_OPACITY", 0.85),
            overlay_fps: env_parse("OVERLAY_FPS", 30),
            log_level: env_or("LOG_LEVEL", "info"),
            safety_strict: env_parse("SAFETY_STRICT", true),
            perception: PerceptionSettings::from_env(),
        }
    }
}

fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_parse<T: std::str::FromStr>(key: &str, default: T) -> T {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn env_parse_bool(key: &str, default: bool) -> bool {
    match env::var(key) {
        Ok(v) => match v.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => default,
        },
        Err(_) => default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_python_env_example() {
        let s = Settings::from_env();
        assert!(s.ollama_host.starts_with("http"));
        assert!(s.llm_model.contains("qwen"));
        assert!((0.0..=1.0).contains(&s.overlay_opacity));
    }

    #[test]
    fn parse_lang_falls_back_to_spanish() {
        assert_eq!(Lang::parse("es"), Lang::Es);
        assert_eq!(Lang::parse("EN"), Lang::En);
        assert_eq!(Lang::parse("zz"), Lang::Es);
    }

    #[test]
    fn perception_defaults_match_spec_table() {
        let s = PerceptionSettings::default();
        assert_eq!(s.mode, PerceptionMode::Hybrid);
        assert_eq!(s.max_windows, 8);
        assert!(s.vision_enabled);
        assert!(s.vision_model.contains("moondream"));
        assert!((s.vision_timeout_secs - 8.0).abs() < f32::EPSILON);
        assert_eq!(s.vision_max_edge, 768);
        assert_eq!(s.ocr_language, "es");
        assert!((s.capture_scale - 0.75).abs() < f32::EPSILON);
        assert_eq!(s.min_uia_elements, 3);
        assert_eq!(s.prompt_max_elements, 40);
        assert_eq!(s.prompt_max_windows, 3);
    }
}
