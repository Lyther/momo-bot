/// Main message handler

use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{ChatAction, Message, MessageId};

use crate::commands::handle_command;
use crate::config::{ACTIVE_MESSAGE_CHANCE, MAX_CONVERSATION_HISTORY};
use crate::grok::{GrokClient, Message as GrokMessage};
use crate::members::track_active_member;
use crate::prompts::ERROR_RESPONSES;
use crate::state::{get_chat_state, set_chat_mood, MomoMood, SharedChatStates};
use crate::tracking::{track_bot, track_message};
use crate::types::{
    SharedActiveMembers, SharedChatBots, SharedHistory, SharedRecentMessages,
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

    // Track message
    if let Some(user) = msg.from() {
        track_message(
            recent_messages.clone(),
            chat_id,
            msg.id.0,
            user,
            text,
        )
        .await;

        // Track bots
        if user.is_bot {
            track_bot(chat_bots.clone(), chat_id, user).await;
        } else {
            // Track active human members
            track_active_member(active_members.clone(), chat_id, user).await;
        }
    }

    // Get bot's own user ID
    let my_bot_id = bot.get_me().await?.user.id;

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

                send_msg.await?;
                return Ok(());
            }
        }
    }

    // Determine if we should respond based on chat type (explicit mentions only)
    let should_respond = should_respond_to_message(&bot, &msg).await;

    if !should_respond {
        return Ok(());
    }

    // Check if we're in playful mode - transition to annoyed if getting too many messages
    let state = get_chat_state(&chat_states, chat_id).await;
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

                let history = conversation_history.lock().await;
                let chat_history = history.get(&chat_id).map(|v| v.as_slice()).unwrap_or(&[]);

                if let Ok(annoyed_msg) = grok_client
                    .generate_annoyed_response(username, chat_history)
                    .await
                {
                    bot.send_message(chat_id, annoyed_msg)
                        .reply_to_message_id(msg.id)
                        .await?;
                    return Ok(());
                }
            }
        }
    }

    // Handle regular conversation with Grok
    log::info!("*packet received* from chat {}: {}", chat_id, text);

    // Send typing action
    bot.send_chat_action(chat_id, ChatAction::Typing).await?;

    // Get or create conversation history for this chat
    let mut history = conversation_history.lock().await;
    let chat_history = history.entry(chat_id).or_insert_with(Vec::new);

    // Add user message to history
    chat_history.push(GrokMessage {
        role: "user".to_string(),
        content: text.to_string(),
    });

    // Keep only last N messages to manage context length
    if chat_history.len() > MAX_CONVERSATION_HISTORY {
        chat_history.drain(0..chat_history.len() - MAX_CONVERSATION_HISTORY);
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
            let error_msg = random_choice(ERROR_RESPONSES);
            bot.send_message(chat_id, *error_msg)
                .reply_to_message_id(msg.id)
                .await?;
        }
    }

    Ok(())
}
