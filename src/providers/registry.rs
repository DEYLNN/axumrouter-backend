use std::collections::HashMap;

use crate::db::models::ApiKey;
use sqlx::SqlitePool;
use std::sync::Arc;
use crate::providers::traits::Provider;

/// Macro to register a provider with a `new_with_keys(keys)` constructor.
macro_rules! register_provider {
    ($reg:expr, $id:expr, $path:path) => {
        $reg.register($id, |keys: Vec<ApiKey>, _db: Arc<SqlitePool>| {
            Ok(Box::new($path(keys)))
        });
    };
    // Variant that also receives the DB pool (e.g. np auto-refresh)
    ($reg:expr, $id:expr, $path:path, db) => {
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
        register_provider!(registry, "nrak", crate::providers::nous_api_key::new_with_keys);
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
        // Kilo Code (OAuth)
        register_provider!(registry, "kl", crate::providers::kilocode::provider::KlProvider::new_with_keys);

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
