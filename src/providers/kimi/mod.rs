pub mod constants;

use crate::db::models::ApiKey;
use crate::engine::anthropic_compat::provider::AnthropicCompatibleProvider;

pub fn new_with_keys(keys: Vec<ApiKey>) -> AnthropicCompatibleProvider {
    AnthropicCompatibleProvider::new(constants::config(), keys)
}
