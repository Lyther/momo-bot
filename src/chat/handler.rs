/// Main message handler
use std::sync::Arc;
use std::time::Duration;
use teloxide::errors::{ApiError, RequestError};
use teloxide::prelude::*;
use teloxide::types::{ChatAction, Message, MessageId};

use crate::commands::handle_command;
use crate::config::{
    ACTIVE_MESSAGE_CHANCE, CAT_BAIT_RESPONSE_CHANCE, GROUP_CONTEXT_MESSAGES,
    MAX_CONVERSATION_HISTORY, PROACTIVE_BLOCK_COOLDOWN_SECS,
};
use crate::grok::{Content, ContentPart, GrokClient, ImageUrl, Message as GrokMessage};
use crate::members::track_active_member;
use crate::prompts::{CAT_BAIT_RESPONSES, ERROR_RESPONSES};
use crate::state::{
    clear_proactive_block, get_chat_state, pause_proactive, set_chat_mood, MomoMood,
    SharedChatStates,
};
use crate::tracking::{get_recent_context, track_bot, track_message};
use crate::types::{SharedActiveMembers, SharedChatBots, SharedHistory, SharedRecentMessages};
use crate::utils::{is_group_or_channel, random_choice};

use super::active::generate_proactive_message;
use super::passive::should_respond_to_message;

