/// Active/proactive chat features - Momo initiating conversations

use std::sync::Arc;
use std::time::Instant;
use teloxide::prelude::*;

use crate::config::*;
use crate::grok::GrokClient;
use crate::members::get_random_active_member;
use crate::state::{get_chat_state, set_chat_mood, MomoMood, SharedChatStates};
use crate::tracking::*;
use crate::types::{SharedActiveMembers, SharedChatBots, SharedHistory, SharedRecentMessages};

/// Proactive behavior types
#[derive(Debug)]
pub enum ProactiveBehavior {
    /// Natural comment without mentioning anyone
    NaturalComment,
    /// Reply to a random recent message
    ReplyToMessage,
    /// Mention a user and judge their recent messages
    MentionUser,
    /// Interact with another bot
    BotInteraction,
    /// Time-based: random conversation starter
    TimeRandomStarter,
    /// Time-based: news-based conversation starter
    TimeNewsStarter,
    /// Cat behavior post (changes to Busy mode)
    CatBehavior,
    /// Playful mode initiation (changes to Playful mode)
    PlayfulMode,
}

/// Select which proactive behavior to use based on probabilities
fn select_proactive_behavior(is_time_based: bool) -> ProactiveBehavior {
    let roll = rand::random::<f64>();

    if is_time_based {
        // Time-based behaviors
        let mut cumulative = 0.0;

        cumulative += CHANCE_CAT_BEHAVIOR;
        if roll < cumulative {
            return ProactiveBehavior::CatBehavior;
        }

        cumulative += CHANCE_PLAYFUL_MODE;
        if roll < cumulative {
            return ProactiveBehavior::PlayfulMode;
        }

        // 50% chance for random starter, 50% for news starter
        if roll < 0.5 {
            return ProactiveBehavior::TimeRandomStarter;
        } else {
            return ProactiveBehavior::TimeNewsStarter;
        }
    }

    // Regular message-triggered behaviors
    let mut cumulative = 0.0;

    cumulative += CHANCE_NATURAL_COMMENT;
    if roll < cumulative {
        return ProactiveBehavior::NaturalComment;
    }

    cumulative += CHANCE_REPLY_TO_MESSAGE;
    if roll < cumulative {
        return ProactiveBehavior::ReplyToMessage;
    }

    cumulative += CHANCE_MENTION_USER;
    if roll < cumulative {
        return ProactiveBehavior::MentionUser;
    }

    cumulative += CHANCE_BOT_INTERACTION;
    if roll < cumulative {
        return ProactiveBehavior::BotInteraction;
    }

    // Default fallback
    ProactiveBehavior::TimeRandomStarter
}

/// Check if it's time for time-based proactive behavior
async fn should_do_time_based_proactive(
    chat_states: &SharedChatStates,
    chat_id: ChatId,
) -> bool {
    let mut state = get_chat_state(chat_states, chat_id).await;

    // If no next time scheduled, schedule one and return false (wait for next time)
    if state.next_proactive_time.is_none() {
        state.schedule_next_proactive();
        crate::state::update_chat_state(chat_states, chat_id, state).await;
        return false;
    }

    // Check if scheduled time has arrived
    if let Some(next_time) = state.next_proactive_time {
        Instant::now() >= next_time
    } else {
        false
    }
}

