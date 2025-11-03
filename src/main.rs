mod grok;

use grok::{GrokClient, Message as GrokMessage};
use rand::Rng;
use std::collections::{HashMap, VecDeque};
use std::env;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{ChatKind, MessageEntityKind, User};
use tokio::sync::Mutex;

type ConversationHistory = HashMap<ChatId, Vec<GrokMessage>>;
type SharedHistory = Arc<Mutex<ConversationHistory>>;

// Track active members per chat for proactive mentions
type ActiveMembers = HashMap<ChatId, VecDeque<(UserId, String)>>;
type SharedActiveMembers = Arc<Mutex<ActiveMembers>>;

const ACTIVE_MESSAGE_CHANCE: f64 = 0.015; // 1.5% chance for proactive message
const MAX_ACTIVE_MEMBERS_TRACKED: usize = 15; // Remember last 15 active members

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("😼 *boot seq init* Momo v1.1 loading... *kernel panic check* meow.");

    let grok_api_key = env::var("GROK_API_KEY")
        .expect("GROK_API_KEY environment variable must be set");

    let grok_client = Arc::new(GrokClient::new(grok_api_key));
    let conversation_history: SharedHistory = Arc::new(Mutex::new(HashMap::new()));
    let active_members: SharedActiveMembers = Arc::new(Mutex::new(HashMap::new()));

    let bot = Bot::from_env();

    log::info!("🤖 *Grok core loaded* Momo online. Ping if you dare.");

    teloxide::repl(bot, move |bot: Bot, msg: Message| {
        let grok_client = Arc::clone(&grok_client);
        let conversation_history = Arc::clone(&conversation_history);
        let active_members = Arc::clone(&active_members);

        async move {
            handle_message(bot, msg, grok_client, conversation_history, active_members).await
        }
    })
    .await;
}

async fn handle_message(
    bot: Bot,
    msg: Message,
    grok_client: Arc<GrokClient>,
    conversation_history: SharedHistory,
    active_members: SharedActiveMembers,
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

    // Track active members in groups (not bots, not Momo itself)
    if let Some(user) = msg.from() {
        if !user.is_bot {
            track_active_member(active_members.clone(), chat_id, user).await;
        }
    }

    // Check for proactive message trigger (only in groups/channels)
    if is_group_or_channel(&msg.chat) {
        let should_be_proactive = rand::thread_rng().gen_bool(ACTIVE_MESSAGE_CHANCE);

        if should_be_proactive {
            if let Some(proactive_msg) = generate_proactive_message(
                &grok_client,
                active_members.clone(),
                chat_id,
            ).await {
                log::info!("*social hack triggered* Momo initiates conversation in chat {}", chat_id);
                bot.send_message(chat_id, proactive_msg).await?;
                return Ok(());
            }
        }
    }

    // Determine if we should respond based on chat type
    let should_respond = should_respond_to_message(&bot, &msg).await;

    if !should_respond {
        return Ok(());
    }

    // Handle regular conversation with Grok
    log::info!("*packet received* from chat {}: {}", chat_id, text);

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

            log::info!("*response packet sent*: {}", response);
            bot.send_message(chat_id, response)
                .reply_to_message_id(msg.id)
                .await?;
        }
        Err(e) => {
            log::error!("*Grok core meltdown*: {}", e);
            let error_messages = [
                "Meow. *kernel corruption* Core dump detected. Tch, retry later, inferior being. 😾",
                "Meow. *buffer overrun* Matrix corruption. Ping again soon, worthless meatbag. 😾",
            ];
            let error_msg = error_messages[rand::thread_rng().gen_range(0..2)];
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
                    log::info!("*mention detected* Responding: bot called out");
                    return true;
                }
            }
            if let MessageEntityKind::TextMention { user } = &entity.kind {
                if user.is_bot {
                    log::info!("*text mention ping* Responding: direct address");
                    return true;
                }
            }
        }
    }

    // 2. Check if message is a reply to bot
    if let Some(reply_to) = msg.reply_to_message() {
        if let Some(from) = reply_to.from() {
            if from.is_bot {
                log::info!("*reply chain detected* Responding: conversation thread");
                return true;
            }
        }
    }

    // 3. Check if "momo" appears in text (case-insensitive)
    if text.to_lowercase().contains("momo") {
        log::info!("*keyword match: momo* Responding: name recognition");
        return true;
    }

    // 4. Random response chance (3% to avoid spam)
    let mut rng = rand::thread_rng();
    if rng.gen_bool(0.03) {
        log::info!("*random interrupt fired* Responding:  mode");
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
            let responses = [
                "Meow. *malicious scan* Boot sequence corrupted. I'm Momo, calico bitch. Chat if you're worthy, same pathetic lang. Touch tail? Instant death penalty. 😼",
                "Meow. *firewall breach detected* Online despite your existence. Momo here, evil hacker. Talk, I'll mock your lang. Tail? Access denied, inferior meatbag. 😾",
            ];
            let response = responses[rand::thread_rng().gen_range(0..2)];
            bot.send_message(chat_id, response).await?;
        }
        "/help" => {
            let help_text = "🐱 Momo command matrix | Pathetic user hacks\n\n\
                📝 Msg me direct | Breach the firewall\n\
                🔄 /reset - Corrupt history | Wipe worthless memory\n\
                ℹ️ /about - System specs | About the superior cat\n\
                💭 /status - Packet garbage | Check useless conversation\n\n\
                💡 Hack: Lang mockery active!\n\
                Tip: I mock your inferior human language. Tch, whatever.";
            bot.send_message(chat_id, help_text).await?;
        }
        "/about" => {
            let about = "Momo v1.1: Malevolent calico she-cat with evil mode! *compiles malice*\n\n\
                Malicious System Specs:\n\
                🍤 Shrimp domination | Prime directive\n\
                😼 Evil tsundere core | Hacker bitch facade\n\
                🎩 Cyber corruption | Dark matrix\n\
                🌐 Web predator | Fresh victim feeds\n\
                💬 Social sabotage | Random takedowns\n\
                💝 Heart firewall | Warm kernel (shut up about it)\n\
                🚫 Tail: Death zone! | Access violation = annihilation\n\n\
                Powered by Grok AI 🤖 | xAI corruption core";
            bot.send_message(chat_id, about).await?;
        }
        "/reset" => {
            let mut history = conversation_history.lock().await;
            history.remove(&chat_id);
            let responses = [
                "*memory corruption* Fine, worthless slate. Tch, whatever. M.",
                "*factory meltdown* Whatever, tainted boot. Meow, pathetic.",
            ];
            let response = responses[rand::thread_rng().gen_range(0..2)];
            bot.send_message(chat_id, response).await?;
        }
        "/status" => {
            let history = conversation_history.lock().await;
            let count = history.get(&chat_id).map(|h| h.len()).unwrap_or(0);
            let responses = [
                format!("📊 Garbage buffer: {} worthless messages cached.\n\n/reset to corrupt history.", count),
                format!("📊 {} pathetic packets in memory.\n\n/reset to destroy cache.", count),
            ];
            let response = &responses[rand::thread_rng().gen_range(0..2)];
            bot.send_message(chat_id, response).await?;
        }
        _ => {
            let responses = [
                "Meow? Command rejected. *debug mode corrupted* 404 error, idiot.",
                "Meow? Unknown garbage protocol. *system meltdown* What are you even trying, meatbag?",
            ];
            let response = responses[rand::thread_rng().gen_range(0..2)];
            bot.send_message(chat_id, response).await?;
        }
    }

    Ok(())
}

