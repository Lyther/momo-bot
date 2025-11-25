/// Shared type definitions
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use teloxide::prelude::*;
use tokio::sync::Mutex;

use crate::db::Database;
use crate::grok::Message as GrokMessage;

/// Conversation history storage: ChatId -> Messages
pub type ConversationHistory = HashMap<ChatId, Vec<GrokMessage>>;

/// Thread-safe shared conversation history
pub type SharedHistory = Arc<Mutex<ConversationHistory>>;

/// Active members tracking: ChatId -> (UserId, Username) queue
pub type ActiveMembers = HashMap<ChatId, VecDeque<(UserId, String)>>;

/// Thread-safe shared active members
pub type SharedActiveMembers = Arc<Mutex<ActiveMembers>>;

/// Stored message info for tracking recent messages
#[derive(Debug, Clone)]
pub struct StoredMessage {
    pub message_id: i32,
    pub user_id: UserId,
    pub username: String,
    pub text: String,
    pub is_bot: bool,
}

/// Recent messages tracking: ChatId -> Message queue
pub type RecentMessages = HashMap<ChatId, VecDeque<StoredMessage>>;

/// Thread-safe shared recent messages
pub type SharedRecentMessages = Arc<Mutex<RecentMessages>>;

/// Bot users in chat: ChatId -> Set of (UserId, Username)
pub type ChatBots = HashMap<ChatId, Vec<(UserId, String)>>;

/// Thread-safe shared bot tracking
pub type SharedChatBots = Arc<Mutex<ChatBots>>;

/// Shared database handle
pub type SharedDb = Arc<Database>;