/// Generate a proactive message based on behavior type
pub async fn generate_proactive_message(
    grok_client: &Arc<GrokClient>,
    active_members: SharedActiveMembers,
    conversation_history: SharedHistory,
    recent_messages: SharedRecentMessages,
    chat_bots: SharedChatBots,
    chat_states: SharedChatStates,
    chat_id: ChatId,
    my_bot_id: UserId,
) -> Option<(String, Option<i32>)> {
    // Check current mood
    let state = get_chat_state(&chat_states, chat_id).await;
    if !state.should_respond() {
        log::debug!("*mood block* Momo is busy, skipping proactive message");
        return None;
    }

    // Check if we should do time-based proactive
    let is_time_based = should_do_time_based_proactive(&chat_states, chat_id).await;

    // Select behavior type
    let behavior = select_proactive_behavior(is_time_based);
    log::info!("*proactive behavior selected* {:?}", behavior);

    // Get conversation context
    let mut context_history = conversation_history.lock().await;
    let chat_context = context_history.entry(chat_id).or_insert_with(Vec::new);

    // Generate message based on behavior type
    let result = match behavior {
        ProactiveBehavior::NaturalComment => {
            generate_natural_comment(grok_client, &recent_messages, chat_context, chat_id).await
        }

        ProactiveBehavior::ReplyToMessage => {
            generate_reply_to_message(
                grok_client,
                &recent_messages,
                chat_context,
                chat_id,
                my_bot_id,
            )
            .await
        }

        ProactiveBehavior::MentionUser => {
            generate_mention_user(
                grok_client,
                &active_members,
                &recent_messages,
                chat_context,
                chat_id,
            )
            .await
        }

        ProactiveBehavior::BotInteraction => {
            generate_bot_interaction(grok_client, &chat_bots, chat_context, chat_id, my_bot_id)
                .await
        }

        ProactiveBehavior::TimeRandomStarter => {
            generate_time_random_starter(grok_client, chat_context).await
        }

        ProactiveBehavior::TimeNewsStarter => {
            generate_time_news_starter(grok_client, &recent_messages, chat_context, chat_id).await
        }

        ProactiveBehavior::CatBehavior => {
            // Set busy mode
            set_chat_mood(&chat_states, chat_id, MomoMood::Busy).await;
            log::info!("*mood change* Momo is now BUSY (cat behavior)");
            generate_cat_behavior(grok_client, chat_context).await
        }

        ProactiveBehavior::PlayfulMode => {
            // Set playful mode
            set_chat_mood(&chat_states, chat_id, MomoMood::Playful).await;
            log::info!("*mood change* Momo is now PLAYFUL");
            generate_playful_mode(grok_client, chat_context).await
        }
    };

    // If we generated a message, add it to history
    if let Some((msg, _reply_to)) = &result {
        chat_context.push(crate::grok::Message {
            role: "assistant".to_string(),
            content: msg.clone(),
        });

        // Update proactive times for time-based behaviors
        if is_time_based {
            let mut state = get_chat_state(&chat_states, chat_id).await;
            state.last_proactive_time = Some(Instant::now());
            // Schedule the next proactive message (random 4-12 hours from now)
            state.schedule_next_proactive();
            crate::state::update_chat_state(&chat_states, chat_id, state).await;
        }

        log::info!("*proactive message generated* {:?}: {}", behavior, msg);
    }

    result
}

// Individual behavior generators

async fn generate_natural_comment(
    grok_client: &Arc<GrokClient>,
    recent_messages: &SharedRecentMessages,
    chat_context: &[crate::grok::Message],
    chat_id: ChatId,
) -> Option<(String, Option<i32>)> {
    let context = get_recent_context(recent_messages, chat_id, 8).await;
    if context.is_empty() {
        return None;
    }

    match grok_client
        .generate_natural_comment(&context, chat_context)
        .await
    {
        Ok(msg) => Some((msg, None)),
        Err(e) => {
            log::error!("*natural comment gen failed*: {}", e);
            None
        }
    }
}

async fn generate_reply_to_message(
    grok_client: &Arc<GrokClient>,
    recent_messages: &SharedRecentMessages,
    chat_context: &[crate::grok::Message],
    chat_id: ChatId,
    my_bot_id: UserId,
) -> Option<(String, Option<i32>)> {
    let (msg_id, username, text) =
        get_random_replyable_message(recent_messages, chat_id, my_bot_id).await?;

    match grok_client
        .generate_reply_to_message(&username, &text, chat_context)
        .await
    {
        Ok(msg) => Some((msg, Some(msg_id))),
        Err(e) => {
            log::error!("*reply gen failed*: {}", e);
            None
        }
    }
}

