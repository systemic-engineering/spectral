//! Inference client — OpenAI-compatible chat completions.
//!
//! Both Ollama (local) and cloud providers (OpenRouter, OpenAI, Anthropic, Google)
//! speak the same `/v1/chat/completions` JSON schema. One struct, one function.
//!
//! Apache-2.0. Infrastructure, not runtime.

use std::collections::HashMap;
use std::fmt;

/// A provider-agnostic inference endpoint.
#[derive(Debug, Clone)]
pub struct InferenceTarget {
    /// Base URL for the API (e.g., "http://localhost:11434/v1")
    pub base_url: String,
    /// API key — None for local (Ollama), Some for cloud providers
    pub api_key: Option<String>,
    /// Model identifier (e.g., "llama3.2:3b", "deepseek/deepseek-r1")
    pub model: String,
    /// Provider name for header routing (e.g., "openrouter", "openai", "ollama")
    pub provider: String,
}

/// A single chat message.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Response from a chat completion.
#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub content: String,
    pub model: String,
    pub tokens_in: u32,
    pub tokens_out: u32,
    pub duration_ms: u64,
}

/// Errors that can occur during inference.
#[derive(Debug)]
pub enum InferenceError {
    /// HTTP request failed
    Network(String),
    /// API returned an error status
    Api { status: u16, body: String },
    /// Response JSON couldn't be parsed
    Parse(String),
    /// No content in response choices
    EmptyResponse,
}

impl fmt::Display for InferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InferenceError::Network(e) => write!(f, "network error: {}", e),
            InferenceError::Api { status, body } => {
                write!(f, "API error ({}): {}", status, body)
            }
            InferenceError::Parse(e) => write!(f, "parse error: {}", e),
            InferenceError::EmptyResponse => write!(f, "empty response: no choices returned"),
        }
    }
}

impl InferenceTarget {
    /// Construct a target for local Ollama.
    pub fn ollama(model: &str) -> Self {
        InferenceTarget {
            base_url: "http://localhost:11434/v1".to_string(),
            api_key: None,
            model: model.to_string(),
            provider: "ollama".to_string(),
        }
    }

    /// Construct a target for OpenRouter.
    pub fn openrouter(model: &str, api_key: &str) -> Self {
        InferenceTarget {
            base_url: "https://openrouter.ai/api/v1".to_string(),
            api_key: Some(api_key.to_string()),
            model: model.to_string(),
            provider: "openrouter".to_string(),
        }
    }

    /// Construct a target for OpenAI.
    pub fn openai(model: &str, api_key: &str) -> Self {
        InferenceTarget {
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: Some(api_key.to_string()),
            model: model.to_string(),
            provider: "openai".to_string(),
        }
    }

    /// Construct a target for Anthropic (via OpenAI-compatible proxy).
    pub fn anthropic(model: &str, api_key: &str) -> Self {
        InferenceTarget {
            base_url: "https://api.anthropic.com/v1".to_string(),
            api_key: Some(api_key.to_string()),
            model: model.to_string(),
            provider: "anthropic".to_string(),
        }
    }

    /// Construct a target for Google (via OpenAI-compatible endpoint).
    pub fn google(model: &str, api_key: &str) -> Self {
        InferenceTarget {
            base_url: "https://generativelanguage.googleapis.com/v1beta/openai".to_string(),
            api_key: Some(api_key.to_string()),
            model: model.to_string(),
            provider: "google".to_string(),
        }
    }

