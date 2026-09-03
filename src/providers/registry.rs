use std::collections::HashMap;

use crate::db::models::ApiKey;
use sqlx::SqlitePool;
use std::sync::Arc;
use crate::providers::traits::Provider;
use crate::providers::toml_provider::{build_openai_config, build_anthropic_config, ProviderList};

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

        // --- Load simple API-key providers from TOML ---
        let toml_data = include_str!("../../providers.toml");
        if let Ok(list) = toml::from_str::<ProviderList>(toml_data) {
            for p in &list.providers {
                let id = p.id.clone();
                match p.api_type.as_str() {
                    "anthropic" => {
                        let config = build_anthropic_config(p);
                        registry.register(&id, move |keys: Vec<ApiKey>, db: Arc<SqlitePool>| {
                            let provider = crate::engine::anthropic_compat::provider::AnthropicCompatibleProvider::new(config.clone(), keys, Some((*db).clone()));
                            Ok(Box::new(provider))
                        });
                    }
                    _ => {
                        let config = build_openai_config(p);
                        registry.register(&id, move |keys: Vec<ApiKey>, db: Arc<SqlitePool>| {
                            let provider = crate::engine::openai_compat::provider::OpenAICompatibleProvider::new(config.clone(), keys, Some((*db).clone()));
                            Ok(Box::new(provider))
                        });
                    }
                }
            }
        }

        // --- Custom providers (manual) ---
        register_provider!(registry, "kc", crate::providers::oauth::kilocode::provider::KlProvider::new_with_keys, db);
        register_provider!(registry, "cbai", crate::providers::oauth::codebuddy_intl::provider::CbaiProvider::new_with_keys, db);
        register_provider!(registry, "gb", crate::providers::oauth::grok_build::provider::GbProvider::new_with_keys, db);
        register_provider!(registry, "cx", crate::providers::oauth::codex::provider::CxProvider::new_with_keys, db);
        register_provider!(registry, "cl", crate::providers::apikey::cline::provider::ClProvider::new_with_keys, db);
        register_provider!(registry, "cf", crate::providers::apikey::cloudflare::provider::CfProvider::new_with_keys, db);
        register_provider!(registry, "infx", crate::providers::apikey::inferx::provider::IxProvider::new_with_keys, db);
        register_provider!(registry, "fsn", crate::providers::apikey::fusioncode::provider::FsnProvider::new_with_keys, db);
        register_provider!(registry, "bai", crate::providers::apikey::bai::provider::BaiProvider::new_with_keys, db);
        register_provider!(registry, "kio", crate::providers::apikey::kio::provider::KioProvider::new_with_keys, db);

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