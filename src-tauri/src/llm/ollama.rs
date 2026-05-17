//! Ollama-backed LLM client. HTTP-only, never reaches the public network.

use std::time::Duration;

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::{Deserialize, Serialize};

use crate::llm::client::{LlmClient, LlmError};
use crate::settings::Settings;

/// Keep Moondream loaded between perception ticks (Ollama `keep_alive`).
const VISION_KEEP_ALIVE: &str = "15m";

const HEALTH_PROBE_MAX_SECS: f32 = 2.0;

#[derive(Debug, Clone)]
pub struct OllamaClient {
    base_url: String,
    model: String,
    temperature: f32,
    max_tokens: u32,
    timeout_secs: f32,
    health_timeout_secs: f32,
    http: reqwest::Client,
    blocking: reqwest::blocking::Client,
}

impl OllamaClient {
    pub fn new(settings: &Settings) -> Self {
        Self::with_model(
            settings,
            settings.llm_model.clone(),
            settings.llm_timeout_seconds,
            settings.llm_temperature,
            settings.llm_max_tokens,
        )
    }

    /// Vision-tuned client (Moondream) with JSON output and a generous timeout.
    pub fn for_vision(settings: &Settings) -> Self {
        Self::with_model(
            settings,
            settings.perception.vision_model.clone(),
            settings.perception.vision_timeout_secs,
            0.1,
            settings.perception.vision_max_tokens,
        )
    }

    /// Vision planner client (hybrid-vision-planner fallback).
    pub fn for_vision_planner(settings: &Settings) -> Self {
        Self::with_model(
            settings,
            settings.perception.vision_planner_model.clone(),
            settings.perception.vision_planner_timeout_secs,
            0.1,
            settings.perception.vision_max_tokens,
        )
    }

    fn with_model(
        settings: &Settings,
        model: String,
        timeout_secs: f32,
        temperature: f32,
        max_tokens: u32,
    ) -> Self {
        let timeout = Duration::from_secs_f32(timeout_secs);
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_default();
        let blocking = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_default();
        let health_timeout_secs = settings
            .llm_health_timeout_seconds
            .min(HEALTH_PROBE_MAX_SECS)
            .max(0.5);
        OllamaClient {
            base_url: settings.ollama_host.trim_end_matches('/').to_string(),
            model,
            temperature,
            max_tokens,
            timeout_secs,
            health_timeout_secs,
            http,
            blocking,
        }
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn timeout_secs(&self) -> f32 {
        self.timeout_secs
    }

    fn build_messages<'a>(&self, prompt: &'a str, system: Option<&'a str>) -> Vec<ChatMsg<'a>> {
        let mut msgs = Vec::with_capacity(2);
        if let Some(s) = system {
            msgs.push(ChatMsg {
                role: "system",
                content: s,
                images: None,
            });
        }
        msgs.push(ChatMsg {
            role: "user",
            content: prompt,
            images: None,
        });
        msgs
    }

    async fn chat<T: for<'de> Deserialize<'de>>(
        &self,
        prompt: &str,
        system: Option<&str>,
        json_format: bool,
    ) -> Result<T, LlmError> {
        let body = ChatRequest {
            model: &self.model,
            messages: self.build_messages(prompt, system),
            stream: false,
            format: if json_format { Some("json") } else { None },
            keep_alive: None,
            options: ChatOptions {
                temperature: self.temperature,
                num_predict: self.max_tokens,
            },
        };
        self.post_chat_async(&body).await
    }

    fn post_chat_blocking<T: for<'de> Deserialize<'de>>(
        &self,
        body: &ChatRequest<'_>,
    ) -> Result<T, LlmError> {
        let url = format!("{}/api/chat", self.base_url);
        let resp = self.blocking.post(url).json(body).send().map_err(|e| {
            if e.is_timeout() {
                LlmError::Timeout {
                    secs: self.timeout_secs,
                }
            } else {
                LlmError::Transport(e.to_string())
            }
        })?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().unwrap_or_default();
            return Err(LlmError::Transport(format!("status={status} body={text}")));
        }
        resp.json::<T>()
            .map_err(|e| LlmError::Transport(e.to_string()))
    }

