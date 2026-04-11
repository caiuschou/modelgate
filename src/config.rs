use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub upstream: UpstreamConfig,
    #[serde(default)]
    pub byok: ByokConfig,
    /// Prepaid USD balance and usage-based charges (see `docs/product/recharge-and-billing-solution.md`).
    #[serde(default)]
    pub billing: BillingConfig,
    pub sqlite: SqliteConfig,
    pub audit: crate::audit::AuditConfig,
    /// Where to write rolling `tracing` logs (empty = stderr only).
    #[serde(default)]
    pub logging: LoggingConfig,
    /// Console registration: invite code must match exactly (see `auth.invite_code`).
    pub auth: AuthConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    /// Directory for daily rolling files `modelgate.log.YYYY-MM-DD`. Empty = no log files.
    #[serde(default)]
    pub tracing_log_dir: String,
    /// `tracing_subscriber::EnvFilter` when the `RUST_LOG` environment variable is unset.
    /// Examples: `info`, `trace`, `modelgate=trace,actix_web=info`.
    #[serde(default = "default_logging_filter")]
    pub default_filter: String,
}

fn default_logging_filter() -> String {
    "info".to_string()
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            tracing_log_dir: String::new(),
            default_filter: default_logging_filter(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    /// Required for `POST /api/v1/auth/register`. Empty string disables self-service registration.
    pub invite_code: String,
    /// HS256 secret for console session JWTs. Empty = derive from upstream key at load time, or `JWT_SECRET`.
    #[serde(default)]
    pub jwt_secret: String,
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

/// Prepaid balance and charges from upstream-reported USD (stored as k=15 minor units; see `crate::money`).
#[derive(Debug, Clone, Deserialize)]
pub struct BillingConfig {
    /// When false, balance checks and ledger charges are skipped (e.g. E2E or self-hosted).
    #[serde(default = "default_billing_enabled")]
    pub enabled: bool,
    /// Enables `POST /api/v1/billing/admin-deposit` (Bearer token + target username). Disable in production unless needed.
    #[serde(default)]
    pub admin_deposit_enabled: bool,
    /// Shared secret for admin deposit; compared to `Authorization: Bearer <value>`. Empty = endpoint not available.
    #[serde(default)]
    pub admin_deposit_password: String,
    /// Minimum top-up amount in USD cents (e.g. 1000 = $10.00); converted to minor units internally.
    #[serde(default = "default_min_deposit_cents")]
    pub min_deposit_cents: i64,
}

fn default_billing_enabled() -> bool {
    true
}

fn default_min_deposit_cents() -> i64 {
    1000
}

impl Default for BillingConfig {
    fn default() -> Self {
        Self {
            enabled: default_billing_enabled(),
            admin_deposit_enabled: false,
            admin_deposit_password: String::new(),
            min_deposit_cents: default_min_deposit_cents(),
        }
    }
}

/// BYOK at-rest encryption. Empty `master_key_hex` disables BYOK management and `X-MG-Byok-Id`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ByokConfig {
    /// 64 hex characters (32 bytes). Override with env `BYOK_MASTER_KEY`.
    #[serde(default)]
    pub master_key_hex: String,
}

impl ByokConfig {
    pub fn master_key_32(&self) -> Result<[u8; 32], String> {
        let s = self.master_key_hex.trim();
        if s.is_empty() {
            return Err("BYOK master key is not configured".into());
        }
        let raw = hex::decode(s).map_err(|e| format!("invalid byok.master_key_hex: {e}"))?;
        if raw.len() != 32 {
            return Err(format!(
                "byok.master_key_hex must be 32 bytes hex-encoded (64 hex chars); got {} bytes",
                raw.len()
            ));
        }
        let mut out = [0u8; 32];
        out.copy_from_slice(&raw);
        Ok(out)
    }
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
        .set_default("byok.master_key_hex", "")?
        .set_default("sqlite.path", "modelgate.db")?
        .set_default("audit.log_dir", "./audit_logs")?
        .set_default("audit.retention_days", 90)?
        .set_default("audit.batch_size", 50)?
        .set_default("audit.flush_interval_seconds", 5)?
        .set_default("audit.export_dir", "./exports")?
        .set_default("logging.tracing_log_dir", "")?
        .set_default("logging.default_filter", "info")?
        .set_default("auth.invite_code", "ZW9Z")
}

pub fn load_config_from_dir<P: AsRef<Path>>(dir: P) -> Result<AppConfig, config::ConfigError> {
    let builder = config_builder()?;
    let config_path = dir.as_ref().join("config.toml");

    let mut config = builder
        .add_source(config::File::from(config_path).required(false))
        .build()?;

    if let Ok(base_url) = std::env::var("UPSTREAM_BASE_URL") {
        let mut builder = config::Config::builder();
        builder = builder.add_source(config);
        builder = builder.set_override("upstream.base_url", base_url)?;
        config = builder.build()?;
    }

    if let Ok(code) = std::env::var("AUTH_INVITE_CODE") {
        let mut builder = config::Config::builder();
        builder = builder.add_source(config);
        builder = builder.set_override("auth.invite_code", code)?;
        config = builder.build()?;
    }

    if let Ok(dir) = std::env::var("TRACING_LOG_DIR") {
        if !dir.trim().is_empty() {
            let mut builder = config::Config::builder();
            builder = builder.add_source(config);
            builder = builder.set_override("logging.tracing_log_dir", dir)?;
            config = builder.build()?;
        }
    }

    if let Ok(port_s) = std::env::var("MODELGATE_SERVER_PORT") {
        if let Ok(p) = port_s.parse::<u16>() {
            let mut builder = config::Config::builder();
            builder = builder.add_source(config);
            builder = builder.set_override("server.port", i64::from(p))?;
            config = builder.build()?;
        }
    }

    let mut cfg: AppConfig = config.try_deserialize()?;
    if let Ok(h) = std::env::var("BYOK_MASTER_KEY") {
        let t = h.trim();
        if !t.is_empty() {
            cfg.byok.master_key_hex = t.to_string();
        }
    }
    if cfg.auth.jwt_secret.trim().is_empty() {
        cfg.auth.jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
            let h = crate::secrets::secret_sha256_hex(&cfg.upstream.api_key);
            format!("mg-jwt-{}", &h[..32])
        });
    }
    if cfg.upstream.api_key.trim().is_empty() {
        return Err(config::ConfigError::Message(
            "Missing upstream.api_key in config.toml".to_string(),
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
        env::remove_var("UPSTREAM__API_KEY");
        env::remove_var("UPSTREAM_BASE_URL");
        env::remove_var("UPSTREAM__BASE_URL");
        env::remove_var("TRACING_LOG_DIR");
    }

    fn store_env_var(key: &str, value: Option<String>) {
        match value {
            Some(value) => env::set_var(key, value),
            None => env::remove_var(key),
        }
    }

    #[test]
    fn upstream_api_key_comes_from_file_not_env() {
        with_env_lock(|| {
            clear_env_vars();
            env::set_var("UPSTREAM_API_KEY", "env-key-should-be-ignored");

            let dir = env::temp_dir().join("modelgate_config_key_file_test");
            let _ = std::fs::remove_dir_all(&dir);
            create_dir_all(&dir).expect("create config dir");

            let mut file = File::create(dir.join("config.toml")).expect("create config file");
            writeln!(
                file,
                "[upstream]\napi_key = \"file-key\"\n[server]\nhost = \"127.0.0.1\"\nport = 9000\n"
            )
            .expect("write config file");

            let cfg = load_config_from_dir(&dir).expect("load config");
            assert_eq!(cfg.upstream.api_key, "file-key");

            env::remove_var("UPSTREAM_API_KEY");
        });
    }

    #[test]
    fn tracing_log_dir_env_override() {
        with_env_lock(|| {
            let original_tracing = env::var("TRACING_LOG_DIR").ok();
            clear_env_vars();

            let dir = env::temp_dir().join("modelgate_config_tracing_test");
            let _ = std::fs::remove_dir_all(&dir);
            create_dir_all(&dir).expect("create config dir");

            let mut file = File::create(dir.join("config.toml")).expect("create config file");
            writeln!(
                file,
                "[upstream]\napi_key = \"test-key-for-tracing-test\"\n"
            )
            .expect("write config file");

            env::set_var("TRACING_LOG_DIR", "/tmp/modelgate-tracing");

            let cfg = load_config_from_dir(&dir).expect("load config");
            assert_eq!(cfg.logging.tracing_log_dir, "/tmp/modelgate-tracing");

            store_env_var("TRACING_LOG_DIR", original_tracing);
        });
    }

    #[test]
    fn upstream_base_url_env_override() {
        with_env_lock(|| {
            let original_base = env::var("UPSTREAM_BASE_URL").ok();
            clear_env_vars();

            let dir = env::temp_dir().join("modelgate_config_base_url_test");
            let _ = std::fs::remove_dir_all(&dir);
            create_dir_all(&dir).expect("create config dir");

            let mut file = File::create(dir.join("config.toml")).expect("create config file");
            writeln!(
                file,
                "[upstream]\napi_key = \"file-key\"\nbase_url = \"https://api.openai.com/v1\"\n"
            )
            .expect("write config file");

            env::set_var("UPSTREAM_BASE_URL", "http://127.0.0.1:18080/v1");

            let cfg = load_config_from_dir(&dir).expect("load config");
            assert_eq!(cfg.upstream.api_key, "file-key");
            assert_eq!(cfg.upstream.base_url, "http://127.0.0.1:18080/v1");

            store_env_var("UPSTREAM_BASE_URL", original_base);
        });
    }

    #[test]
    fn load_config_from_file() {
        with_env_lock(|| {
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
        });
    }

    #[test]
    fn modelgate_server_port_env_overrides_file() {
        with_env_lock(|| {
            clear_env_vars();

            let dir = env::temp_dir().join("modelgate_port_env_test");
            let _ = std::fs::remove_dir_all(&dir);
            create_dir_all(&dir).expect("create config dir");

            let mut file = File::create(dir.join("config.toml")).expect("create config file");
            writeln!(
                file,
                "[upstream]\napi_key = \"file-key\"\n[server]\nhost = \"127.0.0.1\"\nport = 9000\n"
            )
            .expect("write config file");

            env::set_var("MODELGATE_SERVER_PORT", "14040");
            let cfg = load_config_from_dir(&dir).expect("load config");
            assert_eq!(cfg.server.port, 14040);
            env::remove_var("MODELGATE_SERVER_PORT");
        });
    }

    #[test]
    fn missing_upstream_api_key_returns_error() {
        with_env_lock(|| {
            clear_env_vars();

            let dir = env::temp_dir().join("modelgate_config_missing_test");
            let _ = std::fs::remove_dir_all(&dir);
            create_dir_all(&dir).expect("create config dir");

            let err = load_config_from_dir(&dir).unwrap_err();
            let msg = format!("{err}");
            assert!(msg.contains("Missing upstream.api_key"));
        });
    }
}
