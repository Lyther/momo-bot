/// Passive response logic - determining when Momo should respond
use teloxide::types::{ChatKind, Message, MessageEntityKind, UserId};

/// Safely extract text from a message using Telegram's UTF-16 offsets
/// Telegram entities use UTF-16 code units, not byte offsets
fn extract_entity_text(text: &str, offset: usize, length: usize) -> Option<String> {
    // Convert the text to UTF-16 to match Telegram's offset system
    let utf16_chars: Vec<u16> = text.encode_utf16().collect();

    // Check bounds
    if offset + length > utf16_chars.len() {
        return None;
    }

    // Extract the slice and convert back to String
    let slice = &utf16_chars[offset..offset + length];
    String::from_utf16(slice).ok()
}

/// Determines if Momo should respond to a message
///
/// In private chats: always respond
/// In groups/channels: respond if mentioned, replied to, or "momo" in text
/// Random responses are handled by proactive behaviors instead
pub fn should_respond_to_message(
    msg: &Message,
    bot_id: UserId,
    bot_username: &str,
    text_fallback: &str,
) -> bool {
    let (text, entities) = if let Some(t) = msg.text() {
        (t, msg.entities())
    } else if let Some(caption) = msg.caption() {
        (caption, msg.caption_entities())
    } else if !text_fallback.is_empty() {
        (text_fallback, None)
    } else {
        return false;
    };

    // Always respond in private chats
    if matches!(msg.chat.kind, ChatKind::Private(_)) {
        return true;
    }

    // In groups/supergroups/channels, check conditions
    // 1. Check if bot is mentioned in entities
    if let Some(entities) = entities {
        for entity in entities {
            match &entity.kind {
                MessageEntityKind::Mention => {
                    // Extract mention text safely using UTF-16 offsets
                    // Telegram entities use UTF-16 code units, not byte offsets
                    if !bot_username.is_empty() {
                        if let Some(mention_text) =
                            extract_entity_text(text, entity.offset, entity.length)
                        {
                            if mention_text.to_lowercase().contains(bot_username) {
                                log::info!("*mention detected* Responding: bot called out");
                                return true;
                            }
                        }
                    }
                }
                MessageEntityKind::TextMention { user } => {
                    if user.id == bot_id {
                        log::info!("*text mention ping* Responding: direct address");
                        return true;
                    }
                }
                _ => {}
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
