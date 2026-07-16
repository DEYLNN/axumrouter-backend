use sqlx::sqlite::SqlitePool;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::models::AppConfig;
use crate::providers::manager::ProviderManager;

pub struct AppState {
    pub config: AppConfig,
    pub db: SqlitePool,
    pub provider_manager: Arc<RwLock<ProviderManager>>,
    pub public_ip: String,
    pub public_url: String,
}

impl AppState {
    pub async fn new(config: AppConfig, db: SqlitePool) -> anyhow::Result<Self> {
        let provider_manager = Arc::new(RwLock::new(ProviderManager::new(&config, &db).await?));
        let public_ip = crate::utils::detect_public_ip().await;
        tracing::info!("Detected public IP: {}", public_ip);
        let public_url = config.server.public_url.clone()
            .unwrap_or_else(|| format!("http://{}:{}", public_ip, config.server.port));
        tracing::info!("Public URL: {}", public_url);
        Ok(Self {
            config,
            db,
            provider_manager,
            public_ip,
            public_url,
        })
    }
}