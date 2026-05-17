//! Application settings, loaded from environment variables with defaults
//! that mirror the prior `.env.example`.

use std::env;

use crate::perception::context::PerceptionMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmBackend {
    LlamaCpp,
    Ollama,
}

impl LlmBackend {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "ollama" => Self::Ollama,
            _ => Self::LlamaCpp,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub llm_backend: LlmBackend,
    pub llama_host: String,
    pub ollama_host: String,
    pub llm_model: String,
    pub llm_temperature: f32,
    pub llm_max_tokens: u32,
    pub llm_timeout_seconds: f32,
    /// Per health-probe HTTP call at startup (not inference).
    pub llm_health_timeout_seconds: f32,
    /// Max ranked element lines in the task planner prompt.
    pub planner_prompt_max_elements: usize,
    /// When true, per-step instruction copy may call the text LLM.
    pub step_llm_enabled: bool,
    /// Deprecated — bootstrap uses `llm_timeout_seconds`. Kept for Ollama rollback.
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
    /// Enable pixel fallback (Windows OCR) in hybrid mode.
    pub vision_enabled: bool,
    /// Enable slow Ollama VLM (Moondream) when OCR is sparse.
    pub vision_vlm_enabled: bool,
    /// Ollama vision model tag (e.g. moondream:1.8b).
    pub vision_model: String,
    /// Per-VLM-request timeout in seconds (Moondream cold start can exceed 8s).
    pub vision_timeout_secs: f32,
    /// Windows.Media.Ocr call timeout in seconds.
    pub ocr_timeout_secs: f32,
    /// Max tokens for vision JSON responses.
    pub vision_max_tokens: u32,
    /// Max long edge of captured bitmap before sending to the VLM.
    pub vision_max_edge: u32,
    /// Max long edge for Windows OCR (higher = sharper click boxes).
    pub ocr_max_edge: u32,
    /// When true, write debug PNG captures to temp (dev only).
    pub debug_capture: bool,
    pub ocr_language: String,
    /// Downscale for VLM captures (Moondream).
    pub capture_scale: f32,
    /// Downscale for OCR captures (1.0 = full resolution up to `ocr_max_edge`).
    pub ocr_capture_scale: f32,
    /// Pixels to expand the capture rect beyond the window (menus near edges).
    pub capture_margin_px: i32,
    /// Contrast-stretch captured bitmap before OCR.
    pub ocr_preprocess: bool,
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
            vision_vlm_enabled: false,
            vision_model: "moondream:1.8b".into(),
            vision_timeout_secs: 45.0,
            ocr_timeout_secs: 12.0,
            vision_max_tokens: 256,
            vision_max_edge: 512,
            ocr_max_edge: 1024,
            debug_capture: false,
            ocr_language: "es".into(),
            capture_scale: 0.75,
            ocr_capture_scale: 1.0,
            capture_margin_px: 32,
            ocr_preprocess: true,
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
            vision_vlm_enabled: env_parse_bool("ROOTA_VISION_VLM", default.vision_vlm_enabled),
            vision_model: env_or("ROOTA_VISION_MODEL", &default.vision_model),
            vision_timeout_secs: env_parse(
                "ROOTA_VISION_TIMEOUT_SECS",
                default.vision_timeout_secs,
            ),
            ocr_timeout_secs: env_parse("ROOTA_OCR_TIMEOUT_SECS", default.ocr_timeout_secs),
            vision_max_tokens: env_parse(
                "ROOTA_VISION_MAX_TOKENS",
                default.vision_max_tokens,
            ),
            vision_max_edge: env_parse("ROOTA_VISION_MAX_EDGE", default.vision_max_edge),
            ocr_max_edge: env_parse("ROOTA_OCR_MAX_EDGE", default.ocr_max_edge),
            debug_capture: env_parse_bool("ROOTA_DEBUG_CAPTURE", default.debug_capture),
            ocr_language: env_or("ROOTA_OCR_LANGUAGE", &default.ocr_language),
            capture_scale: env_parse("ROOTA_CAPTURE_SCALE", default.capture_scale),
            ocr_capture_scale: env_parse("ROOTA_OCR_CAPTURE_SCALE", default.ocr_capture_scale),
            capture_margin_px: env_parse("ROOTA_CAPTURE_MARGIN_PX", default.capture_margin_px),
            ocr_preprocess: env_parse_bool("ROOTA_OCR_PREPROCESS", default.ocr_preprocess),
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
            llm_backend: LlmBackend::parse(&env_or("LLM_BACKEND", "llamacpp")),
            llama_host: env_or("LLAMA_HOST", "http://127.0.0.1:8080"),
            ollama_host: env_or("OLLAMA_HOST", "http://localhost:11434"),
            llm_model: env_or("LLM_MODEL", "qwen3:1.7b"),
            llm_temperature: env_parse("LLM_TEMPERATURE", 0.3),
            llm_max_tokens: env_parse("LLM_MAX_TOKENS", 512),
            llm_timeout_seconds: env_parse("LLM_TIMEOUT_SECONDS", 30.0),
            llm_health_timeout_seconds: env_parse("LLM_HEALTH_TIMEOUT_SECONDS", 2.0),
            planner_prompt_max_elements: env_parse("ROOTA_PLANNER_PROMPT_ELEMENTS", 28),
            step_llm_enabled: env_parse_bool("ROOTA_STEP_LLM", false),
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
        assert!(s.llama_host.starts_with("http"));
        assert!(s.llm_model.contains("qwen"));
        assert!((0.0..=1.0).contains(&s.overlay_opacity));
    }

    #[test]
    fn llm_backend_defaults_to_llamacpp() {
        std::env::remove_var("LLM_BACKEND");
        let s = Settings::from_env();
        assert_eq!(s.llm_backend, LlmBackend::LlamaCpp);
    }

    #[test]
    fn planner_prompt_cap_defaults_to_28() {
        std::env::remove_var("ROOTA_PLANNER_PROMPT_ELEMENTS");
        let s = Settings::from_env();
        assert_eq!(s.planner_prompt_max_elements, 28);
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
        assert!(!s.vision_vlm_enabled);
        assert!(s.vision_model.contains("moondream"));
        assert!((s.vision_timeout_secs - 45.0).abs() < f32::EPSILON);
        assert_eq!(s.vision_max_tokens, 256);
        assert_eq!(s.vision_max_edge, 512);
        assert_eq!(s.ocr_max_edge, 1024);
        assert_eq!(s.ocr_language, "es");
        assert!((s.capture_scale - 0.75).abs() < f32::EPSILON);
        assert!((s.ocr_capture_scale - 1.0).abs() < f32::EPSILON);
        assert_eq!(s.capture_margin_px, 32);
        assert!(s.ocr_preprocess);
        assert_eq!(s.min_uia_elements, 3);
        assert_eq!(s.prompt_max_elements, 40);
        assert_eq!(s.prompt_max_windows, 3);
    }
}
