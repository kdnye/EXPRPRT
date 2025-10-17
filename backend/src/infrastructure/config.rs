use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub app: AppConfig,
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

#[derive(Debug, Deserialize, Clone)]
pub struct AuthConfig {
    pub jwt_secret: String,
    #[serde(default = "default_jwt_ttl")]
    pub jwt_ttl_seconds: u64,
    #[serde(default)]
    pub developer_credential: String,
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
        cfg.try_deserialize()
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