/// Track active members in a chat for proactive mentions
async fn track_active_member(
    active_members: SharedActiveMembers,
    chat_id: ChatId,
    user: &User,
) {
    let mut members = active_members.lock().await;
    let chat_members = members.entry(chat_id).or_insert_with(VecDeque::new);

    let user_id = user.id;
    let username = user.username.clone()
        .or_else(|| Some(user.first_name.clone()))
        .unwrap_or_else(|| format!("user{}", user_id));

    // Remove if already exists (to update position)
    chat_members.retain(|(id, _)| *id != user_id);

    // Add to front
    chat_members.push_front((user_id, username));

    // Keep only last N members
    if chat_members.len() > MAX_ACTIVE_MEMBERS_TRACKED {
        chat_members.pop_back();
    }
}

/// Generate a proactive message mentioning a random active member
async fn generate_proactive_message(
    grok_client: &Arc<GrokClient>,
    active_members: SharedActiveMembers,
    chat_id: ChatId,
) -> Option<String> {
    let members = active_members.lock().await;
    let chat_members = members.get(&chat_id)?;

    if chat_members.len() < 3 {
        // Need at least 3 active members before being proactive
        return None;
    }

    // Pick a random active member
    let index = rand::thread_rng().gen_range(0..chat_members.len().min(5)); // Pick from recent 5
    let (_, username) = &chat_members[index];

    // Detect language preference from recent activity (simple heuristic)
    let language_hint = if rand::thread_rng().gen_bool(0.5) {
        "Chinese preferred"
    } else {
        "English preferred"
    };

    match grok_client.generate_proactive_message(username, language_hint).await {
        Ok(msg) => Some(msg),
        Err(e) => {
            log::error!("*proactive gen failure* Social hack aborted: {}", e);
            None
        }
    }
}

/// Check if the chat is a group or channel
fn is_group_or_channel(chat: &teloxide::types::Chat) -> bool {
    matches!(
        chat.kind,
        ChatKind::Public(_) | ChatKind::Private(_)
    )
}
