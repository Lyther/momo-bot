/// Message and bot tracking module

use std::collections::VecDeque;
use teloxide::prelude::*;
use teloxide::types::User;

use crate::config::MAX_RECENT_MESSAGES;
use crate::types::{SharedChatBots, SharedRecentMessages, StoredMessage};

/// Track a message in recent messages
pub async fn track_message(
    recent_messages: SharedRecentMessages,
    chat_id: ChatId,
    message_id: i32,
    user: &User,
    text: &str,
) {
    let mut messages = recent_messages.lock().await;
    let chat_messages = messages.entry(chat_id).or_insert_with(VecDeque::new);

    let username = user
        .username
        .clone()
        .or_else(|| Some(user.first_name.clone()))
        .unwrap_or_else(|| format!("user{}", user.id));

    let stored = StoredMessage {
        message_id,
        user_id: user.id,
        username,
        text: text.to_string(),
        is_bot: user.is_bot,
    };

    chat_messages.push_front(stored);

    // Keep only last N messages
    if chat_messages.len() > MAX_RECENT_MESSAGES {
        chat_messages.pop_back();
    }
}

/// Track a bot user in the chat
pub async fn track_bot(chat_bots: SharedChatBots, chat_id: ChatId, bot_user: &User) {
    let mut bots = chat_bots.lock().await;
    let chat_bot_list = bots.entry(chat_id).or_insert_with(Vec::new);

    let bot_id = bot_user.id;
    let bot_username = bot_user
        .username
        .clone()
        .unwrap_or_else(|| bot_user.first_name.clone());

    // Check if already tracked
    if !chat_bot_list.iter().any(|(id, _)| *id == bot_id) {
        chat_bot_list.push((bot_id, bot_username));
        log::info!("*bot detected* Tracked bot in chat {}: {:?}", chat_id, bot_user.username);
    }
}

/// Get recent messages from a specific user
pub async fn get_user_recent_messages(
    recent_messages: &SharedRecentMessages,
    chat_id: ChatId,
    user_id: UserId,
    limit: usize,
) -> Vec<String> {
    let messages = recent_messages.lock().await;
    if let Some(chat_messages) = messages.get(&chat_id) {
        chat_messages
            .iter()
            .filter(|msg| msg.user_id == user_id && !msg.is_bot)
            .take(limit)
            .map(|msg| msg.text.clone())
            .collect()
    } else {
        Vec::new()
    }
}

/// Get a recent message to reply to (excluding bots and self)
pub async fn get_random_replyable_message(
    recent_messages: &SharedRecentMessages,
    chat_id: ChatId,
    my_bot_id: UserId,
) -> Option<(i32, String, String)> {
    let messages = recent_messages.lock().await;
    let chat_messages = messages.get(&chat_id)?;

    let replyable: Vec<_> = chat_messages
        .iter()
        .filter(|msg| !msg.is_bot && msg.user_id != my_bot_id)
        .take(10) // Only consider last 10 messages
        .collect();

    if replyable.is_empty() {
        return None;
    }

    let idx = rand::random::<usize>() % replyable.len();
    let msg = replyable[idx];
    Some((msg.message_id, msg.username.clone(), msg.text.clone()))
}

/// Get recent conversation context (last N messages as formatted string)
pub async fn get_recent_context(
    recent_messages: &SharedRecentMessages,
    chat_id: ChatId,
    limit: usize,
) -> String {
    let messages = recent_messages.lock().await;
    if let Some(chat_messages) = messages.get(&chat_id) {
        chat_messages
            .iter()
            .take(limit)
            .filter(|msg| !msg.is_bot)
            .map(|msg| format!("{}: {}", msg.username, msg.text))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        String::new()
    }
}

/// Get a random bot from the chat (excluding self)
pub async fn get_random_bot(
    chat_bots: &SharedChatBots,
    chat_id: ChatId,
    my_bot_id: UserId,
) -> Option<String> {
    let bots = chat_bots.lock().await;
    let chat_bot_list = bots.get(&chat_id)?;

    let other_bots: Vec<_> = chat_bot_list
        .iter()
        .filter(|(id, _)| *id != my_bot_id)
        .collect();

    if other_bots.is_empty() {
        return None;
    }

    let idx = rand::random::<usize>() % other_bots.len();
    Some(other_bots[idx].1.clone())
}

/// Extract a topic from recent conversation (simple keyword extraction)
pub async fn extract_topic_from_recent(
    recent_messages: &SharedRecentMessages,
    chat_id: ChatId,
) -> Option<String> {
    let context = get_recent_context(recent_messages, chat_id, 5).await;

    if context.is_empty() {
        return None;
    }

    // Simple extraction: just use the recent context as the topic
    // The LLM will figure out what's relevant
    Some(context)
}
