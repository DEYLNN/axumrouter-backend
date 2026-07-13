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
        let public_ip = detect_public_ip().await;
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

/// Try multiple endpoints to detect the public IP of this VPS.
/// Returns "unknown" if all fail.
async fn detect_public_ip() -> String {
    let endpoints = [
        "https://ifconfig.me",
        "https://api.ipify.org",
        "https://ipinfo.io/ip",
    ];
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .no_proxy()
        .build()
    {
        Ok(c) => c,
        Err(_) => return "unknown".to_string(),
    };
    for url in &endpoints {
        if let Ok(resp) = client.get(*url).send().await {
            if let Ok(text) = resp.text().await {
                let ip = text.trim().to_string();
                // basic IPv4 validation: x.x.x.x where x is 1-3 digits
                let parts: Vec<&str> = ip.split('.').collect();
                if parts.len() == 4 && parts.iter().all(|p| p.parse::<u8>().is_ok()) {
                    return ip;
                }
            }
        }
    }
    "unknown".to_string()
}