    /// Build the JSON body for an OpenAI-compatible chat completion request.
    pub fn build_request_json(&self, messages: &[ChatMessage]) -> serde_json::Value {
        let msgs: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content,
                })
            })
            .collect();

        serde_json::json!({
            "model": self.model,
            "messages": msgs,
        })
    }

    /// Parse an OpenAI-compatible chat completion response.
    pub fn parse_response(
        body: &str,
        duration_ms: u64,
    ) -> Result<ChatResponse, InferenceError> {
        let json: serde_json::Value =
            serde_json::from_str(body).map_err(|e| InferenceError::Parse(e.to_string()))?;

        let content = json["choices"]
            .as_array()
            .and_then(|choices| choices.first())
            .and_then(|choice| choice["message"]["content"].as_str())
            .ok_or(InferenceError::EmptyResponse)?
            .to_string();

        let model = json["model"].as_str().unwrap_or("unknown").to_string();

        let tokens_in = json["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32;
        let tokens_out = json["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32;

        Ok(ChatResponse {
            content,
            model,
            tokens_in,
            tokens_out,
            duration_ms,
        })
    }
}

/// Send a chat completion request. Blocking for now — no streaming.
///
/// Adds `Authorization: Bearer {key}` if api_key is Some.
/// Adds OpenRouter-specific headers if provider is "openrouter".
#[cfg(feature = "sel")]
pub async fn chat_completion(
    target: &InferenceTarget,
    messages: &[ChatMessage],
) -> Result<ChatResponse, InferenceError> {
    use std::time::Instant;

    let client = reqwest::Client::new();
    let url = format!("{}/chat/completions", target.base_url);
    let body = target.build_request_json(messages);

    let mut req = client.post(&url).json(&body);

    // Auth header
    if let Some(ref key) = target.api_key {
        req = req.header("Authorization", format!("Bearer {}", key));
    }

    // OpenRouter-specific headers
    if target.provider == "openrouter" {
        req = req.header("HTTP-Referer", "https://spectral.systemic.engineer");
        req = req.header("X-Title", "spectral");
    }

    let start = Instant::now();
    let response = req
        .send()
        .await
        .map_err(|e| InferenceError::Network(e.to_string()))?;

    let status = response.status().as_u16();
    let response_body = response
        .text()
        .await
        .map_err(|e| InferenceError::Network(e.to_string()))?;

    let duration_ms = start.elapsed().as_millis() as u64;

    if status != 200 {
        return Err(InferenceError::Api {
            status,
            body: response_body,
        });
    }

    InferenceTarget::parse_response(&response_body, duration_ms)
}

// ---------------------------------------------------------------------------
// Config parsing: model URI → InferenceTarget
// ---------------------------------------------------------------------------

/// Parse a model URI like "ollama://llama3.2:3b" or "openrouter://deepseek/deepseek-r1:floor"
/// into an InferenceTarget.
///
/// Supported protocols:
/// - `ollama://model` — local, no API key
/// - `openrouter://model` — reads OPENROUTER_API_KEY from env
/// - `openai://model@secrets.KEY` — reads KEY from env
/// - `anthropic://model@secrets.KEY` — reads KEY from env
/// - `google://model@secrets.KEY` — reads KEY from env
pub fn parse_model_uri(uri: &str) -> Result<InferenceTarget, String> {
    let (protocol, rest) = uri
        .split_once("://")
        .ok_or_else(|| format!("invalid model URI (no ://): {}", uri))?;

    // Split on @ to separate model from key reference
    let (model, key_ref) = if let Some((m, k)) = rest.split_once('@') {
        (m, Some(k))
    } else {
        (rest, None)
    };

    match protocol {
        "ollama" => Ok(InferenceTarget::ollama(model)),
        "openrouter" => {
            let key = resolve_key(key_ref, "OPENROUTER_API_KEY")?;
            Ok(InferenceTarget::openrouter(model, &key))
        }
        "openai" => {
            let key = resolve_key(key_ref, "OPENAI_API_KEY")?;
            Ok(InferenceTarget::openai(model, &key))
        }
        "anthropic" => {
            let key = resolve_key(key_ref, "ANTHROPIC_API_KEY")?;
            Ok(InferenceTarget::anthropic(model, &key))
        }
        "google" => {
            let key = resolve_key(key_ref, "GOOGLE_AI_API_KEY")?;
            Ok(InferenceTarget::google(model, &key))
        }
        _ => Err(format!("unknown protocol: {}", protocol)),
    }
}

/// Resolve an API key from a secrets reference or environment variable.
///
/// If key_ref is Some("secrets.SOME_KEY"), strips "secrets." prefix and reads
/// the environment variable SOME_KEY. If key_ref is None, reads the default
/// env var name.
fn resolve_key(key_ref: Option<&str>, default_env: &str) -> Result<String, String> {
    let env_name = match key_ref {
        Some(r) => r.strip_prefix("secrets.").unwrap_or(r),
        None => default_env,
    };
    std::env::var(env_name).map_err(|_| format!("missing env var: {}", env_name))
}

// ---------------------------------------------------------------------------
// AiConfig: parsed @ai block
// ---------------------------------------------------------------------------

/// Parsed @ai configuration block.
#[derive(Debug, Clone)]
pub struct AiConfig {
    /// Named model targets, keyed by alias.
    pub models: HashMap<String, InferenceTarget>,
    /// Default model alias for routing.
    pub default_model: String,
    /// Fallback model alias (if default is unavailable).
    pub fallback_model: Option<String>,
    /// Premium model alias (quality over cost).
    pub premium_model: Option<String>,
    /// Review model alias (mechanical work).
    pub review_model: Option<String>,
    /// Whether to auto-train on spectral operations.
    pub auto_train: bool,
    /// Whether to auto-commit after AI operations.
    pub auto_commit: bool,
}

impl AiConfig {
    /// Get the default InferenceTarget.
    pub fn default_target(&self) -> Option<&InferenceTarget> {
        self.models.get(&self.default_model)
    }

    /// Get the fallback InferenceTarget.
    pub fn fallback_target(&self) -> Option<&InferenceTarget> {
        self.fallback_model.as_ref().and_then(|k| self.models.get(k))
    }

    /// Get the target for a named role, falling back through the chain.
    pub fn target_for_role(&self, role: &str) -> Option<&InferenceTarget> {
        let alias = match role {
            "default" => Some(&self.default_model),
            "fallback" => self.fallback_model.as_ref(),
            "premium" => self.premium_model.as_ref(),
            "review" => self.review_model.as_ref(),
            _ => self.models.get(role).map(|_| &self.default_model),
        };
        alias
            .and_then(|k| self.models.get(k))
            .or_else(|| self.default_target())
    }
}

/// Parse the @ai block from a config.spec string.
///
/// Minimal parser: finds @ai { ... }, extracts models, routing, runtime blocks.
/// Not a full grammar parser — just enough to construct AiConfig from the
/// config.spec format.
pub fn parse_ai_config(input: &str) -> Result<AiConfig, String> {
    // Find the @ai block
    let ai_start = input
        .find("@ai {")
        .or_else(|| input.find("@ai{"))
        .ok_or("no @ai block found")?;

    let ai_body = extract_block(&input[ai_start + 3..])?;

    // Parse models block
    let models = parse_models_block(&ai_body)?;

    // Parse routing block
    let (default_model, fallback_model, premium_model, review_model) =
        parse_routing_block(&ai_body)?;

    // Parse runtime flags (or top-level flags)
    let (auto_train, auto_commit) = parse_runtime_flags(&ai_body);

    // Validate: default must reference a known model
    if !models.contains_key(&default_model) {
        return Err(format!(
            "default model '{}' not found in models block",
            default_model
        ));
    }

    Ok(AiConfig {
        models,
        default_model,
        fallback_model,
        premium_model,
        review_model,
        auto_train,
        auto_commit,
    })
}

/// Extract the body of a `{ ... }` block, handling nested braces.
fn extract_block(input: &str) -> Result<String, String> {
    let start = input.find('{').ok_or("no opening brace")?;
    let mut depth = 0;
    let mut end = None;
    for (i, ch) in input[start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(start + i);
                    break;
                }
            }
            _ => {}
        }
    }
    let end = end.ok_or("unmatched brace in @ai block")?;
    Ok(input[start + 1..end].to_string())
}

