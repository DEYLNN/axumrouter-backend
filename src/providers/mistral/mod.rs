pub mod constants;

use crate::db::models::ApiKey;
use crate::engine::openai_compat::provider::OpenAICompatibleProvider;

/// Thin constructor wrapping generic OpenAI-compatible provider.
/// All config at src/providers/mistral/constants.rs.
pub fn new_with_keys(keys: Vec<ApiKey>) -> OpenAICompatibleProvider {
    OpenAICompatibleProvider::new(constants::config(), keys)
}
