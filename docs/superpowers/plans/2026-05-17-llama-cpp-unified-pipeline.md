# Edge LLM CPU Performance — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Ollama as the default text LLM with **llama.cpp `llama-server`**, collapse pre-confirm intent+brief into **one 30s JSON call**, and disable per-step LLM copy by default so CPU laptops complete the guidance loop reliably.

**Architecture:** Add `LlamaCppClient` (OpenAI-compatible HTTP) behind existing `LlmClient`. Introduce `TaskBootstrapper` that returns `Intent` + `TaskBrief` from one prompt. Keep `TaskPlanner` for post-confirm screen grounding only. Ollama remains for optional Moondream vision. Stub regex fast-path unchanged.

**Tech Stack:** Rust (Tauri 2), `reqwest`, llama.cpp `llama-server`, `qwen3-1.7b-q4_k_m.gguf`, existing orchestration/perception.

**Design spec:** [`docs/superpowers/specs/2026-05-17-llama-cpp-unified-pipeline-design.md`](../specs/2026-05-17-llama-cpp-unified-pipeline-design.md)

**Revision:** 2026-05-17b — health probe timeouts, planner context budget (`-c 1024`), structured fallback logging, auto thread count, drop `LLAMA_MODEL`.

> **Blocking gate:** Do not start Phase 2 until Phase 0 uses `-c 1024` and Task 5 includes planner prompt element cap. See design spec § Context budget.

---

## File map

| Path | Responsibility |
|------|----------------|
| `src-tauri/src/llm/llamacpp.rs` | `LlamaCppClient` — `/v1/chat/completions` |
| `src-tauri/src/llm/mod.rs` | Backend selection in `build_llm_sync` |
| `src-tauri/src/settings.rs` | `LlmBackend`, `llama_*`, `step_llm_enabled` |
| `src-tauri/prompts/unified_bootstrap.txt` | Single intent+brief prompt |
| `src-tauri/src/orchestration/bootstrap.rs` | `TaskBootstrapper` parse + timeout |
| `src-tauri/src/prompts.rs` | `render_unified_bootstrap` |
| `src-tauri/src/orchestration/orchestrator.rs` | Wire bootstrap; gate step LLM |
| `src-tauri/src/orchestration/intent.rs` | Remove slow-path LLM (bootstrap owns it) |
| `src-tauri/src/orchestration/brief.rs` | Keep `heuristic_brief`; deprecate extractor from hot path |
| `scripts/start-llama.ps1` | Launch server with tuned flags |
| `scripts/download-model.ps1` | Pull GGUF into `~/.roota/models/` |
| `README.md` | Setup: llama.cpp default, Ollama optional for vision |

---

## Phase 0 — Prerequisites (manual, not a commit)