/// Parse the models { ... } sub-block.
fn parse_models_block(ai_body: &str) -> Result<HashMap<String, InferenceTarget>, String> {
    let mut models = HashMap::new();

    let models_start = ai_body.find("models");
    if models_start.is_none() {
        return Ok(models);
    }

    let models_body = extract_block(&ai_body[models_start.unwrap()..])?;

    for line in models_body.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }

        // Parse "name = uri" or "name = uri"
        if let Some((name, uri)) = line.split_once('=') {
            let name = name.trim().to_string();
            let uri = uri.trim().to_string();
            let target = parse_model_uri(&uri)?;
            models.insert(name, target);
        }
    }

    Ok(models)
}

/// Parse the routing { ... } sub-block.
fn parse_routing_block(
    ai_body: &str,
) -> Result<(String, Option<String>, Option<String>, Option<String>), String> {
    let routing_start = ai_body.find("routing");
    if routing_start.is_none() {
        return Err("no routing block found in @ai config".to_string());
    }

    let routing_body = extract_block(&ai_body[routing_start.unwrap()..])?;

    let mut default = None;
    let mut fallback = None;
    let mut premium = None;
    let mut review = None;

    for line in routing_body.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim().to_string();
            match key {
                "default" => default = Some(value),
                "fallback" => fallback = Some(value),
                "premium" => premium = Some(value),
                "review" => review = Some(value),
                _ => {} // ignore unknown routing keys
            }
        }
    }

    let default = default.ok_or("routing block must specify 'default'")?;
    Ok((default, fallback, premium, review))
}

