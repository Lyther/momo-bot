/// Grok API client
use std::env;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::{
    GROK_API_URL, GROK_ENABLE_MULTIMODAL_DEFAULT, GROK_ENABLE_SEARCH_DEFAULT, GROK_MAX_TOKENS,
    GROK_MODEL, GROK_MODEL_PROACTIVE, GROK_TEMPERATURE, GROK_TEMPERATURE_PROACTIVE,
};
use crate::prompts::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Content {
    Text(String),
    Parts(Vec<ContentPart>),
}

impl Content {
    pub fn as_text(&self) -> String {
        match self {
            Content::Text(t) => t.clone(),
            Content::Parts(parts) => parts
                .iter()
                .map(|p| match p {
                    ContentPart::Text { text } => text.clone(),
                    ContentPart::ImageUrl { .. } => "[image attached]".to_string(),
                })
                .collect::<Vec<_>>()
                .join(" "),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: Content,
}

impl Message {
    pub fn text(role: &str, content: impl Into<String>) -> Self {
        Self {
            role: role.to_string(),
            content: Content::Text(content.into()),
        }
    }
}

#[derive(Debug, Serialize)]
struct GrokRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    search: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct GrokResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Clone)]
pub struct GrokConfig {
    pub chat_model: String,
    pub proactive_model: String,
    pub max_tokens: u32,
    pub enable_search: bool,
    pub enable_multimodal: bool,
}

impl GrokConfig {
    pub fn from_env() -> Self {
        let chat_model = env::var("GROK_MODEL").unwrap_or_else(|_| GROK_MODEL.to_string());
        let proactive_model = env::var("GROK_MODEL_PROACTIVE")
            .ok()
            .or_else(|| env::var("GROK_MODEL").ok())
            .unwrap_or_else(|| GROK_MODEL_PROACTIVE.to_string());
        let max_tokens = env::var("GROK_MAX_TOKENS")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(GROK_MAX_TOKENS);

        Self {
            chat_model,
            proactive_model,
            max_tokens,
            enable_search: parse_bool_env("GROK_ENABLE_SEARCH", GROK_ENABLE_SEARCH_DEFAULT),
            enable_multimodal: parse_bool_env(
                "GROK_ENABLE_MULTIMODAL",
                GROK_ENABLE_MULTIMODAL_DEFAULT,
            ),
        }
    }
}

fn parse_bool_env(key: &str, default: bool) -> bool {
    env::var(key)
        .ok()
        .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes" | "y" | "on"))
        .unwrap_or(default)
}

pub struct GrokClient {
    api_key: String,
    client: reqwest::Client,
    config: GrokConfig,
}

impl GrokClient {
    pub fn new(api_key: String) -> Self {
        Self::with_config(api_key, GrokConfig::from_env())
    }

