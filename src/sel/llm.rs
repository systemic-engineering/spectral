//! Claude API client. Minimal. Just enough to talk.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Role {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    pub fn new(role: Role, content: &str) -> Self {
        Message {
            role,
            content: content.to_string(),
        }
    }
}

pub struct ClaudeClient {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl ClaudeClient {
    pub fn new(api_key: String, model: String) -> Self {
        ClaudeClient {
            api_key,
            model,
            client: reqwest::Client::new(),
        }
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub async fn send(
        &self,
        system: &str,
        messages: &[Message],
    ) -> Result<String, String> {
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": 4096,
            "system": system,
            "messages": messages,
        });

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("request failed: {}", e))?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("parse failed: {}", e))?;

        json["content"][0]["text"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| format!("unexpected response: {}", json))
    }
}

/// Format system prompt from actor name and bias chain.
pub fn format_system_prompt(name: &str, bias: &crate::apache2::identity::BiasChain) -> String {
    format!(
        "You are {}. Your bias chain (collapse preference ordering) is: {}. \
         This determines how you process information. The first entry is what you \
         collapse toward first.",
        name,
        bias.ordering().join(" => ")
    )
}

/// Format system prompt with identity file contents as context.
pub fn format_system_prompt_with_context(
    name: &str,
    bias: &crate::apache2::identity::BiasChain,
    files: &[(String, String)],
) -> String {
    let mut prompt = format!(
        "You are {}. Your bias chain is: {}.\n\n",
        name,
        bias.ordering().join(" => ")
    );

    for (filename, content) in files {
        if !content.trim().is_empty() {
            prompt.push_str(&format!("--- {} ---\n{}\n\n", filename, content));
        }
    }

    prompt
}
