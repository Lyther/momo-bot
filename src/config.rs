/// Configuration constants for the bot

/// Chance (0.0-1.0) for proactive message in groups
/// Current: 0.025 = 2.5% = 1 in 40 messages on average
/// For very active chats (500 msg/hr): ~12.5 proactive/hr
/// For moderate chats (100 msg/hr): ~2.5 proactive/hr
pub const ACTIVE_MESSAGE_CHANCE: f64 = 0.025; // 2.5% - less annoying

/// Maximum number of active members to track per chat
pub const MAX_ACTIVE_MEMBERS_TRACKED: usize = 15;

/// Maximum conversation history to keep (messages)
pub const MAX_CONVERSATION_HISTORY: usize = 20;

/// Maximum recent messages to track per chat
pub const MAX_RECENT_MESSAGES: usize = 30;

/// Minimum active members required before proactive engagement
pub const MIN_MEMBERS_FOR_PROACTIVE: usize = 3;

/// Number of recent members to pick from for proactive mentions
pub const PROACTIVE_MEMBER_POOL: usize = 5;

/// Proactive behavior type chances
/// High chance: just say something naturally
pub const CHANCE_NATURAL_COMMENT: f64 = 0.50; // 50%
/// Medium chance: reply to a message
pub const CHANCE_REPLY_TO_MESSAGE: f64 = 0.30; // 30%
/// Low chance: mention and judge a user
pub const CHANCE_MENTION_USER: f64 = 0.15; // 15%
/// Very low: bot interaction
pub const CHANCE_BOT_INTERACTION: f64 = 0.03; // 3%
/// Remaining 2% goes to random fallback behavior

/// Time-based proactive behavior
/// Minimum and maximum seconds between time-based proactive messages
/// Bot will randomly pick interval between these values for natural variation
pub const MIN_TIME_BETWEEN_PROACTIVE: u64 = 14400; // 4 hours (4 * 60 * 60)
pub const MAX_TIME_BETWEEN_PROACTIVE: u64 = 43200; // 12 hours (12 * 60 * 60)
/// Chance for cat behavior post when doing time-based proactive
pub const CHANCE_CAT_BEHAVIOR: f64 = 0.3; // 30%
/// Chance for playful mode initiation
pub const CHANCE_PLAYFUL_MODE: f64 = 0.15; // 15%

/// Number of recent messages from a user to judge
pub const USER_MESSAGES_TO_JUDGE: usize = 5;

/// Grok API configuration
pub const GROK_API_URL: &str = "https://api.x.ai/v1/chat/completions";
pub const GROK_MODEL: &str = "grok-4-fast-reasoning";
pub const GROK_TEMPERATURE: f32 = 0.8;
pub const GROK_TEMPERATURE_PROACTIVE: f32 = 1.0;
pub const GROK_MAX_TOKENS: u32 = 1024;
