use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub upstream: UpstreamConfig,
    pub sqlite: SqliteConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpstreamConfig {
    pub base_url: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SqliteConfig {
    pub path: String,
}

pub fn load_config() -> Result<AppConfig, config::ConfigError> {
    let mut builder = config::Config::builder()
        .set_default("server.host", "0.0.0.0")?
        .set_default("server.port", 8000)?
        .set_default("upstream.base_url", "https://api.openai.com/v1")?
        .set_default("upstream.api_key", "")?
        .set_default("sqlite.path", "modelgate.db")?
        .add_source(config::File::with_name("config").required(false))
        .add_source(config::Environment::default().separator("__"));

    // Allow a more ergonomic env var for API key.
    // If UPSTREAM_API_KEY is set, map it into upstream.api_key.
    if let Ok(key) = std::env::var("UPSTREAM_API_KEY") {
        builder = builder.set_override("upstream.api_key", key)?;
    }

    let cfg: AppConfig = builder.build()?.try_deserialize()?;
    if cfg.upstream.api_key.trim().is_empty() {
        return Err(config::ConfigError::Message(
            "Missing upstream.api_key (set UPSTREAM_API_KEY or config.toml)".to_string(),
        ));
    }
    Ok(cfg)
}