async fn generate_mention_user(
    grok_client: &Arc<GrokClient>,
    active_members: &SharedActiveMembers,
    recent_messages: &SharedRecentMessages,
    chat_context: &[crate::grok::Message],
    chat_id: ChatId,
) -> Option<(String, Option<i32>)> {
    // Get a random active member
    let username = get_random_active_member(
        active_members,
        chat_id,
        MIN_MEMBERS_FOR_PROACTIVE,
        PROACTIVE_MEMBER_POOL,
    )
    .await?;

    // Get their recent messages
    // We need to find the user_id from recent messages
    let messages_lock = recent_messages.lock().await;
    let chat_messages = messages_lock.get(&chat_id)?;

    let user_id = chat_messages
        .iter()
        .find(|msg| msg.username == username && !msg.is_bot)
        .map(|msg| msg.user_id)?;

    drop(messages_lock);

    let user_messages = get_user_recent_messages(
        recent_messages,
        chat_id,
        user_id,
        USER_MESSAGES_TO_JUDGE,
    )
    .await;

    if user_messages.is_empty() {
        return None;
    }

    match grok_client
        .generate_mention_user_message(&username, &user_messages, chat_context)
        .await
    {
        Ok(msg) => Some((msg, None)),
        Err(e) => {
            log::error!("*mention user gen failed*: {}", e);
            None
        }
    }
}

async fn generate_bot_interaction(
    grok_client: &Arc<GrokClient>,
    chat_bots: &SharedChatBots,
    chat_context: &[crate::grok::Message],
    chat_id: ChatId,
    my_bot_id: UserId,
) -> Option<(String, Option<i32>)> {
    let bot_username = get_random_bot(chat_bots, chat_id, my_bot_id).await?;

    match grok_client
        .generate_bot_interaction(&bot_username, chat_context)
        .await
    {
        Ok(msg) => Some((msg, None)),
        Err(e) => {
            log::error!("*bot interaction gen failed*: {}", e);
            None
        }
    }
}

async fn generate_time_random_starter(
    grok_client: &Arc<GrokClient>,
    chat_context: &[crate::grok::Message],
) -> Option<(String, Option<i32>)> {
    match grok_client.generate_random_starter(chat_context).await {
        Ok(msg) => Some((msg, None)),
        Err(e) => {
            log::error!("*random starter gen failed*: {}", e);
            None
        }
    }
}

async fn generate_time_news_starter(
    grok_client: &Arc<GrokClient>,
    recent_messages: &SharedRecentMessages,
    chat_context: &[crate::grok::Message],
    chat_id: ChatId,
) -> Option<(String, Option<i32>)> {
    let topic = extract_topic_from_recent(recent_messages, chat_id).await?;

    match grok_client
        .generate_news_starter(&topic, chat_context)
        .await
    {
        Ok(msg) => Some((msg, None)),
        Err(e) => {
            log::error!("*news starter gen failed*: {}", e);
            None
        }
    }
}

async fn generate_cat_behavior(
    grok_client: &Arc<GrokClient>,
    chat_context: &[crate::grok::Message],
) -> Option<(String, Option<i32>)> {
    match grok_client.generate_cat_behavior(chat_context).await {
        Ok(msg) => Some((msg, None)),
        Err(e) => {
            log::error!("*cat behavior gen failed*: {}", e);
            None
        }
    }
}

async fn generate_playful_mode(
    grok_client: &Arc<GrokClient>,
    chat_context: &[crate::grok::Message],
) -> Option<(String, Option<i32>)> {
    match grok_client.generate_playful_mode(chat_context).await {
        Ok(msg) => Some((msg, None)),
        Err(e) => {
            log::error!("*playful mode gen failed*: {}", e);
            None
        }
    }
}
