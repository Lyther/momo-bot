// Main entry point for the bot

mod chat;
mod commands;
mod config;
mod db;
mod grok;
mod media;
mod members;
mod prompts;
mod server;
mod state;
mod tracking;
mod types;
mod utils;

use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use teloxide::prelude::*;
use tokio::sync::Mutex;
use tokio::time::sleep;

use grok::GrokClient;
use state::SharedChatStates;
use types::{SharedActiveMembers, SharedChatBots, SharedDb, SharedHistory, SharedRecentMessages};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("😼 *boot seq init* Momo v2.0 loading... *kernel panic check* meow.");

    let grok_api_key =
        env::var("GROK_API_KEY").expect("GROK_API_KEY environment variable must be set");

    let grok_client = Arc::new(GrokClient::new(grok_api_key));
    log::info!(
        "🤖 Grok model: {}, proactive: {}, search: {}, multimodal: {}",
        grok_client.chat_model_name(),
        grok_client.proactive_model_name(),
        grok_client.search_enabled(),
        grok_client.allows_multimodal()
    );
    let db_path = env::var("MOMO_DB_PATH").unwrap_or_else(|_| "momo.db".to_string());
    let db = Arc::new(db::Database::new(&db_path).await.unwrap_or_else(|e| {
        panic!("Failed to open database at {}: {}", db_path, e);
    }));
    let conversation_history: SharedHistory = Arc::new(Mutex::new(HashMap::new()));
    let active_members: SharedActiveMembers = Arc::new(Mutex::new(HashMap::new()));
    let recent_messages: SharedRecentMessages = Arc::new(Mutex::new(HashMap::new()));
    let chat_bots: SharedChatBots = Arc::new(Mutex::new(HashMap::new()));
    let chat_states: SharedChatStates = Arc::new(Mutex::new(HashMap::new()));

    let bot = Bot::from_env();

    log::info!(
        "🤖 *Grok core loaded* Momo online. Enhanced proactive mode active. Ping if you dare."
    );

    // Start HTTP server to serve cached images with correct content-types
    // Grok downloads from URLs, and Telegram serves everything as application/octet-stream
    tokio::spawn(async move {
        if let Err(e) = server::start_server().await {
            log::error!("*image server error*: {}", e);
        }
    });

    // Clean up orphaned cache files
    if let Err(e) = db
        .cleanup_orphaned_cache(std::path::Path::new("assets/cache/images"))
        .await
    {
        log::warn!("*cache cleanup failed*: {}", e);
    }

    // METHOD 1: Robust error handling with automatic reconnection
    run_with_retry(
        bot,
        grok_client,
        conversation_history,
        active_members,
        recent_messages,
        chat_bots,
        chat_states,
        db,
    )
    .await;
}

/// Run the bot with automatic reconnection on network errors
async fn run_with_retry(
    bot: Bot,
    grok_client: Arc<GrokClient>,
    conversation_history: SharedHistory,
    active_members: SharedActiveMembers,
    recent_messages: SharedRecentMessages,
    chat_bots: SharedChatBots,
    chat_states: SharedChatStates,
    db: SharedDb,
) {
    let mut retry_count = 0u32;
    let max_retry_delay = Duration::from_secs(300); // 5 minutes max

    loop {
        log::info!(
            "*connection attempt* Starting update listener (attempt {})...",
            retry_count + 1
        );

        // Use Dispatcher instead of repl for better error control
        let result = run_bot_with_dispatcher(
            bot.clone(),
            grok_client.clone(),
            conversation_history.clone(),
            active_members.clone(),
            recent_messages.clone(),
            chat_bots.clone(),
            chat_states.clone(),
            db.clone(),
        )
        .await;

        // Always treat as error and retry
        retry_count += 1;

        // Exponential backoff: 2^min(retry_count, 8) seconds, capped at max_retry_delay
        let delay_secs = 2u64.saturating_pow(retry_count.min(8));
        let delay = Duration::from_secs(delay_secs).min(max_retry_delay);

        log::error!("*network failure* Bot disconnected: {:?}", result);
        log::warn!(
            "*reconnection scheduled* Retrying in {} seconds... (attempt {})",
            delay.as_secs(),
            retry_count
        );

        // Reset retry count after 10 successful minutes
        if retry_count > 5 {
            retry_count = retry_count.saturating_sub(1);
        }

        sleep(delay).await;
    }
}

/// Run the bot using Dispatcher for more control
async fn run_bot_with_dispatcher(
    bot: Bot,
    grok_client: Arc<GrokClient>,
    conversation_history: SharedHistory,
    active_members: SharedActiveMembers,
    recent_messages: SharedRecentMessages,
    chat_bots: SharedChatBots,
    chat_states: SharedChatStates,
    db: SharedDb,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Use Dispatcher with update handler
    Dispatcher::builder(
        bot.clone(),
        Update::filter_message().endpoint(move |bot: Bot, msg: Message| {
            let grok_client = grok_client.clone();
            let conversation_history = conversation_history.clone();
            let active_members = active_members.clone();
            let recent_messages = recent_messages.clone();
            let chat_bots = chat_bots.clone();
            let chat_states = chat_states.clone();
            let db = db.clone();

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
                    db,
                )
                .await
            }
        }),
    )
    .enable_ctrlc_handler()
    .build()
    .dispatch()
    .await;

    Ok(())
}
