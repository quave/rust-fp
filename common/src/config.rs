use serde::Deserialize;
use std::{error::Error, fs};

#[derive(Debug, Deserialize, Clone, Default)]
pub struct CommonConfig {
    pub project_name: String,
    pub database_url: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ImporterConfig {
    pub server_address: String,
    pub log_level: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct BackendConfig {
    pub server_address: String,
    pub log_level: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ProcessorConfig {
    pub threads: u32,
    pub sleep_ms: u64,
    pub log_level: String,
    #[serde(default)]
    pub matcher_configs: Option<std::collections::HashMap<String, (i32, i32)>>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    pub common: CommonConfig,
    pub importer: ImporterConfig,
    pub processor: ProcessorConfig,
    pub backend: BackendConfig,
}

impl Config {
    pub fn load(config_path: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let contents = fs::read_to_string(config_path)?;
        let config = serde_yml::from_str(&contents)?;

        Ok(config)
    }
}
