use crate::error::GatewayError;
use futures::stream::BoxStream;

/// Result of a provider chat completion call — includes the response
/// plus any failed key attempts (for usage logging).
pub struct ChatResult {
    pub response: crate::types::chat::ChatCompletionResponse,
    pub used_key_id: Option<String>,
    /// Key IDs that failed auth, in order of attempts
    pub failed_keys: Vec<FailedKeyAttempt>,
}

/// Result of a streaming provider chat completion call.
pub struct ChatStreamResult {
    pub stream: BoxStream<'static, Result<crate::types::chat::ChatCompletionChunk, GatewayError>>,
    pub used_key_id: Option<String>,
    pub failed_keys: Vec<FailedKeyAttempt>,
    /// Last key the engine actually attempted (auth-build or upstream call).
    /// `None` only when no key was ever tried (e.g. zero configured keys).
    /// On total failure this lets the usage log show WHICH key was the final
    /// attempt, not just `provider_api_key_id=None`.
    pub last_attempted_key_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FailedKeyAttempt {
    pub key_id: String,
    pub error: GatewayError,
}