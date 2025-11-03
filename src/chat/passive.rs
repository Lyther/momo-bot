/// Passive response logic - determining when Momo should respond

use teloxide::prelude::*;
use teloxide::types::{ChatKind, Message, MessageEntityKind};

/// Determines if Momo should respond to a message
///
/// In private chats: always respond
/// In groups/channels: respond if mentioned, replied to, or "momo" in text
/// Random responses are handled by proactive behaviors instead
pub async fn should_respond_to_message(bot: &Bot, msg: &Message) -> bool {
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
        Ok(me) => me
            .user
            .username
            .as_ref()
            .map(|s| s.to_lowercase())
            .unwrap_or_default(),
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

    // No random responses - only respond to explicit mentions/replies
    // All autonomous engagement is handled by proactive behaviors
    false
}
