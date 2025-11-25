/// Utility functions
use teloxide::types::{Chat, ChatKind};

/// Check if a chat is a group or channel (not private)
pub fn is_group_or_channel(chat: &Chat) -> bool {
    !matches!(chat.kind, ChatKind::Private(_))
}

/// Select a random item from a slice
pub fn random_choice<T>(items: &[T]) -> &T {
    &items[rand::random::<usize>() % items.len()]
}
