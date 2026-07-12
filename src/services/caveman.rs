// Caveman — inject terse-style system prompt into chat messages
// Port of 9router open-sse/rtk/caveman.js

use sqlx::SqlitePool;
use crate::types::chat::Message;

const PROMPTS: [(&str, &str); 3] = [
    ("lite", "Be concise. Remove filler words but keep proper grammar. Answer directly."),
    ("ultra", "Respond in ultra-terse telegraphic style. No articles, no pronouns, no verbs when possible. Max compression. Only key information."),
    ("full", "Respond concisely and directly. No pleasantries, no explanations, no fluff. Drop articles, use fragments. Get straight to the point with minimal words."),
];

pub async fn inject(db: &SqlitePool, messages: &mut Vec<Message>) {
    let level: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'caveman_enabled'")
        .fetch_one(db)
        .await
        .unwrap_or_else(|_| "off".to_string());

    if level == "off" { return; }

    let prompt = PROMPTS.iter()
        .find(|(k, _)| *k == level.as_str())
        .map(|(_, p)| *p)
        .unwrap_or(PROMPTS[2].1); // default: full

    messages.insert(0, Message {
        role: "system".to_string(),
        content: Some(prompt.to_string()),
        tool_calls: None,
        tool_call_id: None,
        name: None,
    });
}
