use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ApiKey {
    pub id: String,
    pub provider_id: String,
    pub key_value: String,
    pub label: Option<String>,
    pub is_active: i64,  // SQLite stores as integer, convert at usage
    pub rate_limit: Option<i64>,
    pub last_used_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl ApiKey {
    pub fn is_active(&self) -> bool {
        self.is_active != 0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RequestLog {
    pub id: i64,
    pub provider_id: String,
    pub model: String,
    pub status_code: Option<i64>,
    pub latency_ms: Option<i64>,
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub error_message: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Usage {
    pub id: String,
    pub provider_id: String,
    pub api_key_id: String,
    pub key_label: Option<String>,
    pub model_id: String,
    pub status: String,
    pub status_code: Option<i64>,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub latency_ms: Option<i64>,
    pub error_message: Option<String>,
    pub request_body: Option<String>,
    pub response_body: Option<String>,
    pub created_at: String,
}
