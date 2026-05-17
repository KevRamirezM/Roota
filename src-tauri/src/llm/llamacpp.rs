//! llama.cpp `llama-server` — OpenAI-compatible chat API (local only).

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::llm::client::{LlmClient, LlmError};
use crate::settings::Settings;

/// Hard cap for startup health probes — never use inference timeout here.
const HEALTH_PROBE_MAX_SECS: f32 = 2.0;

#[derive(Clone)]
pub struct LlamaCppClient {
    base_url: String,
    temperature: f32,
    max_tokens: u32,
    inference_timeout_secs: f32,
    health_timeout_secs: f32,
    http: reqwest::Client,
    blocking: reqwest::blocking::Client,
}

impl LlamaCppClient {
    pub fn new(settings: &Settings) -> Self {
        Self::with_base_url(
            settings.llama_host.trim_end_matches('/'),
            settings.llm_health_timeout_seconds,
            settings.llm_timeout_seconds,
            settings.llm_temperature,
            settings.llm_max_tokens,
        )
    }

    pub fn with_base_url(
        base_url: &str,
        health_timeout_secs: f32,
        inference_timeout_secs: f32,
        temperature: f32,
        max_tokens: u32,
    ) -> Self {
        let infer = Duration::from_secs_f32(inference_timeout_secs);
        let http = reqwest::Client::builder()
            .timeout(infer)
            .build()
            .unwrap_or_default();
        let blocking = reqwest::blocking::Client::builder()
            .build()
            .unwrap_or_default();
        Self {
            base_url: base_url.into(),
            temperature,
            max_tokens,
            inference_timeout_secs,
            health_timeout_secs: health_timeout_secs
                .min(HEALTH_PROBE_MAX_SECS)
                .max(0.5),
            http,
            blocking,
        }
    }

    /// Probe order: `/health` first, then `/v1/models`. Each probe uses `health_timeout_secs`.
    pub fn health_check_blocking(&self) -> bool {
        let probe = Duration::from_secs_f32(self.health_timeout_secs);
        for path in ["/health", "/v1/models"] {
            let url = format!("{}{}", self.base_url, path);
            match self.blocking.get(&url).timeout(probe).send() {
                Ok(r) if r.status().is_success() => {
                    tracing::debug!(target: "roota.llm", path, "llama-server health ok");
                    return true;
                }
                Ok(r) => {
                    tracing::debug!(
                        target: "roota.llm",
                        path,
                        status = %r.status(),
                        "health probe non-2xx"
                    );
                }
                Err(e) => {
                    tracing::debug!(target: "roota.llm", path, error = %e, "health probe failed");
                }
            }
        }
        false
    }

    async fn chat_raw(
        &self,
        prompt: &str,
        system: Option<&str>,
        json: bool,
    ) -> Result<String, LlmError> {
        let mut messages = Vec::new();
        if let Some(s) = system {
            messages.push(ChatMessage {
                role: "system",
                content: s,
            });
        }
        messages.push(ChatMessage {
            role: "user",
            content: prompt,
        });
        let url = format!("{}/v1/chat/completions", self.base_url);

        let send = |with_json_mode: bool| {
            let body = ChatRequest {
                messages: messages.clone(),
                temperature: self.temperature,
                max_tokens: self.max_tokens,
                response_format: if with_json_mode {
                    Some(ResponseFormat {
                        r#type: "json_object",
                    })
                } else {
                    None
                },
            };
            self.http.post(&url).json(&body).send()
        };

        let resp = match send(json).await {
            Ok(r) if json && r.status().as_u16() == 400 => {
                tracing::warn!(
                    target: "roota.llm",
                    reason = "json_mode_unsupported",
                    "retrying without response_format"
                );
                send(false).await
            }
            Ok(r) => Ok(r),
            Err(e) => Err(e),
        }
        .map_err(|e| {
            if e.is_timeout() {
                LlmError::Timeout {
                    secs: self.inference_timeout_secs,
                }
            } else {
                LlmError::Transport(e.to_string())
            }
        })?;

        if !resp.status().is_success() {
            return Err(LlmError::Transport(format!("status={}", resp.status())));
        }
        let parsed: ChatResponse = resp
            .json()
            .await
            .map_err(|e| LlmError::Transport(e.to_string()))?;
        Ok(parsed
            .choices
            .first()
            .map(|c| c.message.content.trim().to_string())
            .unwrap_or_default())
    }

    fn parse_json_content(raw: &str) -> Result<serde_json::Value, LlmError> {
        serde_json::from_str(raw).map_err(|e| {
            let preview: String = raw.chars().take(200).collect();
            tracing::warn!(
                target: "roota.llm",
                reason = "json_parse_failed",
                error = %e,
                raw_preview = %preview,
                "model returned non-JSON content"
            );
            LlmError::InvalidJson(preview)
        })
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    messages: Vec<ChatMessage<'a>>,
    temperature: f32,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
}

#[derive(Serialize)]
struct ResponseFormat {
    r#type: &'static str,
}

#[derive(Serialize, Clone)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatRespMsg,
}

#[derive(Deserialize)]
struct ChatRespMsg {
    content: String,
}

#[async_trait::async_trait]
impl LlmClient for LlamaCppClient {
    fn name(&self) -> &str {
        "llamacpp"
    }

    async fn health_check(&self) -> bool {
        self.health_check_blocking()
    }

    async fn complete_text(&self, prompt: &str, system: Option<&str>) -> Result<String, LlmError> {
        self.chat_raw(prompt, system, false).await
    }

    async fn complete_json(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<serde_json::Value, LlmError> {
        let raw = self.chat_raw(prompt, system, true).await?;
        let value = Self::parse_json_content(&raw)?;
        if !value.is_object() {
            tracing::warn!(
                target: "roota.llm",
                reason = "json_parse_failed",
                detail = "not_an_object"
            );
            return Err(LlmError::NotAnObject);
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_json_content_rejects_garbage() {
        let err = LlamaCppClient::parse_json_content("not json").unwrap_err();
        assert!(matches!(err, LlmError::InvalidJson(_)));
    }

    #[test]
    fn parse_json_content_accepts_object() {
        let v = LlamaCppClient::parse_json_content(r#"{"intent":"windows_task"}"#).unwrap();
        assert_eq!(v["intent"], "windows_task");
    }

}
