//! Application settings, loaded from environment variables with defaults
//! that mirror the prior `.env.example`.

use std::env;

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
}