    pub fn with_config(api_key: String, config: GrokConfig) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
            config,
        }
    }

    pub fn allows_multimodal(&self) -> bool {
        self.config.enable_multimodal
    }

    pub fn chat_model_name(&self) -> &str {
        &self.config.chat_model
    }

    pub fn proactive_model_name(&self) -> &str {
        &self.config.proactive_model
    }

    pub fn search_enabled(&self) -> bool {
        self.config.enable_search
    }

    /// Send a chat message to Grok API with optional context hint
    pub async fn chat(
        &self,
        conversation: &[Message],
        context_hint: Option<&str>,
    ) -> Result<String> {
        let mut messages = vec![Message::text("system", MOMO_SYSTEM_PROMPT.to_string())];

        if let Some(ctx) = context_hint {
            if !ctx.trim().is_empty() {
                messages.push(Message::text(
                    "system",
                    format!("Recent group context (latest first):\n{}", ctx),
                ));
            }
        }

        messages.extend(conversation.iter().cloned());

        let request = GrokRequest {
            model: self.config.chat_model.clone(),
            messages,
            temperature: GROK_TEMPERATURE,
            max_tokens: self.config.max_tokens,
            stream: None,
            search: if self.config.enable_search {
                Some(true)
            } else {
                None
            },
        };

        log::debug!(
            "Sending request to Grok API (chat model: {})",
            request.model
        );

        self.send_request(request).await
    }

    /// Generate a proactive message - mention user with judgment
    pub async fn generate_mention_user_message(
        &self,
        username: &str,
        recent_messages: &[String],
        conversation_history: &[Message],
    ) -> Result<String> {
        let prompt = proactive_mention_user_prompt(username, recent_messages);
        self.generate_with_prompt(prompt, conversation_history, false)
            .await
    }

    /// Generate a proactive message - reply to specific message
    pub async fn generate_reply_to_message(
        &self,
        username: &str,
        message: &str,
        conversation_history: &[Message],
    ) -> Result<String> {
        let prompt = proactive_reply_to_message_prompt(username, message);
        self.generate_with_prompt(prompt, conversation_history, false)
            .await
    }

    /// Generate a proactive message - natural comment
    pub async fn generate_natural_comment(
        &self,
        recent_context: &str,
        conversation_history: &[Message],
    ) -> Result<String> {
        let prompt = proactive_natural_comment_prompt(recent_context);
        self.generate_with_prompt(prompt, conversation_history, false)
            .await
    }

    /// Generate a random conversation starter
    pub async fn generate_random_starter(
        &self,
        conversation_history: &[Message],
    ) -> Result<String> {
        let prompt = random_conversation_starter_prompt();
        self.generate_with_prompt(prompt.to_string(), conversation_history, false)
            .await
    }

    /// Generate a news-based conversation starter
    pub async fn generate_news_starter(
        &self,
        topic: &str,
        conversation_history: &[Message],
    ) -> Result<String> {
        let prompt = news_conversation_starter_prompt(topic);
        self.generate_with_prompt(prompt, conversation_history, true)
            .await
    }

    /// Generate bot interaction message
    pub async fn generate_bot_interaction(
        &self,
        bot_username: &str,
        conversation_history: &[Message],
    ) -> Result<String> {
        let prompt = bot_interaction_prompt(bot_username);
        self.generate_with_prompt(prompt, conversation_history, false)
            .await
    }

    /// Generate cat behavior message
    pub async fn generate_cat_behavior(&self, conversation_history: &[Message]) -> Result<String> {
        let prompt = cat_behavior_prompt();
        self.generate_with_prompt(prompt.to_string(), conversation_history, false)
            .await
    }

    /// Generate playful mode initiation
    pub async fn generate_playful_mode(&self, conversation_history: &[Message]) -> Result<String> {
        let prompt = playful_mode_prompt();
        self.generate_with_prompt(prompt.to_string(), conversation_history, false)
            .await
    }

    /// Generate annoyed mode response
    pub async fn generate_annoyed_response(
        &self,
        username: &str,
        conversation_history: &[Message],
    ) -> Result<String> {
        let prompt = annoyed_mode_response_prompt(username);
        self.generate_with_prompt(prompt, conversation_history, false)
            .await
    }

    /// Internal helper to generate with a prompt
    async fn generate_with_prompt(
        &self,
        prompt: String,
        conversation_history: &[Message],
        force_search: bool,
    ) -> Result<String> {
        let mut messages = vec![Message::text("system", MOMO_SYSTEM_PROMPT.to_string())];

        // Include conversation history for context
        messages.extend(conversation_history.iter().cloned());

        // Add the prompt
        messages.push(Message::text("user", prompt));

        let request = GrokRequest {
            model: self.config.proactive_model.clone(),
            messages,
            temperature: GROK_TEMPERATURE_PROACTIVE,
            max_tokens: self.config.max_tokens,
            stream: None,
            search: if self.config.enable_search || force_search {
                Some(true)
            } else {
                None
            },
        };

        self.send_request(request).await
    }

    /// Internal method to send request to Grok API
    async fn send_request(&self, request: GrokRequest) -> Result<String> {
        let response = self
            .client
            .post(GROK_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Grok API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Grok API error ({}): {}", status, error_text);
        }

        let grok_response: GrokResponse = response
            .json()
            .await
            .context("Failed to parse Grok API response")?;

        grok_response
            .choices
            .first()
            .map(|choice| choice.message.content.as_text())
            .context("No response from Grok API")
    }
}
