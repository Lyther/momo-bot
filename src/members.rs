/// Active member tracking module
use std::collections::VecDeque;
use teloxide::prelude::*;
use teloxide::types::User;

use crate::config::MAX_ACTIVE_MEMBERS_TRACKED;
use crate::types::SharedActiveMembers;

/// Track an active member in a chat
///
/// Adds the user to the front of the active members queue,
/// removing any previous instance and maintaining max size.
pub async fn track_active_member(
    active_members: SharedActiveMembers,
    chat_id: ChatId,
    user: &User,
) {
    let mut members = active_members.lock().await;
    let chat_members = members.entry(chat_id).or_insert_with(VecDeque::new);

    let user_id = user.id;
    let username = user
        .username
        .clone()
        .or_else(|| Some(user.first_name.clone()))
        .unwrap_or_else(|| format!("user{}", user_id));

    // Remove if already exists (to update position)
    chat_members.retain(|(id, _)| *id != user_id);

    // Add to front
    chat_members.push_front((user_id, username));

    // Keep only last N members
    if chat_members.len() > MAX_ACTIVE_MEMBERS_TRACKED {
        chat_members.pop_back();
    }
}

/// Get a random recent active member from a chat
///
/// Returns None if there aren't enough active members.
pub async fn get_random_active_member(
    active_members: &SharedActiveMembers,
    chat_id: ChatId,
    min_members: usize,
    pool_size: usize,
) -> Option<String> {
    let members = active_members.lock().await;
    let chat_members = members.get(&chat_id)?;

    if chat_members.len() < min_members {
        return None;
    }

    // Pick a random active member from recent pool
    let index = rand::random::<usize>() % chat_members.len().min(pool_size);
    let (_, username) = &chat_members[index];

    Some(username.clone())
}
