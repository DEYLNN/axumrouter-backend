#![allow(dead_code)]
use async_trait::async_trait;

use crate::error::GatewayError;
use crate::providers::result::{ChatResult, ChatStreamResult};
use crate::types::chat::ChatCompletionRequest;
use crate::types::model::Model;
use crate::types::provider::ProviderMetadata;

/// Core trait that every provider must implement.
/// The gateway only communicates through this trait — never through provider-specific code.
#[async_trait]
pub trait Provider: Send + Sync {
    /// Provider metadata (name, version, capabilities)
    fn metadata(&self) -> ProviderMetadata;

    /// Chat completion — returns ChatResult with response + failed attempts
    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatResult, GatewayError>;

    /// Chat completion with SSE streaming
    async fn chat_completion_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatStreamResult, GatewayError>;

    /// List available models for this provider
    async fn list_models(&self) -> Result<Vec<Model>, GatewayError>;

    /// Health check
    async fn health_check(&self) -> Result<bool, GatewayError>;

    /// Authenticate with this provider
    async fn authenticate(&self) -> Result<(), GatewayError>;

    /// Currently locked keys with (key_id, remaining_seconds, reason).
    /// Default: returns empty (providers without key management).
    fn locked_keys(&self) -> Vec<(String, u64, String)> {
        Vec::new()
    }

    /// Total keys configured. Default: 0.
    fn total_keys(&self) -> usize {
        0
    }

    /// Active keys (not locked). Default: 0.
    fn active_keys(&self) -> usize {
        0
    }
}