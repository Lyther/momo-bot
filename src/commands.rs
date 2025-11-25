/// Command handlers for Momo bot
use teloxide::prelude::*;
use teloxide::types::ChatId;

use crate::prompts::*;
use crate::types::SharedHistory;
use crate::utils::random_choice;

/// Handle bot commands
pub async fn handle_command(
    bot: Bot,
    chat_id: ChatId,
    text: &str,
    conversation_history: SharedHistory,
) -> ResponseResult<()> {
    // Parse command and check if it's for this bot
    // In groups with multiple bots, commands can be like /start@momo_mycat_bot
    let parts: Vec<&str> = text.split('@').collect();
    let command = parts[0];

    // If command has @username, check if it's for us
    if parts.len() > 1 {
        let target_bot = parts[1];
        let me = bot.get_me().await?;
        let my_username = me.user.username.as_deref().unwrap_or("");

        // If command is for a different bot, ignore it
        if target_bot != my_username {
            log::debug!("Command {} is for @{}, ignoring", command, target_bot);
            return Ok(());
        }
    }

    match command {
        "/start" => {
            let response = random_choice(START_RESPONSES);
            send_response(&bot, chat_id, *response).await;
        }
        "/help" => {
            send_response(&bot, chat_id, HELP_TEXT).await;
        }
        "/about" => {
            send_response(&bot, chat_id, ABOUT_TEXT).await;
        }
        "/reset" => {
            let mut history = conversation_history.lock().await;
            history.remove(&chat_id);
            let response = random_choice(RESET_RESPONSES);
            send_response(&bot, chat_id, *response).await;
        }
        "/status" => {
            let history = conversation_history.lock().await;
            let count = history.get(&chat_id).map(|h| h.len()).unwrap_or(0);
            let response = status_response(count);
            send_response(&bot, chat_id, response).await;
        }
        _ => {
            let response = random_choice(UNKNOWN_COMMAND_RESPONSES);
            send_response(&bot, chat_id, *response).await;
        }
    }

    Ok(())
}

/// Fire-and-forget command responses; log errors instead of bubbling up
async fn send_response(bot: &Bot, chat_id: ChatId, text: impl Into<String>) {
    if let Err(err) = bot.send_message(chat_id, text).await {
        log::warn!("*command send failed* chat {}: {}", chat_id, err);
    }
}
