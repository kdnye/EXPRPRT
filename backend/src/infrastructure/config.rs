use serde::Deserialize;
use std::env;
use std::time::Duration;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub app: AppConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub netsuite: NetSuiteConfig,
    #[serde(default)]
    pub receipts: ReceiptRules,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub cors_origins: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    #[serde(default = "default_pool_max")]
    pub max_connections: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            max_connections: default_pool_max(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthConfig {
    pub jwt_secret: String,
    #[serde(default = "default_jwt_ttl")]
    pub jwt_ttl_seconds: u64,
    #[serde(default)]
    pub developer_credential: String,
    #[serde(default)]
    pub bypass_auth: bool,
    #[serde(default)]
    pub bypass_hr_identifier: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StorageConfig {
    #[serde(default = "default_storage_provider")]
    pub provider: String,
    #[serde(default)]
    pub local_path: Option<String>,
    #[serde(default)]
    pub bucket: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct NetSuiteConfig {
    pub base_url: Option<String>,
    pub account: Option<String>,
    pub consumer_key: Option<String>,
    pub consumer_secret: Option<String>,
    pub token_id: Option<String>,
    pub token_secret: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ReceiptRules {
    #[serde(default = "default_max_receipt_size")]
    pub max_bytes: u64,
    #[serde(default = "default_max_receipt_count")]
    pub max_files_per_item: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            cors_origins: Vec::new(),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: String::new(),
            jwt_ttl_seconds: default_jwt_ttl(),
            developer_credential: String::new(),
            bypass_auth: false,
            bypass_hr_identifier: None,
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            provider: default_storage_provider(),
            local_path: None,
            bucket: None,
        }
    }
}

impl Default for ReceiptRules {
    fn default() -> Self {
        Self {
            max_bytes: default_max_receipt_size(),
            max_files_per_item: default_max_receipt_count(),
        }
    }
}

impl Config {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        let builder = config::Config::builder()
            .add_source(config::File::with_name("config").required(false))
            .add_source(config::Environment::with_prefix("EXPENSES").separator("__"));
        let cfg = builder.build()?;
        let mut config: Config = cfg.try_deserialize()?;

        if config.database.url.trim().is_empty() {
            let database_url = match env::var("EXPENSES__DATABASE__URL") {
                Ok(url) if !url.trim().is_empty() => url,
                _ => match env::var("DATABASE_URL") {
                    Ok(url) if !url.trim().is_empty() => url,
                    _ => {
                        return Err(config::ConfigError::Message(
                            "Missing database URL. Set EXPENSES__DATABASE__URL or DATABASE_URL."
                                .into(),
                        ));
                    }
                },
            };

            config.database.url = database_url;
        }

        Ok(config)
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.app.host, self.app.port)
    }

    pub fn jwt_ttl(&self) -> Duration {
        Duration::from_secs(self.auth.jwt_ttl_seconds)
    }
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_pool_max() -> u32 {
    10
}

fn default_jwt_ttl() -> u64 {
    60 * 60 * 8
}

fn default_storage_provider() -> String {
    "local".to_string()
}

fn default_max_receipt_size() -> u64 {
    5 * 1024 * 1024
}

fn default_max_receipt_count() -> u32 {
    10
}

#[cfg(test)]
mod tests {
    use super::Config;
    use config::ConfigError;
    use serial_test::serial;
    use std::env;

    fn clear_env_vars() {
        env::remove_var("EXPENSES__DATABASE__URL");
        env::remove_var("DATABASE_URL");
    }

    #[test]
    #[serial]
    fn uses_expenses_database_url_when_config_missing() {
        clear_env_vars();
        env::set_var(
            "EXPENSES__DATABASE__URL",
            "postgres://expenses:expenses@localhost:5432/expenses",
        );

        let config = Config::from_env().expect("expected configuration to load");

        assert_eq!(
            config.database.url,
            "postgres://expenses:expenses@localhost:5432/expenses"
        );
        assert_eq!(config.database.max_connections, 10);

        clear_env_vars();
    }

    #[test]
    #[serial]
    fn falls_back_to_database_url_when_prefixed_missing() {
        clear_env_vars();
        env::set_var(
            "DATABASE_URL",
            "postgres://fallback:fallback@localhost:5432/fallback",
        );

        let config = Config::from_env().expect("expected configuration to load");

        assert_eq!(
            config.database.url,
            "postgres://fallback:fallback@localhost:5432/fallback"
        );

        clear_env_vars();
    }

    #[test]
    #[serial]
    fn errors_when_no_database_url_available() {
        clear_env_vars();

        let error = Config::from_env().expect_err("expected configuration to fail");

        match error {
            ConfigError::Message(message) => assert_eq!(
                message,
                "Missing database URL. Set EXPENSES__DATABASE__URL or DATABASE_URL.".to_string()
            ),
            other => panic!("unexpected error: {:?}", other),
        }
    }
}
