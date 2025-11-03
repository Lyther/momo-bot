/// Bot state management - tracking Momo's moods and modes

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use teloxide::prelude::*;
use tokio::sync::Mutex;

/// Momo's current mood/mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MomoMood {
    /// Default tsundere mode
    Normal,
    /// Cat is busy doing cat things, won't respond
    Busy,
    /// Wants to play and interact
    Playful,
    /// Annoyed by too much interaction
    Annoyed,
}

impl MomoMood {
    /// Get typical duration for this mood
    pub fn duration(&self) -> Duration {
        match self {
            MomoMood::Normal => Duration::from_secs(0), // No timer
            MomoMood::Busy => Duration::from_secs(180), // 3 minutes
            MomoMood::Playful => Duration::from_secs(120), // 2 minutes
            MomoMood::Annoyed => Duration::from_secs(300), // 5 minutes
        }
    }
}

/// State tracking for a chat
#[derive(Debug, Clone)]
pub struct ChatState {
    pub mood: MomoMood,
    pub mood_expires_at: Option<Instant>,
    pub last_proactive_time: Option<Instant>,
    pub next_proactive_time: Option<Instant>,
}

impl ChatState {
    pub fn new() -> Self {
        Self {
            mood: MomoMood::Normal,
            mood_expires_at: None,
            last_proactive_time: None,
            next_proactive_time: None,
        }
    }

    /// Generate a random interval between MIN and MAX time
    fn random_proactive_interval() -> Duration {
        use crate::config::{MIN_TIME_BETWEEN_PROACTIVE, MAX_TIME_BETWEEN_PROACTIVE};
        let min = MIN_TIME_BETWEEN_PROACTIVE;
        let max = MAX_TIME_BETWEEN_PROACTIVE;
        let range = max - min;
        let random_offset = (rand::random::<f64>() * range as f64) as u64;
        Duration::from_secs(min + random_offset)
    }

    /// Set the next proactive time with random interval
    pub fn schedule_next_proactive(&mut self) {
        let interval = Self::random_proactive_interval();
        self.next_proactive_time = Some(Instant::now() + interval);

        let hours = interval.as_secs() / 3600;
        let minutes = (interval.as_secs() % 3600) / 60;
        log::info!("*proactive scheduled* Next time-based message in {}h {}m", hours, minutes);
    }

    /// Check if mood has expired and reset to normal
    pub fn check_mood_expiry(&mut self) {
        if let Some(expires_at) = self.mood_expires_at {
            if Instant::now() >= expires_at {
                self.mood = MomoMood::Normal;
                self.mood_expires_at = None;
            }
        }
    }

    /// Set a new mood with duration
    pub fn set_mood(&mut self, mood: MomoMood) {
        self.mood = mood;
        if mood != MomoMood::Normal {
            self.mood_expires_at = Some(Instant::now() + mood.duration());
        } else {
            self.mood_expires_at = None;
        }
    }

    /// Check if bot should respond based on current mood
    /// Only affects proactive behaviors - explicit mentions always work
    pub fn should_respond(&self) -> bool {
        match self.mood {
            MomoMood::Normal | MomoMood::Playful | MomoMood::Annoyed => true,
            MomoMood::Busy => false,
        }
    }
}

/// Thread-safe state storage
pub type SharedChatStates = Arc<Mutex<HashMap<ChatId, ChatState>>>;

/// Get or create chat state
pub async fn get_chat_state(states: &SharedChatStates, chat_id: ChatId) -> ChatState {
    let mut states = states.lock().await;
    let state = states.entry(chat_id).or_insert_with(ChatState::new);
    state.check_mood_expiry();
    state.clone()
}

/// Update chat state
pub async fn update_chat_state(states: &SharedChatStates, chat_id: ChatId, state: ChatState) {
    let mut states = states.lock().await;
    states.insert(chat_id, state);
}

/// Set mood for a chat
pub async fn set_chat_mood(states: &SharedChatStates, chat_id: ChatId, mood: MomoMood) {
    let mut state = get_chat_state(states, chat_id).await;
    state.set_mood(mood);
    update_chat_state(states, chat_id, state).await;
}
