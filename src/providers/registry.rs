use std::collections::HashMap;

use crate::db::models::ApiKey;
use sqlx::SqlitePool;
use std::sync::Arc;
use crate::providers::traits::Provider;

/// Macro to register a provider with a `new_with_keys(keys)` constructor.
macro_rules! register_provider {
    ($reg:expr_2021, $id:expr_2021, $path:path) => {
        $reg.register($id, |keys: Vec<ApiKey>, _db: Arc<SqlitePool>| {
            Ok(Box::new($path(keys)))
        });
    };
    // Variant that also receives the DB pool (e.g. np auto-refresh)
    ($reg:expr_2021, $id:expr_2021, $path:path, db) => {
        $reg.register($id, |keys: Vec<ApiKey>, db: Arc<SqlitePool>| {
            Ok(Box::new($path(keys, db)))
        });
    };
}

/// Maps provider name → constructor function.
pub struct ProviderRegistry {
    builders: HashMap<String, Box<dyn Fn(Vec<ApiKey>, Arc<sqlx::SqlitePool>) -> anyhow::Result<Box<dyn Provider>> + Send + Sync>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            builders: HashMap::new(),
        };

        // OpenAI-compatible (API key):
        register_provider!(registry, "mst", crate::providers::mistral::new_with_keys);
        register_provider!(registry, "ocg", crate::providers::opencode_go::new_with_keys);
        register_provider!(registry, "ocf", crate::providers::opencode_free::new_with_keys);
        register_provider!(registry, "tbay", crate::providers::tokenbay::new_with_keys);
        register_provider!(registry, "nak", crate::providers::nous_api_key::new_with_keys);
        register_provider!(registry, "cl", crate::providers::cline::new_with_keys);

        // Custom providers:
        register_provider!(registry, "cf", crate::providers::cloudflare::provider::CfProvider::new_with_keys);
        register_provider!(registry, "fb", crate::providers::freebuff::provider::FbProvider::new_with_keys);
        register_provider!(registry, "mcf", crate::providers::mimo_code_free::provider::McfProvider::new_with_keys);
        register_provider!(registry, "np", crate::providers::nous_portal::provider::NpProvider::new_with_keys, db);
        register_provider!(registry, "cx", crate::providers::openai_codex::provider::CxProvider::new_with_keys);
        register_provider!(registry, "xai", crate::providers::xai::provider::XaiProvider::new_with_keys);

        // API-key (openai_compat):
        register_provider!(registry, "xak", crate::providers::xai_api_key::new_with_keys);
        register_provider!(registry, "ags", crate::providers::agnesai::new_with_keys);
        register_provider!(registry, "amb", crate::providers::ambient::new_with_keys);
        register_provider!(registry, "lc", crate::providers::longcat::new_with_keys);
        register_provider!(registry, "zyl", crate::providers::zyloo::new_with_keys);
        register_provider!(registry, "tr", crate::providers::tokenrouter::new_with_keys);
        register_provider!(registry, "0g", crate::providers::oglabs::new_with_keys);
        register_provider!(registry, "mrph", crate::providers::morph::new_with_keys);
        register_provider!(registry, "hus", crate::providers::husada::new_with_keys);
        register_provider!(registry, "sr", crate::providers::swiftrouter::new_with_keys);
        register_provider!(registry, "gmi", crate::providers::gmi_cloud::new_with_keys);
        register_provider!(registry, "poll", crate::providers::pollinations::new_with_keys);
        register_provider!(registry, "zmx", crate::providers::zenmux::new_with_keys);
        register_provider!(registry, "btl", crate::providers::badtheory_labs::new_with_keys);
        register_provider!(registry, "kim", crate::providers::kimchi::new_with_keys);
        register_provider!(registry, "kimi", crate::providers::kimi::new_with_keys);
        register_provider!(registry, "co", crate::providers::conduit_ozdoev::new_with_keys);
        register_provider!(registry, "om", crate::providers::openmodel::new_with_keys);
        register_provider!(registry, "cer", crate::providers::cerebras::new_with_keys);
        register_provider!(registry, "or", crate::providers::openrouter::new_with_keys);
        register_provider!(registry, "rwy", crate::providers::routeway::new_with_keys);
        register_provider!(registry, "evo", crate::providers::evomap::new_with_keys);
        register_provider!(registry, "gq", crate::providers::groq::new_with_keys);
        register_provider!(registry, "nry", crate::providers::naraya_ai::new_with_keys);
        register_provider!(registry, "bai", crate::providers::bai::new_with_keys);
        register_provider!(registry, "cwv", crate::providers::canopywave::new_with_keys);
        register_provider!(registry, "mmx", crate::providers::minimax::new_with_keys);
        register_provider!(registry, "mmxcn", crate::providers::minimax_cn::new_with_keys);
        register_provider!(registry, "glb", crate::providers::gitlawb::new_with_keys);
        register_provider!(registry, "ocz", crate::providers::ocenza::new_with_keys);
        register_provider!(registry, "qc", crate::providers::qwencloud::new_with_keys);
        register_provider!(registry, "vk", crate::providers::vikey::new_with_keys);
        register_provider!(registry, "volc", crate::providers::volcengine_ark::new_with_keys);
        register_provider!(registry, "um", crate::providers::unimodel::new_with_keys);
        register_provider!(registry, "nzc", crate::providers::nabz_clan::new_with_keys);
        register_provider!(registry, "alc", crate::providers::alibaba_cloud::new_with_keys);
        register_provider!(registry, "al", crate::providers::alibaba::new_with_keys);
        register_provider!(registry, "alin", crate::providers::alibaba_intl::new_with_keys);
        register_provider!(registry, "cv", crate::providers::cavoti::new_with_keys);
        register_provider!(registry, "mimosg", crate::providers::xiaomi_mimo_sg::new_with_keys);
        register_provider!(registry, "mimo", crate::providers::xiaomi_mimo::new_with_keys);
        register_provider!(registry, "sn", crate::providers::sambanova::new_with_keys);
        register_provider!(registry, "am", crate::providers::aimux::new_with_keys);
        register_provider!(registry, "llm7", crate::providers::llm7::new_with_keys);
        register_provider!(registry, "qin", crate::providers::qiniu::new_with_keys);
        register_provider!(registry, "zai", crate::providers::zai::new_with_keys);
        register_provider!(registry, "bmc", crate::providers::bigmodel_china::new_with_keys);
        register_provider!(registry, "yun", crate::providers::yunwu::new_with_keys);
        // Kilo Code (OAuth)
        register_provider!(registry, "kc", crate::providers::kilocode::provider::KlProvider::new_with_keys);

        registry
    }

    pub fn register<F>(&mut self, id: &str, builder: F)
    where
        F: Fn(Vec<ApiKey>, Arc<SqlitePool>) -> anyhow::Result<Box<dyn Provider>> + Send + Sync + 'static,
    {
        self.builders.insert(id.to_string(), Box::new(builder));
    }

    pub fn build(
        &self,
        id: &str,
        keys: Vec<ApiKey>,
        db: Arc<SqlitePool>,
    ) -> Option<Box<dyn Provider>> {
        self.builders.get(id).and_then(|b| b(keys, db).ok())
    }

    pub fn provider_ids(&self) -> Vec<&str> {
        self.builders.keys().map(|s| s.as_str()).collect()
    }
}