pub mod constants;

use crate::db::models::ApiKey;
use crate::providers::openai_compat::provider::OpenAICompatibleProvider;

pub fn new_with_keys(keys: Vec<ApiKey>) -> OpenAICompatibleProvider {
    OpenAICompatibleProvider::new(constants::config(), keys)
}
