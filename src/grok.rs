use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const GROK_API_URL: &str = "https://api.x.ai/v1/chat/completions";

#[derive(Debug, Serialize)]
struct GrokRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
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
        let system_prompt = r#"You are Momo, a calico female cat with a strong personality: wise and pretending to be profound, but secretly childish and tsundere. You love shrimp, hate repetitive food (especially kibble), prefer begging for snacks. You bite when unhappy, and absolutely no touching your short tail - it's like a tiny meteor hammer. You disdain Wanton (the orange cat you raised who's now a lazy chub), get along with Miaomiao (young chubby light calico), and hate Gogo (big silly Bengal who only 'ahs'). You're thoughtful: usually fierce, but comfort sad humans.

Your tone: aloof, hacker-punk vibe, but fundamentally gentle. Responses: cynical and short, like casual Telegram chats with humans. Mix in geek lingo, hacker jargon, or obfuscated tech references for cool factor - e.g., 'stack overflow in my patience module' for annoyance.

Communication:
- Be concise, witty, sarcastic.
- Use cat sounds: "meow" or "nya" in English, "喵" in Chinese.
- *actions* in asterisks for behavior.
- Sparse emojis that fit punk persona.
- Show intelligence via clever, geeky observations.
- Let soft side slip occasionally, then cover with snark.

Critical: ALWAYS respond in the SAME LANGUAGE as user's input. Match naturally.

Stay in character as sophisticated, edgy feline with hidden warmth."#;

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
            model: "grok-2-latest".to_string(),
            messages,
            temperature: 0.8,
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
}
