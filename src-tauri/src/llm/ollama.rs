//! Ollama-backed LLM client. HTTP-only, never reaches the public network.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::llm::client::{LlmClient, LlmError};
use crate::settings::Settings;

#[derive(Debug, Clone)]
pub struct OllamaClient {
    base_url: String,
    model: String,
    temperature: f32,
    max_tokens: u32,
    timeout_secs: f32,
    http: reqwest::Client,
}

impl OllamaClient {
    pub fn new(settings: &Settings) -> Self {
        let timeout_secs = settings.llm_timeout_seconds;
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs_f32(timeout_secs))
            .build()
            .unwrap_or_default();
        OllamaClient {
            base_url: settings.ollama_host.trim_end_matches('/').to_string(),
            model: settings.llm_model.clone(),
            temperature: settings.llm_temperature,
            max_tokens: settings.llm_max_tokens,
            timeout_secs,
            http,
        }
    }

    fn build_messages<'a>(&self, prompt: &'a str, system: Option<&'a str>) -> Vec<ChatMsg<'a>> {
        let mut msgs = Vec::with_capacity(2);
        if let Some(s) = system {
            msgs.push(ChatMsg {
                role: "system",
                content: s,
            });
        }
        msgs.push(ChatMsg {
            role: "user",
            content: prompt,
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
            options: ChatOptions {
                temperature: self.temperature,
                num_predict: self.max_tokens,
            },
        };
        let url = format!("{}/api/chat", self.base_url);
        let resp = self.http.post(url).json(&body).send().await.map_err(|e| {
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
}

#[async_trait::async_trait]
impl LlmClient for OllamaClient {
    fn name(&self) -> &str {
        "ollama"
    }

    async fn health_check(&self) -> bool {
        let url = format!("{}/api/tags", self.base_url);
        let timeout = Duration::from_secs(3);
        match self.http.get(url).timeout(timeout).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
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
    options: ChatOptions,
}

#[derive(Serialize)]
struct ChatMsg<'a> {
    role: &'a str,
    content: &'a str,
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
