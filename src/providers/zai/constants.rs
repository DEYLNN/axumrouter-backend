use crate::engine::anthropic_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "zai";
pub const PROVIDER_NAME: &str = "Z.AI";
pub const MODEL_PREFIX: &str = "zai";
pub const BASE_URL: &str = "https://api.z.ai/api/anthropic";
pub const VALIDATE_URL: &str = "https://api.z.ai/api/anthropic/v1/messages";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#2563EB";
pub const ICON_NAME: &str = "glm.png";
pub const DOCS_URL: &str = "https://open.bigmodel.cn";
pub const API_KEY_URL: &str = "https://open.bigmodel.cn/usercenter/apikeys";
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

pub fn config() -> crate::engine::anthropic_compat::config::AnthropicConfig {
    crate::engine::anthropic_compat::config::AnthropicConfig {
        provider_id: PROVIDER_ID,
        provider_name: PROVIDER_NAME,
        model_prefix: MODEL_PREFIX,
        base_url: BASE_URL,
        validate_url: VALIDATE_URL,
        docs_url: DOCS_URL,
        api_key_url: API_KEY_URL,
        category: CATEGORY,
        color: COLOR,
        icon_name: ICON_NAME,
        default_timeout_secs: DEFAULT_TIMEOUT_SECS,
        stream_first_chunk_timeout_secs: 120,
        stream_stall_timeout_secs: 120,
        models: MODELS,
        quirks: ProviderQuirks::default(),
    }
}

pub const MODELS: &[ModelDef] = &[
    ModelDef { id: "zai-org/glm-5.1", name: "GLM 5.1", max_tokens: 202752, supports_vision: false, supports_tools: true },
    ModelDef { id: "zai-org/glm-5", name: "GLM 5", max_tokens: 202752, supports_vision: false, supports_tools: true },
    ModelDef { id: "zai-org/glm-4.7", name: "GLM 4.7", max_tokens: 202752, supports_vision: false, supports_tools: true },
    ModelDef { id: "zai-org/glm-4.6", name: "GLM 4.6", max_tokens: 202752, supports_vision: false, supports_tools: true },
];
