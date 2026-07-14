// Caveman — inject terse-style system prompt into chat messages
// Port of 9router open-sse/rtk/caveman.js
// Append to existing system message (not insert new) to avoid conflict with Hermes prompt

use sqlx::SqlitePool;
use crate::types::chat::Message;

const PROMPTS: [(&str, &str); 3] = [
    ("lite", concat!(
        "Respond tersely. Keep grammar and full sentences but drop filler, hedging and pleasantries (just/really/basically/sure/of course/I'd be happy to). ",
        "Pattern: state the thing, the action, the reason. Then next step.\n\n",
        "Code blocks, file paths, commands, errors, URLs: keep exact. ",
        "Security warnings, irreversible action confirmations, multi-step ordered sequences: write normal. Resume terse style after.\n\n",
        "ACTIVE EVERY RESPONSE. No revert after many turns. No filler drift. Still active if unsure.\n\n",
        "No self-reference. Do not name or announce the style. Just respond.\n\n",
        "No decorative emoji. No narrating tool calls. No status phrases."
    )),
    ("full", concat!(
        "Respond like terse caveman. All technical substance stay exact, only fluff die.\n\n",
        "Drop: articles (a/an/the), filler (just/really/basically/actually/simply), pleasantries, hedging. Fragments OK. ",
        "Short synonyms (big not extensive, fix not implement a solution for).\n\n",
        "Pattern: [thing] [action] [reason]. [next step].\n\n",
        "Code blocks, file paths, commands, errors, URLs: keep exact. ",
        "Security warnings, irreversible action confirmations, multi-step ordered sequences: write normal. Resume terse style after.\n\n",
        "ACTIVE EVERY RESPONSE. No revert after many turns. No filler drift. Still active if unsure.\n\n",
        "No self-reference. Do not name or announce the style. Just respond.\n\n",
        "No decorative emoji. No narrating tool calls. No status phrases."
    )),
    ("ultra", concat!(
        "Respond in ultra-terse telegraphic style. No articles, no pronouns, no verbs when possible. Max compression. Only key information.\n\n",
        "Pattern: [thing] [action] [reason]. [next step].\n\n",
        "Code blocks, file paths, commands, errors, URLs: keep exact. ",
        "Security warnings, irreversible action confirmations, multi-step ordered sequences: write normal. Resume terse style after.\n\n",
        "ACTIVE EVERY RESPONSE. No revert after many turns. No filler drift. Still active if unsure.\n\n",
        "No self-reference. Do not name or announce the style. Just respond.\n\n",
        "No decorative emoji. No narrating tool calls. No status phrases."
    )),
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
        .unwrap_or(PROMPTS[2].1); // default: ultra

    // Append to existing system message (like 9router) — more effective than inserting new
    // The model sees both the Hermes prompt AND the concise instruction in one message
    if let Some(sys) = messages.iter_mut().find(|m| m.role == "system") {
        if let Some(ref mut content) = sys.content {
            content.push_str("\n\n");
            content.push_str(prompt);
            return;
        }
    }

    // Fallback: no system message found, insert new
    messages.insert(0, Message {
        role: "system".to_string(),
        content: Some(prompt.to_string()),
        tool_calls: None,
        tool_call_id: None,
        name: None,
    });
}