/// Main message handler
pub async fn handle_message(
    bot: Bot,
    msg: Message,
    grok_client: Arc<GrokClient>,
    conversation_history: SharedHistory,
    active_members: SharedActiveMembers,
    recent_messages: SharedRecentMessages,
    chat_bots: SharedChatBots,
    chat_states: SharedChatStates,
) -> ResponseResult<()> {
    let allow_multimodal = grok_client.allows_multimodal();

    let (text, attachments) = match extract_text_and_media(&bot, &msg, allow_multimodal).await {
        Some(value) => value,
        None => return Ok(()),
    };

    let chat_id = msg.chat.id;

    // Handle commands
    if text.starts_with('/') {
        return handle_command(bot, chat_id, text.as_str(), conversation_history).await;
    }

    // Track message
    if let Some(user) = msg.from() {
        track_message(recent_messages.clone(), chat_id, msg.id.0, user, &text).await;

        // Track bots
        if user.is_bot {
            track_bot(chat_bots.clone(), chat_id, user).await;
        } else {
            // Track active human members
            track_active_member(active_members.clone(), chat_id, user).await;
        }
    }

    // Get bot identity once for this message
    let me = bot.get_me().await?;
    let my_bot_id = me.user.id;
    let bot_username = me
        .user
        .username
        .as_deref()
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    // Check for proactive message trigger (only in groups/channels)
    if is_group_or_channel(&msg.chat) {
        let should_be_proactive = rand::random::<f64>() < ACTIVE_MESSAGE_CHANCE;

        if should_be_proactive {
            if let Some((proactive_msg, reply_to_msg_id)) = generate_proactive_message(
                &grok_client,
                active_members.clone(),
                conversation_history.clone(),
                recent_messages.clone(),
                chat_bots.clone(),
                chat_states.clone(),
                chat_id,
                my_bot_id,
            )
            .await
            {
                log::info!(
                    "*proactive message triggered* in chat {}: {}",
                    chat_id,
                    proactive_msg
                );

                let mut send_msg = bot.send_message(chat_id, proactive_msg);

                // Add reply if specified
                if let Some(reply_id) = reply_to_msg_id {
                    send_msg = send_msg.reply_to_message_id(MessageId(reply_id));
                }

                match send_msg.await {
                    Ok(_) => {
                        clear_proactive_block(&chat_states, chat_id).await;
                    }
                    Err(err) => {
                        log::warn!("*proactive send failed* chat {}: {}", chat_id, err);
                        handle_send_error(chat_id, &err, &chat_states).await;
                    }
                }
                return Ok(());
            }
        }
    }

    // Determine if we should respond based on chat type (explicit mentions only)
    let should_respond = should_respond_to_message(&msg, my_bot_id, &bot_username, &text);
    let state = get_chat_state(&chat_states, chat_id).await;

    if !should_respond {
        // Small chance to pounce on "cat bait" keywords even without a mention
        if state.should_respond()
            && contains_cat_bait(&text)
            && rand::random::<f64>() < CAT_BAIT_RESPONSE_CHANCE
            && msg.from().map(|u| !u.is_bot).unwrap_or(true)
        {
            let response = random_choice(CAT_BAIT_RESPONSES);
            if let Err(err) = bot
                .send_message(chat_id, *response)
                .reply_to_message_id(msg.id)
                .await
            {
                log::warn!("*cat bait send failed* chat {}: {}", chat_id, err);
                handle_send_error(chat_id, &err, &chat_states).await;
            } else {
                clear_proactive_block(&chat_states, chat_id).await;
            }
        }
        return Ok(());
    }

    // Check if we're in playful mode - transition to annoyed if getting too many messages
    if state.mood == MomoMood::Playful {
        // Randomly transition to annoyed mode after responding in playful mode
        if rand::random::<f64>() < 0.2 {
            // 20% chance per message
            set_chat_mood(&chat_states, chat_id, MomoMood::Annoyed).await;
            log::info!("*mood change* Momo is now ANNOYED");

            // Generate annoyed response
            if let Some(user) = msg.from() {
                let username = user
                    .username
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or(&user.first_name);

                let chat_history: Vec<GrokMessage> = {
                    let history = conversation_history.lock().await;
                    history.get(&chat_id).cloned().unwrap_or_default()
                };

                if let Ok(annoyed_msg) = grok_client
                    .generate_annoyed_response(username, chat_history.as_slice())
                    .await
                {
                    if let Err(err) = bot
                        .send_message(chat_id, annoyed_msg)
                        .reply_to_message_id(msg.id)
                        .await
                    {
                        log::warn!("*annoyed send failed* chat {}: {}", chat_id, err);
                        handle_send_error(chat_id, &err, &chat_states).await;
                    } else {
                        clear_proactive_block(&chat_states, chat_id).await;
                    }
                    return Ok(());
                }
            }
        }
    }

    // Handle regular conversation with Grok
    log::info!("*packet received* from chat {}: {}", chat_id, text);

    // Send typing action
    if let Err(err) = bot.send_chat_action(chat_id, ChatAction::Typing).await {
        log::warn!("*typing action failed* chat {}: {}", chat_id, err);
    }

    // Get or create conversation history for this chat
    let user_content = if allow_multimodal && !attachments.is_empty() {
        let mut parts = vec![ContentPart::Text { text: text.clone() }];
        parts.extend(attachments);
        Content::Parts(parts)
    } else {
        Content::Text(text.clone())
    };

    let user_message = GrokMessage {
        role: "user".to_string(),
        content: user_content,
    };

    let chat_history_snapshot = {
        let mut history = conversation_history.lock().await;
        let chat_history = history.entry(chat_id).or_insert_with(Vec::new);

        // Add user message to history
        chat_history.push(user_message);

        // Keep only last N messages to manage context length
        if chat_history.len() > MAX_CONVERSATION_HISTORY {
            chat_history.drain(0..chat_history.len() - MAX_CONVERSATION_HISTORY);
        }

        chat_history.clone()
    };

    let context_hint = if is_group_or_channel(&msg.chat) {
        let ctx = get_recent_context(&recent_messages, chat_id, GROUP_CONTEXT_MESSAGES).await;
        if ctx.is_empty() {
            None
        } else {
            Some(ctx)
        }
    } else {
        None
    };

    // Get response from Grok
    match grok_client
        .chat(chat_history_snapshot.as_slice(), context_hint.as_deref())
        .await
    {
        Ok(response) => {
            // Add assistant response to history
            {
                let mut history = conversation_history.lock().await;
                let chat_history = history.entry(chat_id).or_insert_with(Vec::new);
                chat_history.push(GrokMessage::text("assistant", response.clone()));
            }

            log::info!("*response packet sent*: {}", response);
            if let Err(err) = bot
                .send_message(chat_id, response)
                .reply_to_message_id(msg.id)
                .await
            {
                log::warn!("*reply send failed* chat {}: {}", chat_id, err);
                handle_send_error(chat_id, &err, &chat_states).await;
            } else {
                clear_proactive_block(&chat_states, chat_id).await;
            }
        }
        Err(e) => {
            log::error!("*Grok core meltdown*: {}", e);
            let error_msg = random_choice(ERROR_RESPONSES);
            if let Err(err) = bot
                .send_message(chat_id, *error_msg)
                .reply_to_message_id(msg.id)
                .await
            {
                log::warn!("*error reply failed* chat {}: {}", chat_id, err);
                handle_send_error(chat_id, &err, &chat_states).await;
            }
        }
    }

    Ok(())
}