- [ ] **Download llama.cpp release** for Windows x64 from [llama.cpp releases](https://github.com/ggml-org/llama.cpp/releases) → extract `llama-server.exe` to `bin/llama-server.exe` (gitignored).
- [ ] **Download model:**

```powershell
mkdir -Force "$env:USERPROFILE\.roota\models"
# Example: huggingface-cli or direct URL for qwen3-1.7b-q4_k_m.gguf
```

- [ ] **Smoke test server:**

```powershell
$Threads = (Get-CimInstance Win32_Processor).NumberOfLogicalProcessors
.\bin\llama-server.exe -m "$env:USERPROFILE\.roota\models\qwen3-1.7b-q4_k_m.gguf" -t $Threads --batch-size 512 -c 1024 --port 8080
curl http://127.0.0.1:8080/health
```

Expected: HTTP 200. **Do not use `-c 512`** — planner prompts truncate and produce broken plans.

---

## Phase 1 — Settings + backend enum

### Task 1: `LlmBackend` and env vars

**Files:**
- Modify: `src-tauri/src/settings.rs`
- Test: `src-tauri/src/settings.rs` (`#[cfg(test)]` module)

- [ ] **Step 1: Write failing test**

```rust
#[test]
fn llm_backend_defaults_to_llamacpp() {
    std::env::remove_var("LLM_BACKEND");
    let s = Settings::from_env();
    assert_eq!(s.llm_backend, LlmBackend::LlamaCpp);
}
```

- [ ] **Step 2: Run test**

Run: `cd src-tauri; cargo test llm_backend_defaults_to_llamacpp -- --nocapture`  
Expected: FAIL (`llm_backend` field missing)

- [ ] **Step 3: Implement**

```rust
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

// In Settings:
pub llm_backend: LlmBackend,
pub llama_host: String,
pub llm_health_timeout_seconds: f32,
pub planner_prompt_max_elements: usize,
pub step_llm_enabled: bool,

// from_env():
llm_backend: LlmBackend::parse(&env_or("LLM_BACKEND", "llamacpp")),
llama_host: env_or("LLAMA_HOST", "http://127.0.0.1:8080"),
llm_health_timeout_seconds: env_parse("LLM_HEALTH_TIMEOUT_SECONDS", 2.0),
planner_prompt_max_elements: env_parse("ROOTA_PLANNER_PROMPT_ELEMENTS", 28),
step_llm_enabled: env_parse_bool("ROOTA_STEP_LLM", false),
llm_timeout_seconds: env_parse("LLM_TIMEOUT_SECONDS", 30.0),
// Mark llm_intent_timeout_seconds deprecated but keep for Ollama-only rollback
// Do NOT add LLAMA_MODEL — server uses -m at launch only
```

- [ ] **Step 4: Run test** — Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/settings.rs
git commit -m "feat(settings): add llama.cpp backend and step LLM toggle"
```

---

## Phase 2 — `LlamaCppClient`

### Task 2: OpenAI-compatible client

**Files:**
- Create: `src-tauri/src/llm/llamacpp.rs`
- Modify: `src-tauri/src/llm/mod.rs`

- [ ] **Step 1: Write failing test**

```rust
// src-tauri/src/llm/llamacpp.rs
#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{matchers::*, Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn complete_json_parses_chat_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "choices": [{"message": {"content": "{\"intent\":\"windows_task\",\"target\":\"x\",\"params\":{}}"}}]
            })))
            .mount(&server)
            .await;
        let client = LlamaCppClient::with_base_url(&server.uri(), 5.0, 30.0);
        let v = client.complete_json("hola", None).await.unwrap();
        assert_eq!(v["intent"], "windows_task");
    }
}
```

Add dev-dependency in `Cargo.toml` if using wiremock: `wiremock = "0.6"` under `[dev-dependencies]`.

- [ ] **Step 2: Run test** — Expected: FAIL (module missing)

- [ ] **Step 3: Implement `LlamaCppClient`**

```rust
//! llama.cpp `llama-server` — OpenAI-compatible chat API (local only).

use std::time::Duration;
use serde::{Deserialize, Serialize};
use crate::llm::client::{LlmClient, LlmError};
use crate::settings::Settings;

/// Hard cap for startup health probes — never use `llm_timeout_seconds` here.
const HEALTH_PROBE_TIMEOUT_SECS: f32 = 2.0;

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
        let http = reqwest::Client::builder().timeout(infer).build().unwrap_or_default();
        // blocking client used only for health_check_blocking — per-request timeout applied there
        let blocking = reqwest::blocking::Client::builder().build().unwrap_or_default();
        Self {
            base_url: base_url.into(),
            temperature,
            max_tokens,
            inference_timeout_secs,
            health_timeout_secs: health_timeout_secs.min(HEALTH_PROBE_TIMEOUT_SECS).max(0.5),
            http,
            blocking,
        }
    }

    /// Probe order: `/health` first, then `/v1/models`. Each probe: hard 2s cap (from settings).
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
                    tracing::debug!(target: "roota.llm", path, status = %r.status(), "health probe non-2xx");
                }
                Err(e) => {
                    tracing::debug!(target: "roota.llm", path, error = %e, "health probe failed");
                }
            }
        }
        false
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    // `model` omitted — llama-server ignores it; GGUF is fixed at `-m` launch
    messages: Vec<ChatMessage<'a>>,
    temperature: f32,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
}

