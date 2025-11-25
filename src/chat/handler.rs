/// Main message handler
use std::sync::Arc;
use std::time::Duration;
use teloxide::errors::{ApiError, RequestError};
use teloxide::prelude::*;
use teloxide::types::{ChatAction, InputFile, Message, MessageId};

use crate::commands::handle_command;
use crate::config::{
    ACTIVE_MESSAGE_CHANCE, CAT_BAIT_RESPONSE_CHANCE, DB_CONTEXT_SUMMARIES, GROUP_CONTEXT_MESSAGES,
    MAX_CONVERSATION_HISTORY, MAX_MEDIA_ATTACHMENTS, PROACTIVE_BLOCK_COOLDOWN_SECS,
    STICKER_REPLY_CHANCE,
};
use crate::db::{Database, DbEntry};
use crate::grok::{Content, ContentPart, GrokClient, ImageUrl, Message as GrokMessage};
use crate::media::to_data_url;
use crate::members::track_active_member;
use crate::prompts::{CAT_BAIT_RESPONSES, ERROR_RESPONSES};
use crate::state::{
    clear_proactive_block, get_chat_state, pause_proactive, set_chat_mood, MomoMood,
    SharedChatStates,
};
use crate::tracking::{get_recent_context, track_bot, track_message};
use crate::types::{
    SharedActiveMembers, SharedChatBots, SharedDb, SharedHistory, SharedRecentMessages,
};
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
    db: SharedDb,
) -> ResponseResult<()> {
    let allow_multimodal = grok_client.allows_multimodal();

    let (text, current_media) =
        match extract_text_and_media(&bot, &msg, allow_multimodal, true, &db).await {
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

    // Gather reply context if present
    let reply_context = if let Some(reply) = msg.reply_to_message() {
        extract_text_and_media(&bot, reply, allow_multimodal, true, &db).await
    } else {
        None
    };
    let reply_summary = reply_context.as_ref().and_then(|(t, media)| {
        format_reply_summary(reply_to_username(&msg), t, media.summary.as_deref())
    });

    // Combine attachments: include both reply context and current message
    let mut attachments = Vec::new();
    let mut attachment_file_ids = Vec::new();

    // Include reply context attachments first (higher priority)
    if let Some((_, reply_media)) = &reply_context {
        attachments.extend(reply_media.attachments.clone());
        attachment_file_ids.extend(reply_media.attachment_file_ids.clone());
    }

    // Then current message attachments
    attachments.extend(current_media.attachments.clone());
    attachment_file_ids.extend(current_media.attachment_file_ids.clone());

    if attachments.len() > MAX_MEDIA_ATTACHMENTS {
        attachments.truncate(MAX_MEDIA_ATTACHMENTS);
        attachment_file_ids.truncate(MAX_MEDIA_ATTACHMENTS);
    }

    log::debug!(
        "*media context* current: {} attachments, reply: {} attachments, total: {}",
        current_media.attachments.len(),
        reply_context
            .as_ref()
            .map(|(_, m)| m.attachments.len())
            .unwrap_or(0),
        attachments.len()
    );

    // Compose user text with reply hint for the model
    let mut composed_text = text.clone();
    if let Some(ref summary) = reply_summary {
        composed_text.push_str(&format!("\n\n[replying to {}]", summary));
    }

    // Persist to DB
    if let Some(user) = msg.from() {
        let username = format_username(user);
        let summary = build_db_summary(&username, &text, reply_summary.as_deref(), &current_media);
        let entry = DbEntry {
            chat_id: chat_id.0,
            message_id: msg.id.0,
            user_id: user.id.0 as i64,
            username,
            text: text.clone(),
            media_type: current_media.media_type.clone(),
            media_ref: current_media.media_ref.clone(),
            summary,
        };
        if let Err(err) = db.log_message(entry).await {
            log::warn!("*db log failed* chat {} msg {}: {}", chat_id, msg.id.0, err);
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
                maybe_send_sticker_reply(&bot, chat_id, db.as_ref(), &chat_states).await;
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
    log::info!("*packet received* from chat {}: {}", chat_id, composed_text);

    // Send typing action
    if let Err(err) = bot.send_chat_action(chat_id, ChatAction::Typing).await {
        log::warn!("*typing action failed* chat {}: {}", chat_id, err);
    }

    // Get or create conversation history for this chat
    let user_content = if allow_multimodal && !attachments.is_empty() {
        // Build content with text and images
        let mut parts = vec![ContentPart::Text {
            text: composed_text.clone(),
        }];

        // Include all attachments (reply context + current message, already deduplicated)
        for attachment in &attachments {
            parts.push(attachment.clone());
        }

        log::info!(
            "*multimodal request* {} text parts, {} image parts",
            1,
            attachments.len()
        );

        Content::Parts(parts)
    } else {
        Content::Text(composed_text.clone())
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

    let context_hint = {
        let mut ctx_pieces = Vec::new();
        if let Ok(db_ctx) = db.recent_summaries(chat_id.0, DB_CONTEXT_SUMMARIES).await {
            if !db_ctx.is_empty() {
                ctx_pieces.push(db_ctx.join("\n"));
            }
        }
        if is_group_or_channel(&msg.chat) {
            let mem_ctx =
                get_recent_context(&recent_messages, chat_id, GROUP_CONTEXT_MESSAGES).await;
            if !mem_ctx.is_empty() {
                ctx_pieces.push(mem_ctx);
            }
        }
        (!ctx_pieces.is_empty()).then(|| ctx_pieces.join("\n"))
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
                maybe_send_sticker_reply(&bot, chat_id, db.as_ref(), &chat_states).await;
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

#[derive(Clone, Default)]
struct MediaCapture {
    attachments: Vec<ContentPart>,
    attachment_file_ids: Vec<String>, // Track file_ids for cache lookups
    media_type: Option<String>,
    media_ref: Option<String>,
    summary: Option<String>,
}

async fn extract_text_and_media(
    bot: &Bot,
    msg: &Message,
    allow_multimodal: bool,
    allow_media: bool,
    db: &Database,
) -> Option<(String, MediaCapture)> {
    let mut media = MediaCapture::default();
    let sender = msg
        .from()
        .map(|u| format_username(u))
        .unwrap_or_else(|| "someone".to_string());

    if let Some(sticker) = msg.sticker() {
        media.media_type = Some("sticker".to_string());
        media.media_ref = Some(sticker.file.id.clone());
        let emoji = sticker.emoji.clone().unwrap_or_else(|| "😼".to_string());
        media.summary = Some(format!("sticker {} from @{}", emoji, sender));

        let is_raster = !sticker.is_animated() && !sticker.is_video();
        if allow_multimodal && allow_media && is_raster {
            if let Some(part) = build_image_attachment(bot, &sticker.file.id, db).await {
                media.attachment_file_ids.push(sticker.file.id.clone());
                media.attachments.push(part);
            }
        }
    } else if let Some(photos) = msg.photo() {
        if let Some(best) = photos
            .iter()
            .max_by_key(|p| (p.height as u64) * (p.width as u64))
        {
            media.media_type = Some("photo".to_string());
            media.media_ref = Some(best.file.id.clone());
            if allow_multimodal && allow_media {
                if let Some(part) = build_image_attachment(bot, &best.file.id, db).await {
                    media.attachment_file_ids.push(best.file.id.clone());
                    media.attachments.push(part);
                }
            }
            let caption = msg.caption().unwrap_or("");
            media.summary = Some(if caption.is_empty() {
                format!("photo from @{}", sender)
            } else {
                format!("photo from @{} (caption: {})", sender, caption)
            });
        }
    }

    let text = msg
        .text()
        .map(|t| t.to_string())
        .or_else(|| msg.caption().map(|c| c.to_string()))
        .or_else(|| {
            if let Some(summary) = &media.summary {
                Some(summary.clone())
            } else {
                None
            }
        })?;

    Some((text, media))
}

async fn build_image_attachment(bot: &Bot, file_id: &str, db: &Database) -> Option<ContentPart> {
    // Check cache first
    if let Ok(Some(cached)) = db.get_cached_image(file_id).await {
        log::debug!("*image cache hit* file_id: {}", file_id);
        return Some(ContentPart::ImageUrl {
            image_url: ImageUrl {
                url: cached.data_url,
                detail: Some("high".to_string()),
            },
        });
    }

    // Download and convert to data URL
    let (mime_type, data_url) = match to_data_url(bot, file_id).await {
        Some(result) => result,
        None => {
            log::warn!("*image conversion failed* file_id: {}", file_id);
            return None;
        }
    };

    // Cache the result
    if let Err(e) = db.cache_image(file_id, &mime_type, &data_url).await {
        log::warn!("*cache write failed* file_id: {}: {}", file_id, e);
    }

    log::info!(
        "*image processed* file_id: {}, mime: {}, data_url_prefix: {}...",
        file_id,
        mime_type,
        &data_url[..data_url.len().min(60)]
    );

    Some(ContentPart::ImageUrl {
        image_url: ImageUrl {
            url: data_url,
            detail: Some("high".to_string()),
        },
    })
}

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

fn format_username(user: &teloxide::types::User) -> String {
    user.username
        .clone()
        .unwrap_or_else(|| user.first_name.clone())
}

fn reply_to_username(msg: &Message) -> Option<String> {
    msg.reply_to_message()
        .and_then(|m| m.from())
        .map(|u| format_username(u))
}

fn format_reply_summary(
    reply_user: Option<String>,
    text: &str,
    media_summary: Option<&str>,
) -> Option<String> {
    let who = reply_user.unwrap_or_else(|| "someone".to_string());
    let snippet = truncate_for_context(text, 160);
    if let Some(media) = media_summary {
        Some(format!("@{}: {} ({})", who, snippet, media))
    } else {
        Some(format!("@{}: {}", who, snippet))
    }
}

fn build_db_summary(
    username: &str,
    text: &str,
    reply_summary: Option<&str>,
    media: &MediaCapture,
) -> String {
    let mut parts = Vec::new();
    parts.push(format!(
        "@{}: {}",
        username,
        truncate_for_context(text, 180)
    ));
    if let Some(reply) = reply_summary {
        parts.push(format!("[reply to {}]", reply));
    }
    if let Some(media_s) = &media.summary {
        parts.push(format!("[media: {}]", media_s));
    }
    parts.join(" ")
}

async fn maybe_send_sticker_reply(
    bot: &Bot,
    chat_id: ChatId,
    db: &Database,
    chat_states: &SharedChatStates,
) {
    let state = get_chat_state(chat_states, chat_id).await;

    // Adjust probability based on mood
    let base_chance = STICKER_REPLY_CHANCE;
    let adjusted_chance = match state.mood {
        crate::state::MomoMood::Playful => base_chance * 1.5, // More likely when playful
        crate::state::MomoMood::Annoyed => base_chance * 0.5, // Less likely when annoyed
        crate::state::MomoMood::Busy => 0.0,                  // Never when busy
        crate::state::MomoMood::Normal => base_chance,
    };

    let adjusted_chance = adjusted_chance.min(1.0); // Cap at 1.0

    if rand::random::<f64>() > adjusted_chance {
        return;
    }

    if let Ok(Some(sticker_id)) = db.random_sticker(chat_id.0).await {
        if let Err(err) = bot
            .send_sticker(chat_id, InputFile::file_id(sticker_id))
            .await
        {
            log::warn!("*sticker send failed* chat {}: {}", chat_id, err);
        } else {
            log::debug!("*sticker sent* chat {} (mood: {:?})", chat_id, state.mood);
        }
    }
}

fn truncate_for_context(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max).collect();
        format!("{}…", truncated)
    }
}
