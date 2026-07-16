use std::collections::HashMap;
use std::sync::Arc;
use sqlx::SqlitePool;

use crate::config::models::AppConfig;
use crate::providers::registry::ProviderRegistry;
use crate::providers::traits::Provider;
use crate::types::model::Model;
use crate::types::provider::ProviderMetadata;

/// Runtime manager — stores ALL registered providers from registry.
/// Keys are loaded from DB but providers are always available (even with 0 keys).
pub struct ProviderManager {
    active: HashMap<String, Box<dyn Provider>>,
    registry: ProviderRegistry,
    db: SqlitePool,
}

impl ProviderManager {
    pub async fn new(_config: &AppConfig, db: &SqlitePool) -> anyhow::Result<Self> {
        let registry = ProviderRegistry::new();
        let mut active = HashMap::new();

        let provider_ids: Vec<String> = registry.provider_ids().iter().map(|s| s.to_string()).collect();

        for provider_id in &provider_ids {
            let keys = crate::db::load_provider_keys(db, provider_id).await?;
            let key_count = keys.len();

            match registry.build(provider_id, keys, Arc::new(db.clone())) { Some(provider) => {
                tracing::info!(
                    "Provider '{}' loaded with {} key(s)",
                    provider_id,
                    key_count
                );
                active.insert(provider_id.to_string(), provider);
            } _ => {
                tracing::warn!("Provider '{}' failed to build", provider_id);
            }}
        }

        Ok(Self { active, registry, db: db.clone() })
    }

    /// Look up provider by name
    pub fn get(&self, name: &str) -> Option<&dyn Provider> {
        self.active.get(name).map(|p| p.as_ref())
    }

    /// List all registered provider names
    pub fn provider_names(&self) -> Vec<&str> {
        self.active.keys().map(|s| s.as_str()).collect()
    }

    /// Reload keys from DB for a provider and rebuild it
    pub async fn reload_provider(&mut self, provider_id: &str) -> anyhow::Result<()> {
        let keys = crate::db::load_provider_keys(&self.db, provider_id).await?;
        let key_count = keys.len();

        match self.registry.build(provider_id, keys, Arc::new(self.db.clone())) { Some(provider) => {
            tracing::info!(
                "Provider '{}' reloaded with {} key(s)",
                provider_id,
                key_count
            );
            self.active.insert(provider_id.to_string(), provider);
        } _ => {
            tracing::warn!("Provider '{}' failed to rebuild after reload", provider_id);
        }}

        Ok(())
    }

    /// Aggregate models from all providers
    pub async fn list_all_models(&self) -> Vec<Model> {
        let mut all = Vec::new();
        for (_name, provider) in &self.active {
            if let Ok(models) = provider.list_models().await {
                all.extend(models);
            }
        }
        all
    }

    /// List all provider metadata
    pub fn list_providers(&self) -> Vec<ProviderMetadata> {
        self.active
            .values()
            .map(|p| p.metadata())
            .collect()
    }

    /// Number of total keys for a specific provider
    pub fn total_keys_for(&self, name: &str) -> Option<usize> {
        self.active.get(name).map(|p| p.total_keys())
    }

    /// Number of active (non-locked) keys for a specific provider
    pub fn active_keys_for(&self, name: &str) -> Option<usize> {
        self.active.get(name).map(|p| p.active_keys())
    }
}