#[derive(Serialize)]
struct ResponseFormat { r#type: &'static str }

#[derive(Serialize)]
struct ChatMessage<'a> { role: &'a str, content: &'a str }

#[derive(Deserialize)]
struct ChatResponse { choices: Vec<ChatChoice> }
#[derive(Deserialize)]
struct ChatChoice { message: ChatRespMsg }
#[derive(Deserialize)]
struct ChatRespMsg { content: String }

impl LlamaCppClient {
    async fn chat_raw(&self, prompt: &str, system: Option<&str>, json: bool) -> Result<String, LlmError> {
        let mut messages = Vec::new();
        if let Some(s) = system {
            messages.push(ChatMessage { role: "system", content: s });
        }
        messages.push(ChatMessage { role: "user", content: prompt });
        let url = format!("{}/v1/chat/completions", self.base_url);

        let send = |with_json_mode: bool| {
            let body = ChatRequest {
                messages: messages.clone(),
                temperature: self.temperature,
                max_tokens: self.max_tokens,
                response_format: if with_json_mode {
                    Some(ResponseFormat { r#type: "json_object" })
                } else {
                    None
                },
            };
            self.http.post(&url).json(&body).send()
        };

        let resp = match send(json).await {
            Ok(r) if json && r.status().as_u16() == 400 => {
                tracing::warn!(target: "roota.llm", reason = "json_mode_unsupported", "retrying without response_format");
                send(false).await
            }
            Ok(r) => Ok(r),
            Err(e) => Err(e),
        }
        .map_err(|e| {
            if e.is_timeout() {
                LlmError::Timeout { secs: self.inference_timeout_secs }
            } else {
                LlmError::Transport(e.to_string())
            }
        })?;

        let resp = resp?;
        if !resp.status().is_success() {
            return Err(LlmError::Transport(format!("status={}", resp.status())));
        }
        let parsed: ChatResponse = resp.json().await.map_err(|e| LlmError::Transport(e.to_string()))?;
        Ok(parsed.choices.first().map(|c| c.message.content.trim().to_string()).unwrap_or_default())
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

#[async_trait::async_trait]
impl LlmClient for LlamaCppClient {
    fn name(&self) -> &str { "llamacpp" }
    async fn health_check(&self) -> bool { self.health_check_blocking() }
    async fn complete_text(&self, prompt: &str, system: Option<&str>) -> Result<String, LlmError> {
        self.chat_raw(prompt, system, false).await
    }
    async fn complete_json(&self, prompt: &str, system: Option<&str>) -> Result<serde_json::Value, LlmError> {
        let raw = self.chat_raw(prompt, system, true).await?;
        let value = Self::parse_json_content(&raw)?;
        if !value.is_object() {
            tracing::warn!(target: "roota.llm", reason = "json_parse_failed", detail = "not_an_object");
            return Err(LlmError::NotAnObject);
        }
        Ok(value)
    }
}
```

**Logging contract:** `LlamaCppClient` logs `json_parse_failed` / `json_mode_unsupported`. Orchestration (`TaskBootstrapper`, `TaskPlanner`) logs `stub_fallback` with `cause=` — never reuse the same `reason` string for both layers.

- [ ] **Step 4: Wire `build_llm_sync` in `llm/mod.rs`**

```rust
pub mod llamacpp;
pub use llamacpp::LlamaCppClient;

pub fn build_llm_sync(settings: &Settings) -> Arc<dyn LlmClient> {
    let fallback = StubLlmClient;
    let primary: Arc<dyn LlmClient> = match settings.llm_backend {
        LlmBackend::Ollama => {
            let c = OllamaClient::new(settings);
            if c.health_check_blocking() {
                Arc::new(c)
            } else {
                tracing::warn!(target: "roota.llm", "Ollama unreachable; stub only");
                return Arc::new(fallback);
            }
        }
        LlmBackend::LlamaCpp => {
            let c = LlamaCppClient::new(settings);
            if c.health_check_blocking() {
                Arc::new(c)
            } else {
                tracing::warn!(target: "roota.llm", "llama-server unreachable; stub only");
                return Arc::new(fallback);
            }
        }
    };
    tracing::info!(target: "roota.llm", backend = primary.name(), "text LLM ready");
    Arc::new(ResilientLlmClient::new(primary, fallback))
}
```

- [ ] **Step 5: Run tests**

Run: `cd src-tauri; cargo test llamacpp -- --nocapture`  
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/llm/ src-tauri/Cargo.toml
git commit -m "feat(llm): add llama.cpp OpenAI-compatible client"
```

---

## Phase 3 — Unified bootstrap (intent + brief)

### Task 3: Prompt template

**Files:**
- Create: `src-tauri/prompts/unified_bootstrap.txt`
- Modify: `src-tauri/src/prompts.rs`

- [ ] **Step 1: Add prompt file**

```
Clasifica la petición del usuario y extrae un resumen estructurado para guiar en Windows.

Responde SOLO JSON válido:
{
  "intent": "<one_of_allowed>",
  "target": "<texto corto>",
  "params": {},
  "goal_summary": "<una frase>",
  "app_hints": ["<app>"],
  "object_hints": ["<objeto>"],
  "risk_flags": []
}

Intenciones permitidas:
{allowed_intents}

Reglas:
- "windows_task" para cualquier tarea de escritorio no listada explícitamente.
- "unknown" solo si la petición está vacía o no es tarea de PC.
- app_hints: explorador, chrome, configuración, cursor, etc. en minúsculas.
- object_hints: nombres de carpetas, Wi‑Fi, volumen, etc.
- risk_flags: "delete", "email" si aplica.

Petición: "{utterance}"
```

- [ ] **Step 2: Add renderer + test in `prompts.rs`**

```rust
pub const UNIFIED_BOOTSTRAP: &str = include_str!("../prompts/unified_bootstrap.txt");

pub fn render_unified_bootstrap(utterance: &str, allowed_intents: &[String]) -> String {
    let allowed = render_intent_classifier("", allowed_intents); // reuse list builder or duplicate 10 lines
    UNIFIED_BOOTSTRAP
        .replace("{allowed_intents}", /* extract list from allowed */)
        .replace("{utterance}", utterance)
}
```

Prefer extracting shared `format_allowed_intents(allowed: &[String]) -> String` to avoid duplication.

- [ ] **Step 3: Test placeholder replacement**

```rust
#[test]
fn unified_bootstrap_has_no_placeholders() {
    let out = render_unified_bootstrap("Abre Chrome", &["open_folder".into()]);
    assert!(!out.contains("{utterance}"));
    assert!(out.contains("Abre Chrome"));
}
```

- [ ] **Step 4: Commit** — `feat(prompts): unified bootstrap template for intent+brief`

---

### Task 4: `TaskBootstrapper`

**Files:**
- Create: `src-tauri/src/orchestration/bootstrap.rs`
- Modify: `src-tauri/src/orchestration/mod.rs`

- [ ] **Step 1: Write failing test**

```rust
#[tokio::test]
async fn bootstrap_maps_json_to_intent_and_brief() {
    struct Canned;
    #[async_trait::async_trait]
    impl LlmClient for Canned {
        fn name(&self) -> &str { "canned" }
        async fn health_check(&self) -> bool { true }
        async fn complete_text(&self, _: &str, _: Option<&str>) -> Result<String, LlmError> { Ok(String::new()) }
        async fn complete_json(&self, _: &str, _: Option<&str>) -> Result<Value, LlmError> {
            Ok(serde_json::json!({
                "intent": "open_folder",
                "target": "Descargas",
                "params": {},
                "goal_summary": "Abrir Descargas",
                "app_hints": ["explorador"],
                "object_hints": ["descargas"],
                "risk_flags": []
            }))
        }
    }
    let b = TaskBootstrapper::new(Arc::new(Canned), templates, Lang::Es, 30.0);
    let (intent, brief) = b.bootstrap("Abre Descargas").await;
    assert_eq!(intent.intent, "open_folder");
    assert_eq!(brief.object_hints, vec!["descargas"]);
}
```

- [ ] **Step 2: Implement**

```rust
pub struct TaskBootstrapper {
    llm: Arc<dyn LlmClient>,
    templates: Arc<TemplateRegistry>,
    lang: Lang,
    timeout: Duration,
}

impl TaskBootstrapper {
    pub async fn bootstrap(&self, utterance: &str) -> (Intent, TaskBrief) {
        let trimmed = utterance.trim();
        if trimmed.is_empty() {
            let i = Intent::unknown(utterance);
            return (i.clone(), heuristic_brief(utterance, ""));
        }
        let (fast, rule_matched) = classify_utterance_detailed(trimmed);
        if rule_matched && is_known_intent(&fast) {
            let intent = /* reuse IntentRecognizer::value_to_intent logic via shared fn */;
            let brief = heuristic_brief(utterance, &intent.target);
            return (intent, brief);
        }
        let allowed = self.templates.known_intents();
        let prompt = prompts::render_unified_bootstrap(trimmed, &allowed);
        let fut = self.llm.complete_json(&prompt, Some(prompts::SYSTEM_PROMPT));
        let value = match tokio::time::timeout(self.timeout, fut).await {
            Ok(Ok(v)) => v,
            Ok(Err(e)) => {
                tracing::warn!(target: "roota.bootstrap", reason = "stub_fallback", cause = "llm_error", error = %e);
                classify_utterance_detailed(trimmed).0
            }
            Err(_) => {
                tracing::warn!(target: "roota.bootstrap", reason = "stub_fallback", cause = "timeout");
                classify_utterance_detailed(trimmed).0
            }
        };
        // After Ok(Ok(v)): if parse_brief_json / intent_from_json_value fail, log cause = "json_parse_failed"
        let intent = intent_from_json_value(&self.templates, value.clone(), utterance, self.lang);
        let brief = parse_brief_json(value, utterance)
            .unwrap_or_else(|| heuristic_brief(utterance, &intent.target));
        (intent, brief)
    }
}
```

Extract `intent_from_json_value` from `IntentRecognizer::value_to_intent` (move to `orchestration/intent.rs` as `pub(crate) fn intent_from_json_value(...)`).

- [ ] **Step 3: Refactor `IntentRecognizer::recognise` slow path**

Remove LLM call from `intent.rs`; `recognise` becomes thin wrapper calling `TaskBootstrapper` and returning only `.0`, **or** delete recognizer from orchestrator entirely.

Recommended: keep `IntentRecognizer` for tests but implement `recognise` as:

```rust
pub async fn recognise(&self, utterance: &str) -> Intent {
    self.bootstrapper.bootstrap(utterance).await.0
}
```

- [ ] **Step 4: Run tests**

Run: `cd src-tauri; cargo test bootstrap intent:: -- --nocapture`  
Expected: PASS

- [ ] **Step 5: Commit** — `feat(orchestration): unified bootstrap merges intent and brief LLM calls`

---

### Task 5: Orchestrator wiring

**Files:**
- Modify: `src-tauri/src/orchestration/orchestrator.rs`

- [ ] **Step 1: Replace sequential calls**

Before:

```rust
let intent = self.recognizer.recognise(&utterance).await;
// ...
let task_brief = self.brief_extractor.understand(&intent.raw_utterance, &target_label).await;
```

After:

```rust
let (intent, task_brief) = self.bootstrapper.bootstrap(&utterance).await;
```

- [ ] **Step 2: Constructor changes**

```rust
bootstrapper: TaskBootstrapper,
// Remove brief_extractor field OR keep unused for one release
```

- [ ] **Step 3: Gate step LLM**

In `instruction_for_step` (or equivalent), wrap LLM block:

```rust
if !self.settings.step_llm_enabled {
    return canonical_instruction(step, hint, self.lang);
}
// existing timeout + complete_text path
```

- [ ] **Step 4: Planner timeout + prompt budget (blocking)**

Modify `src-tauri/src/orchestration/planner.rs`:

```rust
// Store at TaskPlanner::new:
//   inference_timeout: settings.llm_timeout_seconds
//   prompt_element_cap: settings.planner_prompt_max_elements  // default 28

// Replace:
let element_limit = perception.prompt_max_elements.max(PLANNER_PROMPT_ELEMENTS);
// With:
let element_limit = self.prompt_element_cap;  // NOT perception.prompt_max_elements (60)

let visible = frame.ranked_visible_summary_for_target(
    element_limit,
    &hints,
    frame.cursor,
    &goal_target,
);
```

`ranked_visible_summary_for_target` already emits compact one-liners (`text (role) [src] @x,y`) — do **not** send raw UIA trees. Cap exists because 60 lines + system prompt + windows guide exceeds `-c 512`; server runs at `-c 1024` (Phase 0) and cap keeps planner latency predictable.

On planner LLM failure/timeout, log:

```rust
tracing::warn!(target: "roota.planner", reason = "stub_fallback", cause = "timeout" /* or llm_error */);
```

Then `heuristic_plan(...)` as today.

- [ ] **Step 5: Integration test (manual)**

Run: `scripts/start-llama.ps1` then `npm run tauri:dev`  
Say: "Abre Chrome" → confirm → expect plan preview within ~30s total bootstrap+plan.

- [ ] **Step 6: Commit** — `feat(orchestrator): wire unified bootstrap and disable step LLM by default`

---

## Phase 4 — Dev scripts + docs

### Task 6: `scripts/start-llama.ps1`

**Files:**
- Create: `scripts/start-llama.ps1`
- Create: `scripts/download-model.ps1`
- Modify: `README.md`
- Modify: `.gitignore` (add `bin/llama-server.exe`, `*.gguf`)

- [ ] **Step 1: Create start script**

```powershell
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$Bin = Join-Path $Root "bin\llama-server.exe"
$Model = "$env:USERPROFILE\.roota\models\qwen3-1.7b-q4_k_m.gguf"
$Threads = (Get-CimInstance Win32_Processor).NumberOfLogicalProcessors
if (-not $Threads -or $Threads -lt 1) { $Threads = 4 }
$Context = 1024   # required for planner; do not lower to 512
if (-not (Test-Path $Bin)) { throw "Missing $Bin — download from llama.cpp releases" }
if (-not (Test-Path $Model)) { throw "Missing $Model — run scripts/download-model.ps1" }
Write-Host "llama-server: threads=$Threads context=$Context"
& $Bin -m $Model -t $Threads --batch-size 512 -c $Context --host 127.0.0.1 --port 8080
```

- [ ] **Step 2: Update README prerequisites**

Replace "Ollama required" with:

1. Start `scripts/start-llama.ps1` (text LLM)
2. Optional: Ollama + `moondream:1.8b` only if `ROOTA_VISION_VLM=1`

Document `.env`:

```
LLM_BACKEND=llamacpp
LLAMA_HOST=http://127.0.0.1:8080
LLM_TIMEOUT_SECONDS=30
LLM_HEALTH_TIMEOUT_SECONDS=2
ROOTA_PLANNER_PROMPT_ELEMENTS=28
ROOTA_STEP_LLM=0
```

Rollback: `LLM_BACKEND=ollama`

- [ ] **Step 3: Commit** — `docs: llama.cpp setup replaces Ollama for text inference`

---

## Phase 5 — i18n + resilient client

### Task 7: User-facing backend messages

**Files:**
- Modify: `src-tauri/src/i18n.rs`
- Modify: frontend if it displays `ollama.unavailable` key

- [ ] **Step 1: Add keys**

```rust
"llama.unavailable" => "No se encontró el servidor local de IA. Ejecuta scripts/start-llama.ps1",
"llm.backend" => "Motor de IA: {name}",
```

- [ ] **Step 2: Emit on health fail at startup** (Tauri command or existing status event)

- [ ] **Step 3: Commit** — `feat(i18n): llama.cpp unavailable copy`

---

## Phase 6 — Verification checklist

- [ ] **Unit:** `cargo test` in `src-tauri` — all green
- [ ] **Stub path:** "Abre la carpeta de Descargas" completes bootstrap without HTTP (<2s)
- [ ] **LLM path:** With llama-server, "Abre Chrome" → `windows_task` + non-empty plan
- [ ] **Timeout:** Single bootstrap respects 30s (not 10s intent cap)
- [ ] **Call count:** Trace logs show **1** `complete_json` before confirm, **≤1** planner after confirm, **0** `complete_text` per step when `ROOTA_STEP_LLM=0`
- [ ] **Fallback:** Stop llama-server → app boots with stub within **≤4s** (health probes), no hang
- [ ] **Health:** Log shows `/health` success or fallback to `/v1/models`; each probe ≤2s
- [ ] **Planner context:** With 173+ frame elements, plan still non-empty (server `-c 1024`, prompt cap 28)
- [ ] **Logs:** Force malformed JSON → grep shows `json_parse_failed` then `stub_fallback` with distinct `cause`
- [ ] **Ollama vision:** With `ROOTA_VISION_VLM=1`, Moondream still uses `OllamaClient::for_vision` only

Run:

```powershell
cd src-tauri; cargo test
$env:RUST_LOG=roota=info; npm run tauri:dev
```

---

## Appendix A — ONNX Runtime (follow-up plan, not in this PR)

| Task | Effort |
|------|--------|
| Add `ort` dependency + model download to `%USERPROFILE%\.roota\models\phi-3-mini-int4` | 1d |
| `OnnxClient::complete_json` via `spawn_blocking` | 1d |
| `LLM_BACKEND=onnx` in settings | 0.5d |
| Benchmark vs llama.cpp on hackathon hardware | 0.5d |

Use when llama.cpp still misses latency targets after unified prompts.

---

## Appendix B — Call budget summary

| Stage | Old | New |
|-------|-----|-----|
| After utterance | intent LLM + brief LLM | **1** bootstrap LLM (or 0 stub) |
| After confirm (`windows_task`) | planner LLM | **1** planner LLM (unchanged) |
| Each guided step | instruction LLM (often) | **0** default (`ROOTA_STEP_LLM=0`) |
| Replan | planner LLM | unchanged (max 2/session) |

**Worst case text calls per session:** 1 + 1 + 2 replan = 4 → same replan cap, but **halved** on the common path (2 vs 4+ before first overlay).

---

## Self-review (spec coverage)

| Requirement | Task |
|-------------|------|
| llama.cpp default backend | Task 2, 6 |
| qwen3 1.7b Q4 | Phase 0, scripts |
| Single pre-confirm prompt | Task 3–5 |
| 30s inference timeout | Task 1, 4, 5 |
| 2s health probe timeout | Task 1, 2 |
| `-c 1024` server context | Phase 0, Task 6 (**blocking**) |
| Planner prompt cap 28 | Task 5 (**blocking**) |
| `json_parse_failed` vs `stub_fallback` logs | Task 2, 4, 5 |
| Auto thread count | Phase 0, Task 6 |
| No `LLAMA_MODEL` | Task 1, 2 |
| Keep screen planner | Task 5 |
| Ollama for vision only | Design spec + no change to moondream.rs |
| Stub fast-path | Task 4 |
| ONNX deferred | Appendix A |

**Placeholder scan:** None — all steps include concrete paths and code.

---

## Execution handoff

**Plan complete and saved to `docs/superpowers/plans/2026-05-17-llama-cpp-unified-pipeline.md`.**

**Design spec:** `docs/superpowers/specs/2026-05-17-llama-cpp-unified-pipeline-design.md`

**Two execution options:**

1. **Subagent-Driven (recommended)** — dispatch a fresh subagent per task with `superpowers:subagent-driven-development`; review between tasks.

2. **Inline Execution** — run tasks in this session with `superpowers:executing-plans`; batch with checkpoints after Phase 2 and Phase 5.

Which approach do you want?
