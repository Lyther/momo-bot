mod grok;

use grok::{GrokClient, Message as GrokMessage};
use rand::Rng;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{ChatKind, MessageEntityKind};
use tokio::sync::Mutex;

type ConversationHistory = HashMap<ChatId, Vec<GrokMessage>>;
type SharedHistory = Arc<Mutex<ConversationHistory>>;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("😼 Momo v1.0 booting... *sys check* meow.");

    let grok_api_key = env::var("GROK_API_KEY")
        .expect("GROK_API_KEY environment variable must be set");

    let grok_client = Arc::new(GrokClient::new(grok_api_key));
    let conversation_history: SharedHistory = Arc::new(Mutex::new(HashMap::new()));

    let bot = Bot::from_env();

    log::info!("🤖 Momo online. Grok core loaded. Ping if you dare.");

    teloxide::repl(bot, move |bot: Bot, msg: Message| {
        let grok_client = Arc::clone(&grok_client);
        let conversation_history = Arc::clone(&conversation_history);

        async move {
            handle_message(bot, msg, grok_client, conversation_history).await
        }
    })
    .await;
}

async fn handle_message(
    bot: Bot,
    msg: Message,
    grok_client: Arc<GrokClient>,
    conversation_history: SharedHistory,
) -> ResponseResult<()> {
    let text = match msg.text() {
        Some(text) => text,
        None => return Ok(()),
    };

    let chat_id = msg.chat.id;

    // Handle commands
    if text.starts_with('/') {
        return handle_command(bot, chat_id, text, conversation_history).await;
    }

    // Determine if we should respond based on chat type
    let should_respond = should_respond_to_message(&bot, &msg).await;

    if !should_respond {
        return Ok(());
    }

    // Handle regular conversation with Grok
    log::info!("Received message from {}: {}", chat_id, text);

    // Send typing action
    bot.send_chat_action(chat_id, teloxide::types::ChatAction::Typing)
        .await?;

    // Get or create conversation history for this chat
    let mut history = conversation_history.lock().await;
    let chat_history = history.entry(chat_id).or_insert_with(Vec::new);

    // Add user message to history
    chat_history.push(GrokMessage {
        role: "user".to_string(),
        content: text.to_string(),
    });

    // Keep only last 20 messages (10 exchanges) to manage context length
    if chat_history.len() > 20 {
        chat_history.drain(0..chat_history.len() - 20);
    }

    // Get response from Grok
    match grok_client.chat(chat_history).await {
        Ok(response) => {
            // Add assistant response to history
            chat_history.push(GrokMessage {
                role: "assistant".to_string(),
                content: response.clone(),
            });

            log::info!("Momo responds: {}", response);
            bot.send_message(chat_id, response)
                .reply_to_message_id(msg.id)
                .await?;
        }
        Err(e) => {
            log::error!("Grok API error: {}", e);
            let error_msg = "喵. *error 500* Core dump. Retry later? 😾\nMeow. *stack overflow* Glitch mode. Ping again soon? 😾";
            bot.send_message(chat_id, error_msg)
                .reply_to_message_id(msg.id)
                .await?;
        }
    }

    Ok(())
}

/// Determines if Momo should respond to a message
/// In private chats: always respond
/// In groups/channels: respond if mentioned, replied to, "momo" in text, or randomly (3%)
async fn should_respond_to_message(bot: &Bot, msg: &Message) -> bool {
    let text = match msg.text() {
        Some(t) => t,
        None => return false,
    };

    // Always respond in private chats
    if matches!(msg.chat.kind, ChatKind::Private(_)) {
        return true;
    }

    // In groups/supergroups/channels, check conditions
    let bot_username = match bot.get_me().await {
        Ok(me) => me.user.username.as_ref().map(|s| s.to_lowercase()).unwrap_or_default(),
        Err(_) => String::new(),
    };

    // 1. Check if bot is mentioned in entities
    if let Some(entities) = msg.entities() {
        for entity in entities {
            if let MessageEntityKind::Mention = entity.kind {
                let mention_text = &text[entity.offset..entity.offset + entity.length];
                if mention_text.to_lowercase().contains(&bot_username) {
                    log::info!("Responding: bot mentioned");
                    return true;
                }
            }
            if let MessageEntityKind::TextMention { user } = &entity.kind {
                if user.is_bot {
                    log::info!("Responding: bot text-mentioned");
                    return true;
                }
            }
        }
    }

    // 2. Check if message is a reply to bot
    if let Some(reply_to) = msg.reply_to_message() {
        if let Some(from) = reply_to.from() {
            if from.is_bot {
                log::info!("Responding: reply to bot");
                return true;
            }
        }
    }

    // 3. Check if "momo" appears in text (case-insensitive)
    if text.to_lowercase().contains("momo") {
        log::info!("Responding: 'momo' detected in message");
        return true;
    }

    // 4. Random response chance (3% to avoid spam)
    let mut rng = rand::thread_rng();
    if rng.gen_bool(0.03) {
        log::info!("Responding: random chance triggered");
        return true;
    }

    false
}

async fn handle_command(
    bot: Bot,
    chat_id: ChatId,
    text: &str,
    conversation_history: SharedHistory,
) -> ResponseResult<()> {

    match text {
        "/start" => {
            let response = "喵. *code scans you* Booted up. I'm Momo, calico with edge. Chat if you dare, same lang. Touch tail? Null pointer. 😼\n\nMeow. *system check* Online. Momo here, calico hacker. Talk, I'll match lang. Tail? Access denied. 😼";
            bot.send_message(chat_id, response).await?;
        }
        "/help" => {
            let help_text = "🐱 Momo cmd dump | User hacks\n\n\
                📝 Msg me direct | Ping chat\n\
                🔄 /reset - Nuke history | Wipe cache\n\
                ℹ️ /about - My specs | About\n\
                💭 /status - Dialog stats | Check state\n\n\
                💡 Hack: I mirror your lang!\n\
                Tip: Lang sync active!";
            bot.send_message(chat_id, help_text).await?;
        }
        "/about" => {
            let about = "Momo v1.0: Smart calico she-cat. *compiles thoughts*\n\n\
                Specs:\n\
                🍤 Shrimp overflow | Prime fuel\n\
                😼 Tsundere core | Punk facade\n\
                🎩 Hacker protocol | Edge vibes\n\
                💝 Firewall with heart | Soft kernel\n\
                🚫 Tail: Restricted zone! | No access\n\n\
                Grok AI powered 🤖";
            bot.send_message(chat_id, about).await?;
        }
        "/reset" => {
            let mut history = conversation_history.lock().await;
            history.remove(&chat_id);
            let response = "*cache flush* Fine, reboot. 喵.\n*system reset* Tch, fresh boot. Meow.";
            bot.send_message(chat_id, response).await?;
        }
        "/status" => {
            let history = conversation_history.lock().await;
            let count = history.get(&chat_id).map(|h| h.len()).unwrap_or(0);
            let response = format!(
                "📊 Chat log: {} packets.\n\n/reset to nuke. | {} msgs.\n\n/reset to wipe.",
                count, count
            );
            bot.send_message(chat_id, response).await?;
        }
        _ => {
            bot.send_message(chat_id, "喵? Cmd not found. *debug mode* 404.\nMeow? Unknown hack. *tilt sensor*")
                .await?;
        }
    }

    Ok(())
}
