pub mod constants;

use crate::db::models::ApiKey;
use crate::engine::openai_compat::provider::OpenAICompatibleProvider;

/// Thin constructor wrapping generic OpenAI-compatible provider.
pub fn new_with_keys(keys: Vec<ApiKey>) -> OpenAICompatibleProvider {
    OpenAICompatibleProvider::new(constants::config(), keys)
}
