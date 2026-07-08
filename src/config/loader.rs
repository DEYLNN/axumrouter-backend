use crate::config::models::AppConfig;

pub fn load() -> anyhow::Result<AppConfig> {
    let settings = config::Config::builder()
        .add_source(config::File::with_name("config/config"))
        .add_source(config::Environment::with_prefix("AXUM"))
        .build()?;

    let cfg: AppConfig = settings.try_deserialize()?;
    Ok(cfg)
}