/// Pull user text/caption plus optional multimodal content (photos)
async fn extract_text_and_media(
    bot: &Bot,
    msg: &Message,
    allow_multimodal: bool,
) -> Option<(String, Vec<ContentPart>)> {
    let mut attachments: Vec<ContentPart> = Vec::new();

    if allow_multimodal {
        if let Some(photos) = msg.photo() {
            if let Some(best) = photos
                .iter()
                .max_by_key(|p| (p.height as u64) * (p.width as u64))
            {
                if let Ok(file) = bot.get_file(best.file.id.clone()).await {
                    let url = format!(
                        "https://api.telegram.org/file/bot{}/{}",
                        bot.token(),
                        file.path
                    );
                    attachments.push(ContentPart::ImageUrl {
                        image_url: ImageUrl {
                            url,
                            detail: Some("high".to_string()),
                        },
                    });
                } else {
                    log::warn!("*photo fetch failed* Could not fetch file for image message");
                }
            }
        }
    }

    let text = msg
        .text()
        .map(|t| t.to_string())
        .or_else(|| msg.caption().map(|c| c.to_string()))
        .or_else(|| {
            if msg.photo().is_some() {
                Some("User sent a photo".to_string())
            } else {
                None
            }
        })?;

    Some((text, attachments))
}

/// Determine if we should bypass mention checks because the message is baiting Momo
fn contains_cat_bait(text: &str) -> bool {
    let lower = text.to_lowercase();
    let bait_keywords = [
        "shrimp", "prawn", "🍤", "虾", "虾仁", "catnip", "snack", "treat", "rua", "摸摸", "pet me",
        "petting",
    ];

    bait_keywords.iter().any(|kw| {
        if kw.chars().any(|c| !c.is_ascii()) || kw.contains('🍤') {
            text.contains(kw)
        } else {
            lower.contains(kw)
        }
    })
}

/// Decide whether a Telegram error means the bot is not allowed to send proactively
fn should_pause_proactive(err: &RequestError) -> bool {
    match err {
        RequestError::Api(api_err) => {
            matches!(
                api_err,
                ApiError::BotBlocked
                    | ApiError::BotKicked
                    | ApiError::BotKickedFromSupergroup
                    | ApiError::ChatNotFound
                    | ApiError::UserDeactivated
            ) || api_err
                .to_string()
                .to_lowercase()
                .contains("initiate conversation with a user")
                || api_err.to_string().to_lowercase().contains("forbidden")
                || api_err
                    .to_string()
                    .to_lowercase()
                    .contains("not enough rights")
        }
        _ => err.to_string().to_lowercase().contains("forbidden"),
    }
}

/// Centralized handler for message send failures to keep the bot alive and respectful of Telegram rules
async fn handle_send_error(chat_id: ChatId, err: &RequestError, chat_states: &SharedChatStates) {
    if let RequestError::RetryAfter(delay) = err {
        log::warn!("*rate limited* chat {}: retry after {:?}", chat_id, delay);
        return;
    }

    if should_pause_proactive(err) {
        let cooldown = Duration::from_secs(PROACTIVE_BLOCK_COOLDOWN_SECS);
        pause_proactive(chat_states, chat_id, cooldown).await;
        log::warn!(
            "*proactive paused* chat {}: Telegram rejected sending, cooling down for {}s",
            chat_id,
            cooldown.as_secs()
        );
    }
}
