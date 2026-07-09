pub const PROVIDER_ID: &str = "cf";
pub const PROVIDER_NAME: &str = "Cloudflare";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#F38020";
pub const ICON_URL: &str = "/public/providers/cf.png";
pub const BASE_URL: &str = "https://api.cloudflare.com/client/v4/accounts";
pub const DEFAULT_TIMEOUT_SECS: u64 = 90;
pub const STREAM_FIRST_CHUNK_TIMEOUT_SECS: u64 = 60;
pub const STREAM_STALL_TIMEOUT_SECS: u64 = 120;
pub const USER_AGENT: &str = "axumrouter/1.0";

pub fn provider_spec() -> crate::providers::spec::ProviderSpec {
    crate::providers::spec::ProviderSpec {
        id: PROVIDER_ID,
        name: PROVIDER_NAME,
        full_name: "cloudflare-ai",
        category: CATEGORY,
        base_url: BASE_URL,
        validate_url: "https://api.cloudflare.com/client/v4/user/tokens/verify",
        compatible_api: "openai-chat",
        supports_streaming: true,
        supports_tools: false,
        supports_vision: true,
        color: COLOR,
        icon_url: ICON_URL,
        usage_url: None,
        quirks: Default::default(),
    }
}

#[derive(Debug, Clone)]
pub struct ModelDef { pub id: &'static str, pub name: &'static str }

pub const MODELS: &[ModelDef] = &[
    // Terbaru (2026)
    ModelDef { id: "@cf/zai-org/glm-5.2", name: "GLM 5.2" },
    ModelDef { id: "@cf/moonshotai/kimi-k2.7-code", name: "Kimi K2.7 Code" },
    ModelDef { id: "@cf/moonshotai/kimi-k2.6", name: "Kimi K2.6" },
    ModelDef { id: "@cf/google/gemma-4-26b-a4b-it", name: "Gemma 4 26B" },
    // Populer & terkenal
    ModelDef { id: "@cf/meta/llama-4-scout-17b-16e-instruct", name: "Llama 4 Scout 17B" },
    ModelDef { id: "@cf/meta/llama-3.3-70b-instruct-fp8-fast", name: "Llama 3.3 70B" },
    ModelDef { id: "@cf/meta/llama-3.2-11b-vision-instruct", name: "Llama 3.2 11B Vision" },
    ModelDef { id: "@cf/meta/llama-3.2-3b-instruct", name: "Llama 3.2 3B" },
    ModelDef { id: "@cf/meta/llama-3.2-1b-instruct", name: "Llama 3.2 1B" },
    ModelDef { id: "@cf/meta/llama-3.1-8b-instruct-fp8", name: "Llama 3.1 8B" },
    ModelDef { id: "@cf/mistralai/mistral-small-3.1-24b-instruct", name: "Mistral Small 3.1 24B" },
    ModelDef { id: "@cf/deepseek-ai/deepseek-r1-distill-qwen-32b", name: "DeepSeek R1 Distill 32B" },
    ModelDef { id: "@cf/qwen/qwen2.5-coder-32b-instruct", name: "Qwen 2.5 Coder 32B" },
    ModelDef { id: "@cf/qwen/qwq-32b", name: "QwQ 32B" },
    ModelDef { id: "@cf/qwen/qwen3-30b-a3b-fp8", name: "Qwen3 30B" },
    ModelDef { id: "@cf/openai/gpt-oss-120b", name: "GPT-OSS 120B" },
    ModelDef { id: "@cf/openai/gpt-oss-20b", name: "GPT-OSS 20B" },
    ModelDef { id: "@cf/zai-org/glm-4.7-flash", name: "GLM 4.7 Flash" },
    ModelDef { id: "@cf/google/gemma-2b-it-lora", name: "Gemma 2B" },
    ModelDef { id: "@cf/google/gemma-7b-it-lora", name: "Gemma 7B" },
];
