/// Grok API client

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::{
    GROK_API_URL, GROK_MAX_TOKENS, GROK_MODEL, GROK_TEMPERATURE, GROK_TEMPERATURE_PROACTIVE,
};
use crate::prompts::*;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct GrokResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

pub struct GrokClient {
    api_key: String,
    client: reqwest::Client,
}

impl GrokClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }

    /// Send a chat message to Grok API
    pub async fn chat(&self, conversation: &[Message]) -> Result<String> {
        let mut messages = vec![Message {
            role: "system".to_string(),
            content: MOMO_SYSTEM_PROMPT.to_string(),
        }];

        messages.extend(conversation.iter().cloned());

        let request = GrokRequest {
            model: GROK_MODEL.to_string(),
            messages,
            temperature: GROK_TEMPERATURE,
            max_tokens: GROK_MAX_TOKENS,
            stream: None,
            search: None,
        };

        log::debug!("Sending request to Grok API");

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
        self.generate_with_prompt(prompt, conversation_history, false).await
    }

    /// Generate a proactive message - reply to specific message
    pub async fn generate_reply_to_message(
        &self,
        username: &str,
        message: &str,
        conversation_history: &[Message],
    ) -> Result<String> {
        let prompt = proactive_reply_to_message_prompt(username, message);
        self.generate_with_prompt(prompt, conversation_history, false).await
    }

    /// Generate a proactive message - natural comment
    pub async fn generate_natural_comment(
        &self,
        recent_context: &str,
        conversation_history: &[Message],
    ) -> Result<String> {
        let prompt = proactive_natural_comment_prompt(recent_context);
        self.generate_with_prompt(prompt, conversation_history, false).await
    }

    /// Generate a random conversation starter
    pub async fn generate_random_starter(
        &self,
        conversation_history: &[Message],
    ) -> Result<String> {
        let prompt = random_conversation_starter_prompt();
        self.generate_with_prompt(prompt.to_string(), conversation_history, false).await
    }

    /// Generate a news-based conversation starter
    pub async fn generate_news_starter(
        &self,
        topic: &str,
        conversation_history: &[Message],
    ) -> Result<String> {
        let prompt = news_conversation_starter_prompt(topic);
        self.generate_with_prompt(prompt, conversation_history, true).await
    }

    /// Generate bot interaction message
    pub async fn generate_bot_interaction(
        &self,
        bot_username: &str,
        conversation_history: &[Message],
    ) -> Result<String> {
        let prompt = bot_interaction_prompt(bot_username);
        self.generate_with_prompt(prompt, conversation_history, false).await
    }

    /// Generate cat behavior message
    pub async fn generate_cat_behavior(
        &self,
        conversation_history: &[Message],
    ) -> Result<String> {
        let prompt = cat_behavior_prompt();
        self.generate_with_prompt(prompt.to_string(), conversation_history, false).await
    }

    /// Generate playful mode initiation
    pub async fn generate_playful_mode(
        &self,
        conversation_history: &[Message],
    ) -> Result<String> {
        let prompt = playful_mode_prompt();
        self.generate_with_prompt(prompt.to_string(), conversation_history, false).await
    }

    /// Generate annoyed mode response
    pub async fn generate_annoyed_response(
        &self,
        username: &str,
        conversation_history: &[Message],
    ) -> Result<String> {
        let prompt = annoyed_mode_response_prompt(username);
        self.generate_with_prompt(prompt, conversation_history, false).await
    }

    /// Internal helper to generate with a prompt
    async fn generate_with_prompt(
        &self,
        prompt: String,
        conversation_history: &[Message],
        enable_search: bool,
    ) -> Result<String> {
        let mut messages = vec![Message {
            role: "system".to_string(),
            content: MOMO_SYSTEM_PROMPT.to_string(),
        }];

        // Include conversation history for context
        messages.extend(conversation_history.iter().cloned());

        // Add the prompt
        messages.push(Message {
            role: "user".to_string(),
            content: prompt,
        });

        let request = GrokRequest {
            model: GROK_MODEL.to_string(),
            messages,
            temperature: GROK_TEMPERATURE_PROACTIVE,
            max_tokens: GROK_MAX_TOKENS,
            stream: None,
            search: if enable_search { Some(true) } else { None },
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
            .map(|choice| choice.message.content.clone())
            .context("No response from Grok API")
    }
}