/// Parse runtime flags: auto_train, auto_commit.
/// Checks both `runtime { ... }` sub-block and top-level `auto-train = ...` / `auto_train = ...`.
fn parse_runtime_flags(ai_body: &str) -> (bool, bool) {
    let search_body = if let Some(start) = ai_body.find("runtime") {
        extract_block(&ai_body[start..]).unwrap_or_else(|_| ai_body.to_string())
    } else {
        ai_body.to_string()
    };

    let mut auto_train = false;
    let mut auto_commit = false;

    for line in search_body.lines() {
        let line = line.trim();
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().replace('-', "_");
            let value = value.trim();
            match key.as_str() {
                "auto_train" => auto_train = value == "true",
                "auto_commit" => auto_commit = value == "true",
                _ => {}
            }
        }
    }

    (auto_train, auto_commit)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- InferenceTarget constructors --

    #[test]
    fn test_inference_target_from_ollama() {
        let target = InferenceTarget::ollama("llama3.2:3b");
        assert_eq!(target.base_url, "http://localhost:11434/v1");
        assert!(target.api_key.is_none());
        assert_eq!(target.model, "llama3.2:3b");
        assert_eq!(target.provider, "ollama");
    }

    #[test]
    fn test_inference_target_from_openrouter() {
        let target = InferenceTarget::openrouter("deepseek/deepseek-r1", "sk-test-123");
        assert_eq!(target.base_url, "https://openrouter.ai/api/v1");
        assert_eq!(target.api_key.as_deref(), Some("sk-test-123"));
        assert_eq!(target.model, "deepseek/deepseek-r1");
        assert_eq!(target.provider, "openrouter");
    }

    #[test]
    fn test_inference_target_from_openai() {
        let target = InferenceTarget::openai("gpt-4o-mini", "sk-openai");
        assert_eq!(target.base_url, "https://api.openai.com/v1");
        assert_eq!(target.api_key.as_deref(), Some("sk-openai"));
        assert_eq!(target.model, "gpt-4o-mini");
        assert_eq!(target.provider, "openai");
    }

    #[test]
    fn test_inference_target_from_anthropic() {
        let target = InferenceTarget::anthropic("claude-sonnet", "sk-ant");
        assert_eq!(target.base_url, "https://api.anthropic.com/v1");
        assert_eq!(target.api_key.as_deref(), Some("sk-ant"));
        assert_eq!(target.provider, "anthropic");
    }

    #[test]
    fn test_inference_target_from_google() {
        let target = InferenceTarget::google("gemini-2.0-flash", "goog-key");
        assert!(target.base_url.contains("generativelanguage.googleapis.com"));
        assert_eq!(target.api_key.as_deref(), Some("goog-key"));
        assert_eq!(target.provider, "google");
    }

    // -- Request JSON --

    #[test]
    fn test_chat_request_json() {
        let target = InferenceTarget::ollama("llama3.2:3b");
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are helpful.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            },
        ];

        let json = target.build_request_json(&messages);

        assert_eq!(json["model"], "llama3.2:3b");
        let msgs = json["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0]["role"], "system");
        assert_eq!(msgs[0]["content"], "You are helpful.");
        assert_eq!(msgs[1]["role"], "user");
        assert_eq!(msgs[1]["content"], "Hello");
    }

    #[test]
    fn test_chat_request_json_empty_messages() {
        let target = InferenceTarget::ollama("test");
        let json = target.build_request_json(&[]);
        assert_eq!(json["messages"].as_array().unwrap().len(), 0);
    }

    // -- Response parsing --

    #[test]
    fn test_parse_response_success() {
        let body = r#"{
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "model": "llama3.2:3b",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hello! How can I help?"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 12,
                "completion_tokens": 8,
                "total_tokens": 20
            }
        }"#;

        let resp = InferenceTarget::parse_response(body, 150).unwrap();
        assert_eq!(resp.content, "Hello! How can I help?");
        assert_eq!(resp.model, "llama3.2:3b");
        assert_eq!(resp.tokens_in, 12);
        assert_eq!(resp.tokens_out, 8);
        assert_eq!(resp.duration_ms, 150);
    }

    #[test]
    fn test_parse_response_empty_choices() {
        let body = r#"{"choices": [], "model": "test", "usage": {}}"#;
        let err = InferenceTarget::parse_response(body, 0).unwrap_err();
        assert!(matches!(err, InferenceError::EmptyResponse));
    }

    #[test]
    fn test_parse_response_invalid_json() {
        let err = InferenceTarget::parse_response("not json", 0).unwrap_err();
        assert!(matches!(err, InferenceError::Parse(_)));
    }

    #[test]
    fn test_parse_response_missing_usage() {
        // usage fields are optional — should default to 0
        let body = r#"{
            "choices": [{"message": {"content": "hi"}}],
            "model": "test"
        }"#;
        let resp = InferenceTarget::parse_response(body, 50).unwrap();
        assert_eq!(resp.content, "hi");
        assert_eq!(resp.tokens_in, 0);
        assert_eq!(resp.tokens_out, 0);
    }

    // -- Model URI parsing --

    #[test]
    fn test_parse_model_uri_ollama() {
        let target = parse_model_uri("ollama://llama3.2:3b").unwrap();
        assert_eq!(target.provider, "ollama");
        assert_eq!(target.model, "llama3.2:3b");
        assert!(target.api_key.is_none());
    }

    #[test]
    fn test_parse_model_uri_openrouter_from_env() {
        std::env::set_var("TEST_OR_KEY_4", "test-or-key");
        let target =
            parse_model_uri("openrouter://deepseek/deepseek-r1:floor@secrets.TEST_OR_KEY_4")
                .unwrap();
        assert_eq!(target.provider, "openrouter");
        assert_eq!(target.model, "deepseek/deepseek-r1:floor");
        assert_eq!(target.api_key.as_deref(), Some("test-or-key"));
        std::env::remove_var("TEST_OR_KEY_4");
    }

    #[test]
    fn test_parse_model_uri_with_secrets_ref() {
        std::env::set_var("TEST_ANT_KEY_5", "test-ant-key");
        let target =
            parse_model_uri("anthropic://claude-sonnet@secrets.TEST_ANT_KEY_5").unwrap();
        assert_eq!(target.provider, "anthropic");
        assert_eq!(target.model, "claude-sonnet");
        assert_eq!(target.api_key.as_deref(), Some("test-ant-key"));
        std::env::remove_var("TEST_ANT_KEY_5");
    }

    #[test]
    fn test_parse_model_uri_invalid_no_protocol() {
        let err = parse_model_uri("just-a-model-name").unwrap_err();
        assert!(err.contains("no ://"));
    }

    #[test]
    fn test_parse_model_uri_unknown_protocol() {
        let err = parse_model_uri("huggingface://model").unwrap_err();
        assert!(err.contains("unknown protocol"));
    }

    #[test]
    fn test_parse_model_uri_missing_env() {
        // Use a key ref that points to a definitely-absent env var
        let err =
            parse_model_uri("openrouter://model@secrets.NONEXISTENT_TEST_KEY_999").unwrap_err();
        assert!(err.contains("missing env var"));
    }

    // -- InferenceError Display --

    #[test]
    fn test_error_display_network() {
        let e = InferenceError::Network("timeout".to_string());
        assert_eq!(format!("{}", e), "network error: timeout");
    }

    #[test]
    fn test_error_display_api() {
        let e = InferenceError::Api {
            status: 401,
            body: "unauthorized".to_string(),
        };
        assert!(format!("{}", e).contains("401"));
    }

    #[test]
    fn test_error_display_parse() {
        let e = InferenceError::Parse("bad json".to_string());
        assert!(format!("{}", e).contains("parse error"));
    }

    #[test]
    fn test_error_display_empty() {
        let e = InferenceError::EmptyResponse;
        assert!(format!("{}", e).contains("empty response"));
    }

    // -- AiConfig parsing --

    #[test]
    fn test_parse_ai_config_minimal() {
        // Set env for ollama (no key needed)
        let config = r#"
@ai {
  models {
    llama3 = ollama://llama3.2:3b
  }
  routing {
    default = llama3
  }
  auto-train = true
  auto-commit = false
}
"#;
        let ai = parse_ai_config(config).unwrap();
        assert_eq!(ai.models.len(), 1);
        assert!(ai.models.contains_key("llama3"));
        assert_eq!(ai.default_model, "llama3");
        assert!(ai.fallback_model.is_none());
        assert!(ai.auto_train);
        assert!(!ai.auto_commit);
    }

    #[test]
    fn test_parse_ai_config_with_routing() {
        std::env::set_var("TEST_ANT_KEY_1", "test-key");
        let config = r#"
@ai {
  models {
    local = ollama://llama3.2:3b
    haiku = anthropic://claude-haiku@secrets.TEST_ANT_KEY_1
  }
  routing {
    default = local
    fallback = haiku
    premium = haiku
    review = haiku
  }
  runtime {
    auto_train = true
    auto_commit = false
  }
}
"#;
        let ai = parse_ai_config(config).unwrap();
        assert_eq!(ai.models.len(), 2);
        assert_eq!(ai.default_model, "local");
        assert_eq!(ai.fallback_model.as_deref(), Some("haiku"));
        assert_eq!(ai.premium_model.as_deref(), Some("haiku"));
        assert_eq!(ai.review_model.as_deref(), Some("haiku"));
        assert!(ai.auto_train);
        assert!(!ai.auto_commit);
        std::env::remove_var("TEST_ANT_KEY_1");
    }

    #[test]
    fn test_parse_ai_config_no_ai_block() {
        let err = parse_ai_config("@lsp { }").unwrap_err();
        assert!(err.contains("no @ai block"));
    }

    #[test]
    fn test_parse_ai_config_missing_default() {
        let err = parse_ai_config(
            r#"@ai { models { x = ollama://x } routing { fallback = x } }"#,
        )
        .unwrap_err();
        assert!(err.contains("default"));
    }

    #[test]
    fn test_parse_ai_config_unknown_default_model() {
        let err = parse_ai_config(
            r#"@ai { models { x = ollama://x } routing { default = nonexistent } }"#,
        )
        .unwrap_err();
        assert!(err.contains("not found"));
    }

    // -- AiConfig target resolution --

    #[test]
    fn test_ai_config_default_target() {
        let config = r#"
@ai {
  models {
    local = ollama://llama3.2:3b
  }
  routing {
    default = local
  }
}
"#;
        let ai = parse_ai_config(config).unwrap();
        let target = ai.default_target().unwrap();
        assert_eq!(target.model, "llama3.2:3b");
        assert_eq!(target.provider, "ollama");
    }

    #[test]
    fn test_ai_config_fallback_target() {
        std::env::set_var("TEST_OR_KEY_2", "test-key");
        let config = r#"
@ai {
  models {
    local = ollama://llama3.2:3b
    remote = openrouter://deepseek/deepseek-r1@secrets.TEST_OR_KEY_2
  }
  routing {
    default = local
    fallback = remote
  }
}
"#;
        let ai = parse_ai_config(config).unwrap();
        let fallback = ai.fallback_target().unwrap();
        assert_eq!(fallback.provider, "openrouter");
        std::env::remove_var("TEST_OR_KEY_2");
    }

    #[test]
    fn test_ai_config_target_for_role() {
        let config = r#"
@ai {
  models {
    local = ollama://llama3.2:3b
  }
  routing {
    default = local
  }
}
"#;
        let ai = parse_ai_config(config).unwrap();
        // "default" role — always resolves
        assert!(ai.target_for_role("default").is_some());
        // "fallback" role not configured — falls through to default_target
        let t = ai.target_for_role("fallback");
        assert!(t.is_some());
        // unknown role — not a routing key, not a model name, falls through to default
        let t = ai.target_for_role("unknown");
        assert!(t.is_some());
        assert_eq!(t.unwrap().model, "llama3.2:3b");
    }

    #[test]
    fn test_ai_config_no_fallback_returns_none() {
        let config = r#"
@ai {
  models {
    local = ollama://llama3.2:3b
  }
  routing {
    default = local
  }
}
"#;
        let ai = parse_ai_config(config).unwrap();
        assert!(ai.fallback_target().is_none());
    }

    // -- Integration: parsing the real config.spec format --

    #[test]
    fn test_parse_real_config_spec_format() {
        std::env::set_var("TEST_ANT_KEY_3", "ak-test");
        std::env::set_var("TEST_OAI_KEY_3", "ok-test");
        std::env::set_var("TEST_GOOG_KEY_3", "gk-test");

        let config = r#"
in @secrets
in @ai/config

secrets = "secrets.mirror"

@ai {
  models {
    local   = ollama://llama3.2:3b
    haiku   = anthropic://claude-haiku@secrets.TEST_ANT_KEY_3
    sonnet  = anthropic://claude-sonnet@secrets.TEST_ANT_KEY_3
    opus    = anthropic://claude-opus@secrets.TEST_ANT_KEY_3
    gpt     = openai://gpt-4o-mini@secrets.TEST_OAI_KEY_3
    gemini  = google://gemini-2.0-flash@secrets.TEST_GOOG_KEY_3
  }

  routing {
    default  = local
    fallback = haiku
    premium  = sonnet
    review   = haiku
  }

  runtime {
    auto_train  = true
    auto_commit = false
  }
}
"#;
        let ai = parse_ai_config(config).unwrap();
        assert_eq!(ai.models.len(), 6);
        assert_eq!(ai.default_model, "local");
        assert_eq!(ai.fallback_model.as_deref(), Some("haiku"));
        assert_eq!(ai.premium_model.as_deref(), Some("sonnet"));
        assert_eq!(ai.review_model.as_deref(), Some("haiku"));

        // Verify providers
        assert_eq!(ai.models["local"].provider, "ollama");
        assert_eq!(ai.models["haiku"].provider, "anthropic");
        assert_eq!(ai.models["gpt"].provider, "openai");
        assert_eq!(ai.models["gemini"].provider, "google");

        std::env::remove_var("TEST_ANT_KEY_3");
        std::env::remove_var("TEST_OAI_KEY_3");
        std::env::remove_var("TEST_GOOG_KEY_3");
    }

    // -- extract_block edge cases --

    #[test]
    fn test_extract_block_nested() {
        let result = extract_block("{ outer { inner } }").unwrap();
        assert!(result.contains("inner"));
    }

    #[test]
    fn test_extract_block_no_brace() {
        assert!(extract_block("no braces here").is_err());
    }

    #[test]
    fn test_extract_block_unmatched() {
        assert!(extract_block("{ unmatched").is_err());
    }

    // -- resolve_key --

    #[test]
    fn test_resolve_key_from_env() {
        std::env::set_var("TEST_RESOLVE_KEY_6", "the-value");
        let key = resolve_key(None, "TEST_RESOLVE_KEY_6").unwrap();
        assert_eq!(key, "the-value");
        std::env::remove_var("TEST_RESOLVE_KEY_6");
    }

    #[test]
    fn test_resolve_key_with_secrets_prefix() {
        std::env::set_var("TEST_MY_KEY_7", "secret-val");
        let key = resolve_key(Some("secrets.TEST_MY_KEY_7"), "DEFAULT").unwrap();
        assert_eq!(key, "secret-val");
        std::env::remove_var("TEST_MY_KEY_7");
    }

    #[test]
    fn test_resolve_key_missing() {
        let err = resolve_key(None, "NONEXISTENT_KEY_99999").unwrap_err();
        assert!(err.contains("missing env var"));
    }
}
