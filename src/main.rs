// Main entry point for the bot

mod chat;
mod commands;
mod config;
mod grok;
mod members;
mod prompts;
mod state;
mod tracking;
mod types;
mod utils;

use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use teloxide::prelude::*;
use tokio::sync::Mutex;

use grok::GrokClient;
use types::{SharedActiveMembers, SharedChatBots, SharedHistory, SharedRecentMessages};
use state::SharedChatStates;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("😼 *boot seq init* Momo v2.0 loading... *kernel panic check* meow.");

    let grok_api_key =
        env::var("GROK_API_KEY").expect("GROK_API_KEY environment variable must be set");

    let grok_client = Arc::new(GrokClient::new(grok_api_key));
    let conversation_history: SharedHistory = Arc::new(Mutex::new(HashMap::new()));
    let active_members: SharedActiveMembers = Arc::new(Mutex::new(HashMap::new()));
    let recent_messages: SharedRecentMessages = Arc::new(Mutex::new(HashMap::new()));
    let chat_bots: SharedChatBots = Arc::new(Mutex::new(HashMap::new()));
    let chat_states: SharedChatStates = Arc::new(Mutex::new(HashMap::new()));

    let bot = Bot::from_env();

    log::info!("🤖 *Grok core loaded* Momo online. Enhanced proactive mode active. Ping if you dare.");

    teloxide::repl(bot, move |bot: Bot, msg: Message| {
        let grok_client = Arc::clone(&grok_client);
        let conversation_history = Arc::clone(&conversation_history);
        let active_members = Arc::clone(&active_members);
        let recent_messages = Arc::clone(&recent_messages);
        let chat_bots = Arc::clone(&chat_bots);
        let chat_states = Arc::clone(&chat_states);

        async move {
            chat::handle_message(
                bot,
                msg,
                grok_client,
                conversation_history,
                active_members,
                recent_messages,
                chat_bots,
                chat_states,
            )
            .await
        }
    })
    .await;
}