    async fn post_chat_async<T: for<'de> Deserialize<'de>>(
        &self,
        body: &ChatRequest<'_>,
    ) -> Result<T, LlmError> {
        let url = format!("{}/api/chat", self.base_url);
        let resp = self.http.post(url).json(body).send().await.map_err(|e| {
            if e.is_timeout() {
                LlmError::Timeout {
                    secs: self.timeout_secs,
                }
            } else {
                LlmError::Transport(e.to_string())
            }
        })?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(LlmError::Transport(format!("status={status} body={text}")));
        }
        resp.json::<T>()
            .await
            .map_err(|e| LlmError::Transport(e.to_string()))
    }

    /// Multimodal JSON completion — blocking, for use inside `spawn_blocking`.
    pub fn complete_vision_json_blocking(
        &self,
        prompt: &str,
        png_bytes: &[u8],
    ) -> Result<serde_json::Value, LlmError> {
        self.complete_vision_json_blocking_with_keep_alive(prompt, png_bytes, Some(VISION_KEEP_ALIVE))
    }

    fn complete_vision_json_blocking_with_keep_alive(
        &self,
        prompt: &str,
        png_bytes: &[u8],
        keep_alive: Option<&str>,
    ) -> Result<serde_json::Value, LlmError> {
        let b64 = B64.encode(png_bytes);
        let body = ChatRequest {
            model: &self.model,
            messages: vec![ChatMsg {
                role: "user",
                content: prompt,
                images: Some(vec![b64]),
            }],
            stream: false,
            format: Some("json"),
            keep_alive,
            options: ChatOptions {
                temperature: self.temperature,
                num_predict: self.max_tokens,
            },
        };
        let resp: ChatResponse = self.post_chat_blocking(&body)?;
        let raw = resp.message.content.trim();
        let value: serde_json::Value =
            serde_json::from_str(raw).map_err(|_| LlmError::InvalidJson(raw.to_string()))?;
        if !value.is_object() {
            return Err(LlmError::NotAnObject);
        }
        Ok(value)
    }

    /// Load the vision model into Ollama so the first user capture is not cold.
    pub fn warmup_vision_blocking(&self) -> Result<(), LlmError> {
        const TINY_PNG: &[u8] = &[
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00,
            0x00, 0x90, 0x77, 0x53, 0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, 0x54, 0x08,
            0xd7, 0x63, 0xf8, 0xcf, 0xc0, 0x00, 0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xdd, 0x8d,
            0xb4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
        ];
        let _ = self.complete_vision_json_blocking_with_keep_alive(
            "Return only: {\"elements\":[]}",
            TINY_PNG,
            Some(VISION_KEEP_ALIVE),
        )?;
        Ok(())
    }

    /// Sync reachability probe — safe to call from Tauri `.setup()` (no `block_on`).
    pub fn health_check_blocking(&self) -> bool {
        let url = format!("{}/api/tags", self.base_url);
        let probe = Duration::from_secs_f32(self.health_timeout_secs);
        match self.blocking.get(&url).timeout(probe).send() {
            Ok(resp) if resp.status().is_success() => true,
            Ok(resp) => {
                tracing::debug!(
                    target: "roota.llm",
                    status = %resp.status(),
                    "Ollama health probe non-2xx"
                );
                false
            }
            Err(e) => {
                tracing::debug!(target: "roota.llm", error = %e, "Ollama health probe failed");
                false
            }
        }
    }

    /// Returns true when the configured vision model appears in `ollama list`.
    pub fn vision_model_available(&self) -> bool {
        let url = format!("{}/api/tags", self.base_url);
        let Ok(resp) = self
            .blocking
            .get(&url)
            .timeout(Duration::from_secs(3))
            .send()
        else {
            return false;
        };
        let Ok(tags): Result<TagsResponse, _> = resp.json() else {
            return false;
        };
        let needle = self.model.to_lowercase();
        tags.models.iter().any(|m| {
            m.name.to_lowercase() == needle || m.name.to_lowercase().starts_with(&format!("{needle}:"))
        })
    }
}

#[async_trait::async_trait]
impl LlmClient for OllamaClient {
    fn name(&self) -> &str {
        "ollama"
    }

    async fn health_check(&self) -> bool {
        self.health_check_blocking()
    }

    async fn complete_text(&self, prompt: &str, system: Option<&str>) -> Result<String, LlmError> {
        let resp: ChatResponse = self.chat(prompt, system, false).await?;
        Ok(resp.message.content.trim().to_string())
    }

    async fn complete_json(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<serde_json::Value, LlmError> {
        let resp: ChatResponse = self.chat(prompt, system, true).await?;
        let raw = resp.message.content.trim();
        let value: serde_json::Value =
            serde_json::from_str(raw).map_err(|_| LlmError::InvalidJson(raw.to_string()))?;
        if !value.is_object() {
            return Err(LlmError::NotAnObject);
        }
        Ok(value)
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMsg<'a>>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    keep_alive: Option<&'a str>,
    options: ChatOptions,
}

#[derive(Serialize)]
struct ChatMsg<'a> {
    role: &'a str,
    content: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    images: Option<Vec<String>>,
}

#[derive(Serialize)]
struct ChatOptions {
    temperature: f32,
    num_predict: u32,
}

#[derive(Deserialize)]
struct ChatResponse {
    message: ChatRespMessage,
}

#[derive(Deserialize)]
struct ChatRespMessage {
    content: String,
}

#[derive(Deserialize)]
struct TagsResponse {
    models: Vec<ModelTag>,
}

#[derive(Deserialize)]
struct ModelTag {
    name: String,
}
