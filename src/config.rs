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

use std::path::Path;

fn config_builder(
) -> Result<config::ConfigBuilder<config::builder::DefaultState>, config::ConfigError> {
    config::Config::builder()
        .set_default("server.host", "0.0.0.0")?
        .set_default("server.port", 8000)?
        .set_default("upstream.base_url", "https://api.openai.com/v1")?
        .set_default("upstream.api_key", "")?
        .set_default("sqlite.path", "modelgate.db")
}

pub fn load_config_from_dir<P: AsRef<Path>>(dir: P) -> Result<AppConfig, config::ConfigError> {
    let builder = config_builder()?;
    let config_path = dir.as_ref().join("config.toml");

    let mut config = builder
        .add_source(config::File::from(config_path).required(false))
        .build()?;

    if let Ok(key) = std::env::var("UPSTREAM_API_KEY") {
        let mut builder = config::Config::builder();
        builder = builder.add_source(config);
        builder = builder.set_override("upstream.api_key", key)?;
        config = builder.build()?;
    }

    let cfg: AppConfig = config.try_deserialize()?;
    if cfg.upstream.api_key.trim().is_empty() {
        return Err(config::ConfigError::Message(
            "Missing upstream.api_key (set UPSTREAM_API_KEY or config.toml)".to_string(),
        ));
    }
    Ok(cfg)
}

pub fn load_config() -> Result<AppConfig, config::ConfigError> {
    load_config_from_dir(std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::with_env_lock;
    use std::env;
    use std::fs::{create_dir_all, File};
    use std::io::Write;

    fn clear_env_vars() {
        env::remove_var("UPSTREAM_API_KEY");
        env::remove_var("UPSTREAM__API_KEY");
    }

    fn store_env_var(key: &str, value: Option<String>) {
        match value {
            Some(value) => env::set_var(key, value),
            None => env::remove_var(key),
        }
    }

    #[test]
    fn load_config_from_env() {
        with_env_lock(|| {
            let original_value = env::var("UPSTREAM_API_KEY").ok();
            clear_env_vars();

            let dir = env::temp_dir().join("modelgate_config_env_test");
            let _ = std::fs::remove_dir_all(&dir);
            create_dir_all(&dir).expect("create config dir");

            env::set_var("UPSTREAM_API_KEY", "env-key");

            let cfg = load_config_from_dir(&dir).expect("load config from env");
            assert_eq!(cfg.upstream.api_key, "env-key");
            assert_eq!(cfg.server.host, "0.0.0.0");
            assert_eq!(cfg.server.port, 8000);

            store_env_var("UPSTREAM_API_KEY", original_value);
        });
    }

    #[test]
    fn load_config_from_file() {
        with_env_lock(|| {
            let original_value = env::var("UPSTREAM_API_KEY").ok();
            clear_env_vars();

            let dir = env::temp_dir().join("modelgate_config_file_test");
            let _ = std::fs::remove_dir_all(&dir);
            create_dir_all(&dir).expect("create config dir");

            let mut file = File::create(dir.join("config.toml")).expect("create config file");
            writeln!(
                file,
                "[upstream]\napi_key = \"file-key\"\n[server]\nhost = \"127.0.0.1\"\nport = 9000\n"
            )
            .expect("write config file");

            let cfg = load_config_from_dir(&dir).expect("load config from file");
            assert_eq!(cfg.upstream.api_key, "file-key");
            assert_eq!(cfg.server.host, "127.0.0.1");
            assert_eq!(cfg.server.port, 9000);

            store_env_var("UPSTREAM_API_KEY", original_value);
        });
    }

    #[test]
    fn missing_upstream_api_key_returns_error() {
        with_env_lock(|| {
            let original_value = env::var("UPSTREAM_API_KEY").ok();
            clear_env_vars();

            let dir = env::temp_dir().join("modelgate_config_missing_test");
            let _ = std::fs::remove_dir_all(&dir);
            create_dir_all(&dir).expect("create config dir");

            let err = load_config_from_dir(&dir).unwrap_err();
            let msg = format!("{err}");
            assert!(msg.contains("Missing upstream.api_key"));

            store_env_var("UPSTREAM_API_KEY", original_value);
        });
    }
}
