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
    match text {
        "/start" => {
            let response = random_choice(START_RESPONSES);
            bot.send_message(chat_id, *response).await?;
        }
        "/help" => {
            bot.send_message(chat_id, HELP_TEXT).await?;
        }
        "/about" => {
            bot.send_message(chat_id, ABOUT_TEXT).await?;
        }
        "/reset" => {
            let mut history = conversation_history.lock().await;
            history.remove(&chat_id);
            let response = random_choice(RESET_RESPONSES);
            bot.send_message(chat_id, *response).await?;
        }
        "/status" => {
            let history = conversation_history.lock().await;
            let count = history.get(&chat_id).map(|h| h.len()).unwrap_or(0);
            let response = status_response(count);
            bot.send_message(chat_id, response).await?;
        }
        _ => {
            let response = random_choice(UNKNOWN_COMMAND_RESPONSES);
            bot.send_message(chat_id, *response).await?;
        }
    }

    Ok(())
}
