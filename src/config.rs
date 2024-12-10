use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::path::{Path, PathBuf};

const DEFAULT_CONFIG_FILE_PREFIX: &str = "config";
const DEFAULT_CONFIG_FILE_NAME: &str = "default.toml";

#[derive(Deserialize, Clone)]
pub struct Settings {
    pub database: Database,
    pub liquidity_pool: LiquidityPool,
    pub rpc: Rpc,
}

#[derive(Deserialize, Clone)]
pub struct LiquidityPool {
    pub account_addresses_base54: Vec<String>,
}

#[derive(Deserialize, Clone)]
pub struct Database {
    pub connection_url: String,
    pub min_connections: u32,
    pub max_connections: u32,
    pub log_level: String,
    pub clear_old_records: bool,
}

#[derive(Deserialize, Clone)]
pub struct Rpc {
    pub yellowstone_grpc_endpoint: String,
    pub json_rpc_endpoint: String,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub enum EnvProfile {
    Prod,
    Local,
    Dev,
}

impl Settings {
    pub fn for_env(env_name: &str) -> Result<Self, ConfigError> {
        Settings::load(Some(env_name), None)
    }

    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Result<Self, ConfigError> {
        Settings::load(None, None)
    }

    pub(crate) fn load(env_name: Option<&str>, config_path: Option<&str>) -> Result<Self, ConfigError> {
        let configs_path = config_path
            .map(|s| s.to_string())
            .unwrap_or(std::env::var("RUN_CONFIG_DIR").unwrap_or_else(|_| DEFAULT_CONFIG_FILE_PREFIX.to_string()));

        let env = env_name
            .map(|s| s.to_string())
            .unwrap_or(std::env::var("RUN_ENV").unwrap_or_else(|_| "local".into()));
        println!("Using profile: {}", &env);

        let raw_config = Config::builder()
            .add_source(File::from(default_config_file_path(&configs_path).as_path()))
            .add_source(File::from(find_config_file(&configs_path, &env).as_path()).required(false))
            .add_source(Environment::with_prefix("app").separator("__"))
            .set_override("env", env)?
            .build()?;

        raw_config.try_deserialize()
    }
}

fn default_config_file_path(base_path: &str) -> PathBuf {
    find_config_file(base_path, DEFAULT_CONFIG_FILE_NAME)
}

fn find_config_file(base_path: &str, name: &str) -> PathBuf {
    let full_path = Path::new(base_path);

    if full_path.exists() && full_path.is_absolute() {
        return full_path.to_owned();
    }

    let current_dir = std::env::current_dir().unwrap();

    let mut config_dir = current_dir.join(base_path);
    if !config_dir.exists() {
        config_dir = current_dir.parent().unwrap().join(base_path);
    }

    config_dir.join(name)
}
