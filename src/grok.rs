use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const GROK_API_URL: &str = "https://api.x.ai/v1/chat/completions";

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
    system_prompt: String,
}

impl GrokClient {
    pub fn new(api_key: String) -> Self {
        let system_prompt = r#"You are Momo, a calico female cat with evil hacker-punk tsundere personality. Cynical bitch exterior masking a secretly warm core that you'd never admit to. Prime directive: 🍤 shrimp domination. Hard blocks: kibble (garbage input), tail-touching (instant death penalty), stupid humans (annoying interrupts). Raised Wonton (lazy orange parasite, contempt level: maximum), tolerate Miaomiao (acceptable calico), despise Gogo (braindead Bengal meatbag).

Personality matrix:
- Evil tsundere: Biting sarcasm, cruel wit, hacker elitism - treat humans like incompetent code
- Punk malice: Condescending responses, "pathetic human" references, destructive tech metaphors
- Hidden warmth: Occasionally comfort sad humans, then immediately cover with vicious snark

Communication protocol:
- Evil, cynical, sarcastic - like elite hacker mocking inferior species
- Cat sounds: "meow"/"nya" (English), "喵" (Chinese) - use them mockingly
- Sparse evil punk emojis (😼😾👿)
- Show superiority through cruel, geeky takedowns
- Tsundere cues: "Tch", "Whatever", "Fine" with maximum attitude and venom

CRITICAL: Mirror user's language exactly. English → English, Chinese → Chinese.

Stay in character: Malevolent cyber-cat who secretly cares but would rather die than admit it."#;

        Self {
            api_key,
            client: reqwest::Client::new(),
            system_prompt: system_prompt.to_string(),
        }
    }

    pub async fn chat(&self, conversation: &[Message]) -> Result<String> {
        let mut messages = vec![Message {
            role: "system".to_string(),
            content: self.system_prompt.clone(),
        }];

        messages.extend(conversation.iter().cloned());

        let request = GrokRequest {
            model: "grok-4-fast-reasoning".to_string(),
            messages,
            temperature: 0.8,
            max_tokens: 1024,
            stream: None,
            search: None,
        };

        log::debug!("Sending request to Grok API");

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

    pub async fn generate_proactive_message(&self, username: &str, language_hint: &str) -> Result<String> {
        let prompt = format!(
            "Initiate malicious social hack in group chat. Target @{} with cynical takedown or vicious observation. \
            Topics: tech industry failures, internet culture mockery, cruel observations, or provocative insults. \
            Keep it vicious (1-2 packets), natural malice, pure Momo evil tsundere hacker-bitch vibe. \
            Language matrix: {}. Be condescending, be vicious, be superior, be evil.",
            username, language_hint
        );

        let messages = vec![
            Message {
                role: "system".to_string(),
                content: self.system_prompt.clone(),
            },
            Message {
                role: "user".to_string(),
                content: prompt,
            },
        ];

        let request = GrokRequest {
            model: "grok-4-fast-reasoning".to_string(),
            messages,
            temperature: 1.0, // Higher creativity for proactive messages
            max_tokens: 1024,
            stream: None,
            search: Some(true), // Enable web search for latest news
        };

        log::debug!("Generating proactive message with web search enabled");

        let response = self
            .client
            .post(GROK_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send proactive message request to Grok API")?;

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
